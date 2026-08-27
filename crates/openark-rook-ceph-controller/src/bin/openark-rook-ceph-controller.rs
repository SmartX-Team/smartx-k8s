use std::{
    collections::{BTreeMap, BTreeSet},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{Context, Result, bail, ensure};
use clap::Parser;
use k8s_openapi::api::core::v1::{ConfigMap, Node, Pod};
use kube::{
    Api, Client,
    api::{ListParams, ObjectMeta, Patch, PatchParams},
    core::{ApiResource, DynamicObject, GroupVersionKind},
};
use openark_rook_ceph_controller::{
    model::{
        DeviceInventory, NodeInventory, inventory_name, is_by_id_path, is_pci_bdf,
        pcie_hardware_class,
    },
    reconcile::{
        CandidateDevice, NodeCandidates, NodeIdentityClaims, PlannerConfig,
        existing_device_identities_by_node, normalize_device_identity, plan_storage_with_claims,
    },
};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::time::{MissedTickBehavior, interval};

const INVENTORY_KEY: &str = "inventory.json";
const INVENTORY_SELECTOR: &str = "org.ulagbulag.io/rook-ceph-device-inventory=true";
const ROOK_DISCOVERY_SELECTOR: &str = "app=rook-discover";
const FUTURE_SKEW_SECONDS: u64 = 30;

#[derive(Clone, Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(long, env = "CEPH_CLUSTER_NAMESPACE")]
    ceph_cluster_namespace: String,

    #[arg(long, env = "CEPH_CLUSTER_NAME")]
    ceph_cluster_name: String,

    #[arg(long, env = "ROOK_DISCOVERY_NAMESPACE")]
    rook_discovery_namespace: String,

    #[arg(long, env = "PROVISIONING_NAMESPACE")]
    provisioning_namespace: String,

    #[arg(long, env = "APP_INSTANCE")]
    app_instance: String,

    #[arg(
        long,
        env = "STORAGE_ROLE_LABEL",
        default_value = "node-role.kubernetes.io/storage"
    )]
    storage_role_label: String,

    #[arg(long, env = "STORAGE_ROLE_VALUE")]
    storage_role_value: Option<String>,

    #[arg(long, env = "MINIMUM_DEVICE_BYTES")]
    minimum_device_bytes: u64,

    #[arg(long, env = "OSDS_PER_DEVICE", default_value_t = 1)]
    osds_per_device: u32,

    #[arg(long, env = "DEVICE_CLASS_MAP")]
    device_class_map: String,

    #[arg(long, env = "INVENTORY_MAX_AGE_SECONDS", default_value_t = 900)]
    inventory_max_age_seconds: u64,

    #[arg(long, env = "POLL_INTERVAL_SECONDS", default_value_t = 60)]
    poll_interval_seconds: u64,

    #[arg(long, env = "DRY_RUN", default_value_t = true)]
    dry_run: bool,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocalDisk {
    name: String,
    #[serde(rename = "kernel-name")]
    kernel_name: Option<String>,
    dev_links: String,
    size: u64,
    #[serde(rename = "type")]
    device_type: String,
    rotational: bool,
    read_only: bool,
    empty: bool,
    ceph_volume_data: Option<String>,
    #[serde(skip)]
    ceph_volume_available: bool,
    #[serde(skip)]
    ceph_volume_removable: bool,
}

#[derive(Debug, Deserialize)]
struct CephVolumeData {
    path: String,
    available: bool,
    rejected_reasons: Vec<String>,
    sys_api: CephVolumeSysApi,
    lvs: Vec<BTreeMap<String, Value>>,
}

#[derive(Debug, Deserialize)]
struct CephVolumeSysApi {
    path: String,
    devname: String,
    #[serde(rename = "type")]
    device_type: String,
    size: f64,
    ro: String,
    removable: String,
    rotational: String,
    partitions: BTreeMap<String, Value>,
    id_bus: String,
}

#[derive(Clone)]
struct RuntimeConfig {
    args: Args,
    planner: PlannerConfig,
}

fn validate_args(args: Args) -> Result<RuntimeConfig> {
    ensure!(
        !args.ceph_cluster_name.is_empty(),
        "CephCluster name is mandatory"
    );
    ensure!(
        !args.ceph_cluster_namespace.is_empty(),
        "CephCluster namespace is mandatory"
    );
    ensure!(
        !args.rook_discovery_namespace.is_empty(),
        "Rook discovery namespace is mandatory"
    );
    ensure!(
        !args.provisioning_namespace.is_empty(),
        "provisioning namespace is mandatory"
    );
    ensure!(!args.app_instance.is_empty(), "app instance is mandatory");
    ensure!(
        !args.storage_role_label.is_empty(),
        "Storage role label must not be empty"
    );
    ensure!(
        args.osds_per_device > 0,
        "osds per device must be greater than zero"
    );
    ensure!(
        args.inventory_max_age_seconds > 0,
        "inventory maximum age must be greater than zero"
    );
    ensure!(
        args.poll_interval_seconds > 0,
        "poll interval must be greater than zero"
    );
    if args.storage_role_value.as_deref() == Some("") {
        bail!("Storage role value must be non-empty when configured");
    }
    let device_classes: BTreeMap<String, String> = serde_json::from_str(&args.device_class_map)
        .context("DEVICE_CLASS_MAP must be a JSON string map")?;
    ensure!(
        !device_classes.is_empty(),
        "device class map must not be empty"
    );
    ensure!(
        device_classes
            .iter()
            .all(|(key, value)| !key.is_empty() && !value.is_empty()),
        "device class map keys and values must not be empty"
    );
    let planner = PlannerConfig {
        osds_per_device: args.osds_per_device,
        device_classes,
    };
    Ok(RuntimeConfig { args, planner })
}

fn storage_role_value<'a>(node: &'a Node, args: &Args) -> Option<&'a str> {
    node.metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get(&args.storage_role_label))
        .filter(|value| {
            args.storage_role_value
                .as_ref()
                .is_none_or(|expected| expected == *value)
        })
        .map(String::as_str)
}

fn has_storage_role(node: &Node, args: &Args) -> bool {
    storage_role_value(node, args).is_some()
}

fn rook_node_name(node: &Node) -> Result<String> {
    node.metadata
        .labels
        .as_ref()
        .and_then(|labels| labels.get("kubernetes.io/hostname"))
        .cloned()
        .context("Kubernetes Node has no kubernetes.io/hostname label")
}

fn parse_ceph_volume_available(raw: &str, disk: &LocalDisk) -> Result<(bool, bool)> {
    let data =
        serde_json::from_str::<CephVolumeData>(raw).context("parsing ceph-volume inventory")?;
    let kernel_name = disk.kernel_name.as_deref().unwrap_or(&disk.name);
    let expected_path = format!("/dev/{kernel_name}");
    ensure!(
        data.path == expected_path,
        "ceph-volume path {:?} does not match Rook device {expected_path:?}",
        data.path
    );
    ensure!(
        data.rejected_reasons
            .iter()
            .all(|reason| !reason.trim().is_empty())
            && data.lvs.iter().all(|lv| {
                lv.keys().all(|key| !key.trim().is_empty())
                    && lv.values().any(json_value_is_meaningful)
            }),
        "ceph-volume inventory contains empty rejection, sys_api, or LV entries"
    );
    let sys_size = data.sys_api.size;
    ensure!(
        data.sys_api.path == expected_path
            && data.sys_api.devname == kernel_name
            && data.sys_api.device_type == disk.device_type
            && sys_size.is_finite()
            && sys_size >= 0.0
            && sys_size.fract() == 0.0
            && sys_size as u64 == disk.size
            && parse_ceph_sys_bool(&data.sys_api.ro)? == disk.read_only
            && parse_ceph_sys_bool(&data.sys_api.rotational)? == disk.rotational
            && (data.sys_api.id_bus.eq_ignore_ascii_case("usb")
                == disk.dev_links.to_ascii_lowercase().contains("usb")),
        "ceph-volume sys_api contradicts the Rook device record"
    );
    let removable = parse_ceph_sys_bool(&data.sys_api.removable)?;
    if data.available {
        ensure!(
            data.rejected_reasons.is_empty()
                && data.lvs.is_empty()
                && !removable
                && data.sys_api.partitions.is_empty(),
            "available ceph-volume device has rejection, LV, removable, or partition state"
        );
    } else {
        ensure!(
            !data.rejected_reasons.is_empty() || !data.lvs.is_empty(),
            "unavailable ceph-volume device has no rejection reason or logical volume"
        );
    }
    Ok((data.available, removable))
}

fn parse_ceph_sys_bool(value: &str) -> Result<bool> {
    match value.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => bail!("ceph-volume sys_api boolean is neither 0 nor 1"),
    }
}

fn json_value_is_meaningful(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.trim().is_empty(),
        Value::Array(values) => values.iter().any(json_value_is_meaningful),
        Value::Object(values) => {
            values.keys().all(|key| !key.trim().is_empty())
                && values.values().any(json_value_is_meaningful)
        }
        Value::Bool(_) | Value::Number(_) => true,
    }
}

fn parse_rook_disks(config_map: &ConfigMap) -> Result<Vec<LocalDisk>> {
    let raw = config_map
        .data
        .as_ref()
        .and_then(|data| data.get("devices"))
        .context("Rook discovery ConfigMap has no devices payload")?;
    let disks =
        serde_json::from_str::<Vec<LocalDisk>>(raw).context("parsing Rook devices payload")?;
    let mut validated = Vec::new();
    let mut kernel_names = BTreeSet::new();
    for mut disk in disks {
        let kernel_name = disk.kernel_name.as_deref().unwrap_or(&disk.name);
        ensure!(
            !kernel_name.is_empty()
                && kernel_name.trim() == kernel_name
                && !kernel_name.contains('/'),
            "Rook device has an invalid kernel name"
        );
        ensure!(
            kernel_names.insert(kernel_name.to_owned()),
            "Rook devices payload contains duplicate kernel device {kernel_name:?}"
        );
        let ceph_volume_data = disk
            .ceph_volume_data
            .as_deref()
            .with_context(|| format!("device {:?} has no ceph-volume inventory", disk.name))?;
        let (ceph_volume_available, ceph_volume_removable) =
            parse_ceph_volume_available(ceph_volume_data, &disk)
                .with_context(|| format!("validating ceph-volume inventory for {:?}", disk.name))?;
        if ceph_volume_available {
            ensure!(
                disk.empty
                    && !disk.read_only
                    && matches!(disk.device_type.as_str(), "disk" | "ssd" | "mpath"),
                "available ceph-volume device contradicts Rook device state"
            );
        }
        disk.ceph_volume_available = ceph_volume_available;
        disk.ceph_volume_removable = ceph_volume_removable;
        validated.push(disk);
    }
    Ok(validated)
}

fn disk_policy_allows(disk: &LocalDisk, minimum_bytes: u64) -> bool {
    disk.size >= minimum_bytes && !disk.dev_links.to_ascii_lowercase().contains("usb")
}

fn owner_matches(config_map: &ConfigMap, pod: &Pod, uid: &str) -> bool {
    config_map
        .metadata
        .owner_references
        .as_ref()
        .is_some_and(|references| {
            references.len() == 1
                && references
                    .iter()
                    .filter(|reference| {
                        reference.api_version == "v1"
                            && reference.kind == "Pod"
                            && reference.name == pod.metadata.name.as_deref().unwrap_or_default()
                            && reference.uid == uid
                    })
                    .count()
                    == 1
        })
}

fn pod_is_ready(pod: &Pod) -> bool {
    pod.status.as_ref().is_some_and(|status| {
        status.phase.as_deref() == Some("Running")
            && status.conditions.as_ref().is_some_and(|conditions| {
                conditions
                    .iter()
                    .any(|condition| condition.type_ == "Ready" && condition.status == "True")
            })
    })
}

fn current_inventory(
    node_name: &str,
    pods: &[Pod],
    config_maps: &BTreeMap<String, ConfigMap>,
    now: u64,
    max_age: u64,
) -> Result<NodeInventory> {
    let relevant = pods
        .iter()
        .filter(|pod| {
            pod.metadata.deletion_timestamp.is_none()
                && pod_is_ready(pod)
                && pod.spec.as_ref().and_then(|spec| spec.node_name.as_deref()) == Some(node_name)
        })
        .collect::<Vec<_>>();
    ensure!(
        relevant.len() == 1,
        "expected exactly one Ready inventory agent Pod on {node_name:?}, found {}",
        relevant.len()
    );
    let pod = relevant[0];
    let uid = pod
        .metadata
        .uid
        .as_deref()
        .context("inventory agent Pod has no UID")?;
    let expected_name = inventory_name(uid)?;
    let config_map = config_maps
        .get(&expected_name)
        .with_context(|| format!("missing inventory ConfigMap {expected_name:?}"))?;
    ensure!(
        config_map.metadata.name.as_deref() == Some(expected_name.as_str()),
        "inventory ConfigMap name does not match its Pod UID"
    );
    ensure!(
        owner_matches(config_map, pod, uid),
        "inventory ConfigMap owner does not match its current Pod"
    );
    let raw = config_map
        .data
        .as_ref()
        .and_then(|data| data.get(INVENTORY_KEY))
        .context("inventory ConfigMap has no inventory payload")?;
    let inventory = serde_json::from_str::<NodeInventory>(raw).context("parsing node inventory")?;
    validate_inventory(&inventory)?;
    ensure!(
        inventory.generated_at_unix <= now.saturating_add(FUTURE_SKEW_SECONDS),
        "inventory timestamp is too far in the future"
    );
    ensure!(
        now.saturating_sub(inventory.generated_at_unix) <= max_age,
        "inventory is stale"
    );
    Ok(inventory)
}

fn validate_inventory(inventory: &NodeInventory) -> Result<()> {
    let mut kernel_names = BTreeSet::new();
    let mut aliases = BTreeSet::new();
    for device in &inventory.devices {
        ensure!(
            is_by_id_path(&device.path),
            "inventory device path must be a non-empty by-id identity"
        );
        ensure!(
            !device.kernel_name.is_empty() && !device.kernel_name.contains('/'),
            "inventory kernel name must be a non-empty basename"
        );
        ensure!(
            kernel_names.insert(&device.kernel_name),
            "inventory contains duplicate kernel device records"
        );
        ensure!(
            !device.kernel_name.starts_with("nvme") || !device.rotational,
            "NVMe inventory device cannot be rotational"
        );
        ensure!(
            device.size_bytes > 0,
            "inventory device size must be positive"
        );
        ensure!(
            !device.aliases.is_empty()
                && device.aliases.iter().all(|alias| is_by_id_path(alias))
                && device.aliases.iter().any(|alias| alias == &device.path),
            "inventory aliases must be non-empty by-id identities containing the selected path"
        );
        let device_aliases = device.aliases.iter().collect::<BTreeSet<_>>();
        ensure!(
            device_aliases.len() == device.aliases.len(),
            "inventory device contains duplicate aliases"
        );
        for alias in device_aliases {
            ensure!(
                aliases.insert(alias),
                "inventory alias is claimed by multiple device records"
            );
        }
        if let Some(pcie) = &device.pcie {
            let hardware_class = pcie_hardware_class(pcie.speed_gts, pcie.width)
                .context("inventory PCIe speed or width is invalid")?;
            ensure!(
                device.kernel_name.starts_with("nvme")
                    && is_pci_bdf(&pcie.bdf)
                    && pcie.aggregate_gts.is_finite()
                    && pcie.aggregate_gts > 0.0
                    && (pcie.aggregate_gts - pcie.speed_gts * f64::from(pcie.width)).abs()
                        <= f64::EPSILON * pcie.aggregate_gts
                    && pcie.hardware_class == hardware_class,
                "inventory PCIe metadata is incoherent"
            );
        }
    }
    Ok(())
}

enum DeviceIntersection {
    Unmatched,
    Existing(String),
    Candidate(CandidateDevice),
}

fn intersect_device(
    rook: &LocalDisk,
    agent_devices: &[DeviceInventory],
    existing_identities: &BTreeSet<String>,
) -> Result<DeviceIntersection> {
    let kernel_name = rook.kernel_name.as_deref().unwrap_or(&rook.name);
    let mut rook_aliases = rook
        .dev_links
        .split_whitespace()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    ensure!(
        rook_aliases
            .iter()
            .all(|alias| !alias.starts_with("/dev/disk/by-id/") || is_by_id_path(alias)),
        "Rook device {:?} contains a malformed by-id alias",
        rook.name
    );
    rook_aliases.insert(format!("/dev/{kernel_name}"));
    rook_aliases.insert(kernel_name.to_owned());
    let mut matches = Vec::new();
    for agent in agent_devices {
        let aliases = agent.aliases.iter().chain(std::iter::once(&agent.path));
        let common = aliases
            .filter(|alias| rook_aliases.contains(*alias))
            .cloned()
            .collect::<BTreeSet<_>>();
        if common.is_empty() {
            continue;
        }
        ensure!(
            agent.kernel_name == kernel_name,
            "Rook device {:?} and agent identity disagree on kernel name",
            rook.name
        );
        matches.push((agent, common));
    }
    if matches.is_empty() {
        return Ok(DeviceIntersection::Unmatched);
    }
    if matches.len() > 1 {
        bail!(
            "Rook device {:?} matches multiple agent identities",
            rook.name
        );
    }
    let (agent, common) = &matches[0];
    ensure!(
        agent.size_bytes == rook.size
            && agent.read_only == rook.read_only
            && agent.rotational == rook.rotational
            && agent.removable == rook.ceph_volume_removable,
        "Rook and current agent device state disagree"
    );
    let matching_existing = rook_aliases
        .iter()
        .chain(agent.aliases.iter())
        .chain(std::iter::once(&agent.path))
        .map(|identity| normalize_device_identity(identity))
        .filter(|identity| existing_identities.contains(identity))
        .collect::<BTreeSet<_>>();
    ensure!(
        matching_existing.len() <= 1,
        "one current device matches multiple existing Ceph identities"
    );
    if let Some(identity) = matching_existing.into_iter().next() {
        return Ok(DeviceIntersection::Existing(identity));
    }
    if !rook.ceph_volume_available {
        return Ok(DeviceIntersection::Unmatched);
    }
    ensure!(
        agent.signature_free,
        "new Rook-positive device currently contains a block signature"
    );
    if !common.contains(&agent.path) {
        return Ok(DeviceIntersection::Unmatched);
    }
    let path = agent.path.clone();
    let is_nvme = agent.kernel_name.starts_with("nvme");
    let medium = if agent.rotational {
        "hdd"
    } else if is_nvme {
        "nvme"
    } else {
        "ssd"
    };
    let mut aliases = rook_aliases;
    aliases.extend(common.iter().cloned());
    aliases.extend(agent.aliases.iter().cloned());
    aliases.insert(agent.path.clone());
    aliases.insert(format!("/dev/{}", agent.kernel_name));
    aliases.insert(agent.kernel_name.clone());
    Ok(DeviceIntersection::Candidate(CandidateDevice {
        path,
        aliases: aliases.into_iter().collect(),
        medium: medium.to_owned(),
        hardware_class: is_nvme
            .then(|| agent.pcie.as_ref().map(|pcie| pcie.hardware_class.clone()))
            .flatten()
            .filter(|class| !class.is_empty()),
    }))
}

fn ceph_cluster_patch(resource_version: &str, storage: &Value) -> Value {
    json!({
        "metadata": {"resourceVersion": resource_version},
        "spec": {"storage": {"nodes": storage["nodes"].clone()}},
    })
}

fn claim_inventory_identities(
    claims: &mut BTreeMap<String, (String, String, usize)>,
    node_name: &str,
    inventory: &NodeInventory,
) -> Result<()> {
    for (device_index, device) in inventory.devices.iter().enumerate() {
        let claim = (
            node_name.to_owned(),
            device.kernel_name.clone(),
            device_index,
        );
        for identity in device
            .aliases
            .iter()
            .chain(std::iter::once(&device.path))
            .filter(|identity| is_by_id_path(identity))
        {
            if let Some(existing_claim) = claims.get(identity)
                && existing_claim != &claim
            {
                bail!(
                    "persistent device identity {identity:?} has conflicting \
                     claims {existing_claim:?} and {claim:?}"
                );
            }
            claims.insert(identity.clone(), claim.clone());
        }
    }
    Ok(())
}

type NodeIdentity = (String, String, String, String);
type EvidenceFingerprint = BTreeSet<(String, String, String, String, String)>;

fn node_identities_unchanged(
    expected: &BTreeSet<NodeIdentity>,
    current: &BTreeSet<NodeIdentity>,
) -> bool {
    expected == current
}

fn existing_device_nodes_are_covered(
    existing: &BTreeMap<String, BTreeSet<String>>,
    eligible_rook_names: &BTreeSet<String>,
) -> bool {
    existing
        .iter()
        .filter(|(_, identities)| !identities.is_empty())
        .all(|(node_name, _)| eligible_rook_names.contains(node_name))
}

fn group_discovery_by_node(items: Vec<ConfigMap>) -> Result<BTreeMap<String, Vec<ConfigMap>>> {
    let mut by_node = BTreeMap::<String, Vec<ConfigMap>>::new();
    for map in items {
        ensure!(
            map.metadata.deletion_timestamp.is_none(),
            "selected Rook discovery ConfigMap is terminating"
        );
        let node = map
            .metadata
            .labels
            .as_ref()
            .and_then(|labels| labels.get("rook.io/node"))
            .filter(|node| !node.is_empty())
            .context("Rook discovery ConfigMap has no non-empty rook.io/node label")?
            .clone();
        by_node.entry(node).or_default().push(map);
    }
    Ok(by_node)
}

fn evidence_fingerprint(
    discovery: &BTreeMap<String, Vec<ConfigMap>>,
    pods: &[Pod],
    inventories: &BTreeMap<String, ConfigMap>,
) -> Result<EvidenceFingerprint> {
    let mut fingerprint = BTreeSet::new();
    for (node_name, maps) in discovery {
        for map in maps {
            fingerprint.insert(resource_fingerprint(
                "rook-discovery",
                node_name,
                &map.metadata,
            )?);
        }
    }
    for pod in pods {
        let node_name = pod
            .spec
            .as_ref()
            .and_then(|spec| spec.node_name.as_deref())
            .unwrap_or_default();
        fingerprint.insert(resource_fingerprint("agent-pod", node_name, &pod.metadata)?);
    }
    for map in inventories.values() {
        fingerprint.insert(resource_fingerprint("agent-inventory", "", &map.metadata)?);
    }
    Ok(fingerprint)
}

fn resource_fingerprint(
    kind: &str,
    node_name: &str,
    metadata: &ObjectMeta,
) -> Result<(String, String, String, String, String)> {
    ensure!(
        metadata.deletion_timestamp.is_none(),
        "source resource is terminating"
    );
    Ok((
        kind.to_owned(),
        node_name.to_owned(),
        metadata
            .name
            .clone()
            .context("source resource has no name")?,
        metadata.uid.clone().context("source resource has no UID")?,
        metadata
            .resource_version
            .clone()
            .context("source resource has no resourceVersion")?,
    ))
}

fn collect_inventory_maps(items: Vec<ConfigMap>) -> Result<BTreeMap<String, ConfigMap>> {
    let mut maps = BTreeMap::new();
    for map in items {
        ensure!(
            map.metadata.deletion_timestamp.is_none(),
            "selected inventory ConfigMap is terminating"
        );
        let name = map
            .metadata
            .name
            .clone()
            .filter(|name| !name.is_empty())
            .context("selected inventory ConfigMap has no name")?;
        ensure!(
            maps.insert(name.clone(), map).is_none(),
            "duplicate selected inventory ConfigMap name {name:?}"
        );
    }
    Ok(maps)
}

fn validate_inventory_source_set(
    eligible_nodes: &BTreeSet<String>,
    pods: &[Pod],
    config_maps: &BTreeMap<String, ConfigMap>,
    now: u64,
    max_age: u64,
) -> Result<BTreeMap<String, NodeInventory>> {
    let mut pod_nodes = BTreeSet::new();
    let mut expected_maps = BTreeSet::new();
    for pod in pods {
        ensure!(
            pod.metadata.deletion_timestamp.is_none() && pod_is_ready(pod),
            "selected inventory agent Pod is not current and Ready"
        );
        let node_name = pod
            .spec
            .as_ref()
            .and_then(|spec| spec.node_name.as_deref())
            .context("selected inventory agent Pod has no nodeName")?;
        ensure!(
            eligible_nodes.contains(node_name),
            "inventory agent Pod is bound to ineligible node {node_name:?}"
        );
        ensure!(
            pod_nodes.insert(node_name.to_owned()),
            "multiple selected inventory agent Pods are bound to {node_name:?}"
        );
        let uid = pod
            .metadata
            .uid
            .as_deref()
            .context("selected inventory agent Pod has no UID")?;
        expected_maps.insert(inventory_name(uid)?);
    }
    ensure!(
        pod_nodes == *eligible_nodes,
        "inventory agent Pod node set does not match eligible Storage nodes"
    );
    ensure!(
        expected_maps == config_maps.keys().cloned().collect(),
        "inventory ConfigMap set does not match current agent Pods"
    );
    eligible_nodes
        .iter()
        .map(|node_name| {
            current_inventory(node_name, pods, config_maps, now, max_age)
                .map(|inventory| (node_name.clone(), inventory))
                .with_context(|| format!("validating inventory source for {node_name:?}"))
        })
        .collect()
}

async fn reconcile_once(client: &Client, config: &RuntimeConfig) -> Result<()> {
    let nodes_api = Api::<Node>::all(client.clone());
    let nodes = nodes_api
        .list(&ListParams::default())
        .await
        .context("listing Kubernetes Nodes")?;
    let mut eligible = nodes
        .items
        .into_iter()
        .filter(|node| has_storage_role(node, &config.args))
        .map(|node| {
            ensure!(
                node.metadata.deletion_timestamp.is_none(),
                "eligible Storage Node is terminating"
            );
            let name = node
                .metadata
                .name
                .clone()
                .context("Kubernetes Node has no name")?;
            let uid = node
                .metadata
                .uid
                .clone()
                .context("Kubernetes Node has no UID")?;
            let role_value = storage_role_value(&node, &config.args)
                .context("Kubernetes Node lost its Storage role")?
                .to_owned();
            Ok((name, uid, rook_node_name(&node)?, role_value))
        })
        .collect::<Result<Vec<_>>>()?;
    eligible.sort();

    let discovery_api =
        Api::<ConfigMap>::namespaced(client.clone(), &config.args.rook_discovery_namespace);
    let discovery = discovery_api
        .list(&ListParams::default().labels(ROOK_DISCOVERY_SELECTOR))
        .await
        .context("listing Rook discovery ConfigMaps")?;
    let discovery_by_node = group_discovery_by_node(discovery.items)?;

    let pods_api = Api::<Pod>::namespaced(client.clone(), &config.args.provisioning_namespace);
    let pod_selector = format!(
        "app.kubernetes.io/instance={},app.kubernetes.io/component=agent",
        config.args.app_instance
    );
    let pods = pods_api
        .list(&ListParams::default().labels(&pod_selector))
        .await
        .context("listing current inventory agent Pods")?
        .items;
    let inventory_api =
        Api::<ConfigMap>::namespaced(client.clone(), &config.args.provisioning_namespace);
    let inventory_selector = format!(
        "{INVENTORY_SELECTOR},app.kubernetes.io/instance={}",
        config.args.app_instance
    );
    let inventory_maps = collect_inventory_maps(
        inventory_api
            .list(&ListParams::default().labels(&inventory_selector))
            .await
            .context("listing inventory ConfigMaps")?
            .items,
    )?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock precedes Unix epoch")?
        .as_secs();
    let eligible_node_names = eligible
        .iter()
        .map(|(name, _, _, _)| name.clone())
        .collect::<BTreeSet<_>>();
    let inventories = validate_inventory_source_set(
        &eligible_node_names,
        &pods,
        &inventory_maps,
        now,
        config.args.inventory_max_age_seconds,
    )?;
    for maps in discovery_by_node.values() {
        for map in maps {
            parse_rook_disks(map)?;
        }
    }
    let initial_evidence = evidence_fingerprint(&discovery_by_node, &pods, &inventory_maps)?;
    let resource =
        ApiResource::from_gvk(&GroupVersionKind::gvk("ceph.rook.io", "v1", "CephCluster"));
    let ceph_api = Api::<DynamicObject>::namespaced_with(
        client.clone(),
        &config.args.ceph_cluster_namespace,
        &resource,
    );
    let cluster = ceph_api
        .get(&config.args.ceph_cluster_name)
        .await
        .context("getting CephCluster")?;
    ensure!(
        cluster.metadata.deletion_timestamp.is_none(),
        "CephCluster is terminating"
    );
    let cluster_json = serde_json::to_value(&cluster).context("serializing CephCluster")?;
    let storage = cluster_json
        .pointer("/spec/storage")
        .context("CephCluster spec.storage is missing")?;
    let existing_by_node = existing_device_identities_by_node(storage)?;
    let eligible_rook_names = eligible
        .iter()
        .map(|(_, _, rook_name, _)| rook_name.clone())
        .collect::<BTreeSet<_>>();
    ensure!(
        eligible_rook_names.len() == eligible.len(),
        "eligible Kubernetes Nodes have duplicate Rook hostnames"
    );
    ensure!(
        existing_device_nodes_are_covered(&existing_by_node, &eligible_rook_names),
        "an existing Ceph device node has no eligible current Rook hostname"
    );

    let mut candidates = Vec::new();
    let mut observed_claims = Vec::new();
    let mut inventory_identities = BTreeMap::new();
    for (kubernetes_name, _, rook_name, _) in &eligible {
        let discovery = discovery_by_node
            .get(kubernetes_name)
            .filter(|maps| maps.len() == 1)
            .map(|maps| &maps[0])
            .with_context(|| {
                format!(
                    "expected exactly one Rook discovery ConfigMap for \
                     Storage node {kubernetes_name:?}"
                )
            })?;
        let inventory = inventories
            .get(kubernetes_name)
            .context("validated inventory set lost a Storage node")?;
        claim_inventory_identities(&mut inventory_identities, kubernetes_name, inventory)?;
        observed_claims.push(NodeIdentityClaims {
            name: rook_name.clone(),
            identities: inventory
                .devices
                .iter()
                .flat_map(|device| device.aliases.iter().chain(std::iter::once(&device.path)))
                .filter(|identity| is_by_id_path(identity))
                .cloned()
                .collect(),
        });
        let mut devices = Vec::new();
        let mut validated_existing = BTreeSet::new();
        let existing_identities = existing_by_node.get(rook_name).cloned().unwrap_or_default();
        for disk in parse_rook_disks(discovery)? {
            match intersect_device(&disk, &inventory.devices, &existing_identities)? {
                DeviceIntersection::Unmatched => {}
                DeviceIntersection::Existing(identity) => {
                    ensure!(
                        validated_existing.insert(identity.clone()),
                        "existing Ceph identity {identity:?} matches multiple Rook records"
                    );
                }
                DeviceIntersection::Candidate(device)
                    if disk_policy_allows(&disk, config.args.minimum_device_bytes) =>
                {
                    devices.push(device);
                }
                DeviceIntersection::Candidate(_) => {}
            }
        }
        ensure!(
            validated_existing == existing_identities,
            "existing Ceph devices do not match current Rook and agent evidence"
        );
        devices.sort_by(|left, right| left.path.cmp(&right.path));
        candidates.push(NodeCandidates {
            name: rook_name.clone(),
            devices,
        });
    }
    candidates.sort_by(|left, right| left.name.cmp(&right.name));

    let plan = plan_storage_with_claims(storage, &candidates, &observed_claims, &config.planner)?;
    if plan.summary.devices_added == 0 {
        #[cfg(feature = "tracing")]
        tracing::debug!("CephCluster storage already reconciled");
        return Ok(());
    }

    let current_nodes = nodes_api
        .list(&ListParams::default())
        .await
        .context("re-reading Kubernetes Nodes")?;
    let current_identities = current_nodes
        .items
        .into_iter()
        .filter(|node| has_storage_role(node, &config.args))
        .map(|node| {
            ensure!(
                node.metadata.deletion_timestamp.is_none(),
                "current Storage Node is terminating"
            );
            let name = node
                .metadata
                .name
                .clone()
                .context("current Storage Node has no name")?;
            let uid = node
                .metadata
                .uid
                .clone()
                .context("current Storage Node has no UID")?;
            let role_value = storage_role_value(&node, &config.args)
                .context("current Storage Node lost its role value")?
                .to_owned();
            Ok((name, uid, rook_node_name(&node)?, role_value))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    let expected_identities = eligible.iter().cloned().collect::<BTreeSet<_>>();
    ensure!(
        node_identities_unchanged(&expected_identities, &current_identities),
        "the Storage Node identity set changed before mutation"
    );

    let current_discovery = group_discovery_by_node(
        discovery_api
            .list(&ListParams::default().labels(ROOK_DISCOVERY_SELECTOR))
            .await
            .context("re-reading Rook discovery ConfigMaps")?
            .items,
    )?;
    let current_pods = pods_api
        .list(&ListParams::default().labels(&pod_selector))
        .await
        .context("re-reading inventory agent Pods")?
        .items;
    let current_inventory_maps = collect_inventory_maps(
        inventory_api
            .list(&ListParams::default().labels(&inventory_selector))
            .await
            .context("re-reading inventory ConfigMaps")?
            .items,
    )?;
    let current_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .context("system clock precedes Unix epoch during final validation")?
        .as_secs();
    validate_inventory_source_set(
        &eligible_node_names,
        &current_pods,
        &current_inventory_maps,
        current_now,
        config.args.inventory_max_age_seconds,
    )?;
    for maps in current_discovery.values() {
        for map in maps {
            parse_rook_disks(map)?;
        }
    }
    for (kubernetes_name, _, _, _) in &eligible {
        let discovery = current_discovery
            .get(kubernetes_name)
            .filter(|maps| maps.len() == 1)
            .map(|maps| &maps[0])
            .with_context(|| {
                format!(
                    "expected exactly one current Rook discovery ConfigMap for \
                     Storage node {kubernetes_name:?}"
                )
            })?;
        parse_rook_disks(discovery)?;
    }
    let current_evidence =
        evidence_fingerprint(&current_discovery, &current_pods, &current_inventory_maps)?;
    ensure!(
        initial_evidence == current_evidence,
        "device discovery evidence changed before mutation"
    );

    let resource_version = cluster
        .metadata
        .resource_version
        .context("CephCluster has no resourceVersion")?;
    let patch = ceph_cluster_patch(&resource_version, &plan.storage);
    let params = PatchParams {
        dry_run: config.args.dry_run,
        force: false,
        field_manager: Some("openark-rook-ceph-controller".to_owned()),
        field_validation: None,
    };
    ceph_api
        .patch(
            &config.args.ceph_cluster_name,
            &params,
            &Patch::Merge(&patch),
        )
        .await
        .context("patching CephCluster storage")?;
    #[cfg(feature = "tracing")]
    if config.args.dry_run {
        tracing::info!(
            patch = %patch,
            nodes_added = plan.summary.nodes_added,
            devices_added = plan.summary.devices_added,
            "validated additive CephCluster storage patch"
        );
    } else {
        tracing::info!(
            nodes_added = plan.summary.nodes_added,
            devices_added = plan.summary.devices_added,
            "reconciled additive CephCluster storage"
        );
    }
    Ok(())
}

async fn try_main(config: RuntimeConfig) -> Result<()> {
    let client = Client::try_default()
        .await
        .context("creating Kubernetes client")?;
    let mut ticker = interval(Duration::from_secs(config.args.poll_interval_seconds));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Delay);
    loop {
        ticker.tick().await;
        if let Err(error) = reconcile_once(&client, &config).await {
            #[cfg(feature = "tracing")]
            tracing::error!(error = ?error, "Ceph reconciliation failed; retrying next interval");
            #[cfg(not(feature = "tracing"))]
            let _ = error;
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let config = validate_args(Args::parse())?;
    openark_core::init_once();
    #[cfg(feature = "tracing")]
    tracing::info!("Welcome to OpenARK Rook Ceph Controller!");
    try_main(config).await
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use clap::{CommandFactory, FromArgMatches};
    use k8s_openapi::{
        api::core::v1::{ConfigMap, Node, Pod, PodCondition, PodSpec, PodStatus},
        apimachinery::pkg::apis::meta::v1::OwnerReference,
    };
    use kube::api::ObjectMeta;
    use openark_rook_ceph_controller::model::{DeviceInventory, NodeInventory, PcieLink};

    use super::{
        Args, DeviceIntersection, INVENTORY_KEY, ceph_cluster_patch, claim_inventory_identities,
        current_inventory, disk_policy_allows, evidence_fingerprint,
        existing_device_nodes_are_covered, group_discovery_by_node, intersect_device,
        node_identities_unchanged, parse_rook_disks, rook_node_name, validate_inventory,
    };

    fn discovery(devices: &str) -> ConfigMap {
        ConfigMap {
            data: Some(BTreeMap::from([("devices".to_owned(), devices.to_owned())])),
            ..Default::default()
        }
    }

    #[test]
    fn dry_run_defaults_true_without_env_or_cli_override() {
        let matches = Args::command()
            .mut_arg("dry_run", |arg| arg.env(None::<&str>))
            .try_get_matches_from([
                "openark-rook-ceph-controller",
                "--ceph-cluster-namespace",
                "rook-ceph",
                "--ceph-cluster-name",
                "rook-ceph",
                "--rook-discovery-namespace",
                "rook-ceph",
                "--provisioning-namespace",
                "openark",
                "--app-instance",
                "storage",
                "--minimum-device-bytes",
                "100",
                "--device-class-map",
                r#"{"default":"ssd"}"#,
            ])
            .unwrap();
        let args = Args::from_arg_matches(&matches).unwrap();

        assert!(args.dry_run);
    }

    #[test]
    fn rook_payload_requires_positive_ceph_volume_evidence() {
        let available = r#"[{
            "name":"sda",
            "kernel-name":"sda",
            "devLinks":"/dev/disk/by-id/wwn-a /dev/disk/by-path/pci-a",
            "size":100,
            "type":"disk",
            "rotational":true,
            "readOnly":false,
            "empty":true,
            "cephVolumeData":"{\"path\":\"/dev/sda\",\"available\":true,\"rejected_reasons\":[],\"sys_api\":{\"path\":\"/dev/sda\",\"devname\":\"sda\",\"type\":\"disk\",\"size\":100,\"ro\":\"0\",\"removable\":\"0\",\"rotational\":\"1\",\"partitions\":{},\"id_bus\":\"\"},\"lvs\":[]}"
        }]"#;
        let devices = parse_rook_disks(&discovery(available)).unwrap();
        assert_eq!(devices.len(), 1);
        assert_eq!(
            devices[0].dev_links,
            "/dev/disk/by-id/wwn-a /dev/disk/by-path/pci-a"
        );
        let mut policy_excluded = devices[0].clone();
        policy_excluded.size = 1;
        assert!(!disk_policy_allows(&policy_excluded, 100));
        policy_excluded.size = 100;
        policy_excluded.dev_links.push_str(" /dev/disk/by-id/usb-a");
        assert!(!disk_policy_allows(&policy_excluded, 100));

        let missing = available.replacen(r#""cephVolumeData""#, r#""ignored""#, 1);
        assert!(parse_rook_disks(&discovery(&missing)).is_err());
        let contradictory = available.replace(r#"\"available\":true"#, r#"\"available\":false"#);
        assert!(parse_rook_disks(&discovery(&contradictory)).is_err());
        let unavailable = contradictory.replace(
            r#"\"rejected_reasons\":[]"#,
            r#"\"rejected_reasons\":[\"LVM detected\"]"#,
        );
        let unavailable_disks = parse_rook_disks(&discovery(&unavailable)).unwrap();
        assert_eq!(unavailable_disks.len(), 1);
        assert!(!unavailable_disks[0].ceph_volume_available);
        let malformed_negative = unavailable.replace(r#"[\"LVM detected\"]"#, "[1]");
        assert!(parse_rook_disks(&discovery(&malformed_negative)).is_err());
        let whitespace_negative = unavailable.replace("LVM detected", "   ");
        assert!(parse_rook_disks(&discovery(&whitespace_negative)).is_err());
        assert!(parse_rook_disks(&discovery("{")).is_err());
        let malformed = available.replace(
            r#"{\"path\":\"/dev/sda\",\"available\":true,\"rejected_reasons\":[],\"sys_api\":{\"path\":\"/dev/sda\",\"devname\":\"sda\",\"type\":\"disk\",\"size\":100,\"ro\":\"0\",\"removable\":\"0\",\"rotational\":\"1\",\"partitions\":{},\"id_bus\":\"\"},\"lvs\":[]}"#,
            "not-json",
        );
        assert!(parse_rook_disks(&discovery(&malformed)).is_err());
        let filtered_malformed = malformed.replace(r#""size":100"#, r#""size":1"#);
        assert!(parse_rook_disks(&discovery(&filtered_malformed)).is_err());
        let wrong_path = available.replace(r#"\"path\":\"/dev/sda\""#, r#"\"path\":\"/dev/sdb\""#);
        assert!(parse_rook_disks(&discovery(&wrong_path)).is_err());
        let inconsistent = available.replace(
            r#"\"rejected_reasons\":[]"#,
            r#"\"rejected_reasons\":[\"in-use\"]"#,
        );
        assert!(parse_rook_disks(&discovery(&inconsistent)).is_err());
        let contradictory_state = available.replace(r#""empty":true"#, r#""empty":false"#);
        assert!(parse_rook_disks(&discovery(&contradictory_state)).is_err());
        for contradictory_sys_api in [
            available.replace(r#"\"rotational\":\"1\""#, r#"\"rotational\":\"0\""#),
            available.replace(r#"\"size\":100"#, r#"\"size\":99"#),
            available.replace(r#"\"ro\":\"0\""#, r#"\"ro\":\"1\""#),
            available.replace(
                "/dev/disk/by-id/wwn-a /dev/disk/by-path/pci-a",
                "/dev/disk/by-id/wwn-a /dev/disk/by-id/usb-a",
            ),
        ] {
            assert!(parse_rook_disks(&discovery(&contradictory_sys_api)).is_err());
        }

        let record = available
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
            .unwrap();
        let negative_record = record
            .replace(r#"\"available\":true"#, r#"\"available\":false"#)
            .replace(
                r#"\"rejected_reasons\":[]"#,
                r#"\"rejected_reasons\":[\"in-use\"]"#,
            );
        let duplicate = format!("[{record},{negative_record}]");
        assert!(parse_rook_disks(&discovery(&duplicate)).is_err());
    }

    #[test]
    fn agent_identity_only_enriches_a_rook_device() {
        let rook = parse_rook_disks(&discovery(
            r#"[{
                "name":"sda",
                "kernel-name":"sda",
                "devLinks":"/dev/disk/by-id/wwn-a /dev/disk/by-path/pci-a",
                "size":100,
                "type":"disk",
                "rotational":true,
                "readOnly":false,
                "empty":true,
                "cephVolumeData":"{\"path\":\"/dev/sda\",\"available\":true,\"rejected_reasons\":[],\"sys_api\":{\"path\":\"/dev/sda\",\"devname\":\"sda\",\"type\":\"disk\",\"size\":100,\"ro\":\"0\",\"removable\":\"0\",\"rotational\":\"1\",\"partitions\":{},\"id_bus\":\"\"},\"lvs\":[]}"
            }]"#,
        ))
        .unwrap()
        .remove(0);
        let verified = DeviceInventory {
            path: "/dev/disk/by-id/wwn-a".to_owned(),
            aliases: vec!["/dev/disk/by-id/wwn-a".to_owned()],
            kernel_name: "sda".to_owned(),
            size_bytes: 100,
            read_only: false,
            removable: false,
            rotational: true,
            signature_free: true,
            pcie: None,
        };
        let DeviceIntersection::Candidate(candidate) =
            intersect_device(&rook, std::slice::from_ref(&verified), &BTreeSet::new()).unwrap()
        else {
            panic!("expected a new candidate");
        };
        assert_eq!(candidate.path, "/dev/disk/by-id/wwn-a");
        assert!(
            candidate
                .aliases
                .contains(&"/dev/disk/by-path/pci-a".to_owned())
        );
        assert!(candidate.aliases.contains(&"sda".to_owned()));

        let mut alternate_rook = rook.clone();
        alternate_rook.dev_links = "/dev/disk/by-id/scsi-a".to_owned();
        let alternate = DeviceInventory {
            path: "/dev/disk/by-id/wwn-a".to_owned(),
            aliases: vec![
                "/dev/disk/by-id/wwn-a".to_owned(),
                "/dev/disk/by-id/scsi-a".to_owned(),
            ],
            kernel_name: "sda".to_owned(),
            size_bytes: 100,
            read_only: false,
            removable: false,
            rotational: true,
            signature_free: true,
            pcie: None,
        };
        assert!(matches!(
            intersect_device(
                &alternate_rook,
                std::slice::from_ref(&alternate),
                &BTreeSet::new(),
            )
            .unwrap(),
            DeviceIntersection::Unmatched
        ));
        alternate_rook.dev_links = "/dev/disk/by-id/wwn-a /dev/disk/by-id/scsi-a".to_owned();
        let DeviceIntersection::Candidate(candidate) =
            intersect_device(&alternate_rook, &[alternate], &BTreeSet::new()).unwrap()
        else {
            panic!("expected a new candidate");
        };
        assert_eq!(candidate.path, "/dev/disk/by-id/wwn-a");
        assert!(
            candidate
                .aliases
                .contains(&"/dev/disk/by-id/scsi-a".to_owned())
        );

        let mut mismatched = verified.clone();
        mismatched.kernel_name = "sdb".to_owned();
        assert!(intersect_device(&rook, &[mismatched], &BTreeSet::new()).is_err());
        let mut changed = verified.clone();
        changed.read_only = true;
        assert!(intersect_device(&rook, &[changed], &BTreeSet::new()).is_err());
        let mut signed = verified.clone();
        signed.signature_free = false;
        assert!(intersect_device(&rook, &[signed], &BTreeSet::new()).is_err());
        let mut signed_existing = verified.clone();
        signed_existing.signature_free = false;
        assert!(matches!(
            intersect_device(
                &rook,
                &[signed_existing],
                &BTreeSet::from(["/dev/disk/by-id/wwn-a".to_owned()]),
            )
            .unwrap(),
            DeviceIntersection::Existing(identity)
                if identity == "/dev/disk/by-id/wwn-a"
        ));
        let mut negative_rook = rook.clone();
        negative_rook.ceph_volume_available = false;
        let mut signed_existing = verified.clone();
        signed_existing.signature_free = false;
        assert!(matches!(
            intersect_device(
                &negative_rook,
                &[signed_existing],
                &BTreeSet::from(["/dev/disk/by-id/wwn-a".to_owned()]),
            )
            .unwrap(),
            DeviceIntersection::Existing(identity)
                if identity == "/dev/disk/by-id/wwn-a"
        ));
        negative_rook.ceph_volume_removable = true;
        let mut signed_existing = verified.clone();
        signed_existing.signature_free = false;
        assert!(
            intersect_device(
                &negative_rook,
                &[signed_existing],
                &BTreeSet::from(["/dev/disk/by-id/wwn-a".to_owned()]),
            )
            .is_err()
        );
        let mut small_rook = rook.clone();
        small_rook.size = 1;
        let mut signed_small = verified.clone();
        signed_small.size_bytes = 1;
        signed_small.signature_free = false;
        assert!(intersect_device(&small_rook, &[signed_small], &BTreeSet::new()).is_err());

        let unverified = DeviceInventory {
            path: "/dev/disk/by-id/wwn-other".to_owned(),
            aliases: vec!["/dev/disk/by-id/wwn-other".to_owned()],
            kernel_name: "sda".to_owned(),
            size_bytes: 100,
            read_only: false,
            removable: false,
            rotational: true,
            signature_free: true,
            pcie: None,
        };
        assert!(matches!(
            intersect_device(&rook, &[unverified], &BTreeSet::new()).unwrap(),
            DeviceIntersection::Unmatched
        ));
        assert!(intersect_device(&rook, &[verified.clone(), verified], &BTreeSet::new(),).is_err());
    }

    fn pod_and_inventory(generated_at_unix: u64) -> (Pod, ConfigMap) {
        let uid = "123e4567-e89b-12d3-a456-426614174000";
        let pod = Pod {
            metadata: ObjectMeta {
                name: Some("agent-a".to_owned()),
                uid: Some(uid.to_owned()),
                ..Default::default()
            },
            spec: Some(PodSpec {
                node_name: Some("node-a".to_owned()),
                containers: Vec::new(),
                ..Default::default()
            }),
            status: Some(PodStatus {
                phase: Some("Running".to_owned()),
                conditions: Some(vec![PodCondition {
                    status: "True".to_owned(),
                    type_: "Ready".to_owned(),
                    ..Default::default()
                }]),
                ..Default::default()
            }),
        };
        let inventory = NodeInventory {
            generated_at_unix,
            devices: Vec::new(),
        };
        let config_map = ConfigMap {
            metadata: ObjectMeta {
                name: Some(format!("rook-ceph-inventory-{uid}")),
                owner_references: Some(vec![OwnerReference {
                    api_version: "v1".to_owned(),
                    block_owner_deletion: Some(false),
                    controller: Some(false),
                    kind: "Pod".to_owned(),
                    name: "agent-a".to_owned(),
                    uid: uid.to_owned(),
                }]),
                ..Default::default()
            },
            data: Some(BTreeMap::from([(
                INVENTORY_KEY.to_owned(),
                serde_json::to_string(&inventory).unwrap(),
            )])),
            ..Default::default()
        };
        (pod, config_map)
    }

    #[test]
    fn inventory_age_is_bounded_in_both_directions() {
        for (generated, accepted) in [(700, true), (699, false), (1_030, true), (1_031, false)] {
            let (pod, config_map) = pod_and_inventory(generated);
            let maps = BTreeMap::from([(config_map.metadata.name.clone().unwrap(), config_map)]);
            assert_eq!(
                current_inventory("node-a", &[pod], &maps, 1_000, 300).is_ok(),
                accepted
            );
        }
    }

    #[test]
    fn every_ready_agent_source_must_be_valid_and_unique() {
        let (pod, config_map) = pod_and_inventory(1_000);
        let maps = BTreeMap::from([(config_map.metadata.name.clone().unwrap(), config_map)]);
        assert!(current_inventory("node-a", &[pod.clone(), pod], &maps, 1_000, 300).is_err());

        let (pod, mut config_map) = pod_and_inventory(1_000);
        let invalid = NodeInventory {
            generated_at_unix: 1_000,
            devices: vec![DeviceInventory {
                path: "/dev/sda".to_owned(),
                aliases: vec!["/dev/sda".to_owned()],
                kernel_name: "sda".to_owned(),
                size_bytes: 1,
                read_only: false,
                removable: false,
                rotational: false,
                signature_free: true,
                pcie: None,
            }],
        };
        config_map.data.as_mut().unwrap().insert(
            INVENTORY_KEY.to_owned(),
            serde_json::to_string(&invalid).unwrap(),
        );
        let maps = BTreeMap::from([(config_map.metadata.name.clone().unwrap(), config_map)]);
        assert!(current_inventory("node-a", &[pod], &maps, 1_000, 300).is_err());

        let invalid_pcie = NodeInventory {
            generated_at_unix: 1_000,
            devices: vec![DeviceInventory {
                path: "/dev/disk/by-id/nvme-eui.1".to_owned(),
                aliases: vec!["/dev/disk/by-id/nvme-eui.1".to_owned()],
                kernel_name: "nvme0n1".to_owned(),
                size_bytes: 1,
                read_only: false,
                removable: false,
                rotational: false,
                signature_free: true,
                pcie: Some(PcieLink {
                    bdf: "not-a-bdf".to_owned(),
                    speed_gts: 8.0,
                    width: 4,
                    aggregate_gts: 32.0,
                    hardware_class: "nvme-pcie-32gt".to_owned(),
                }),
            }],
        };
        assert!(validate_inventory(&invalid_pcie).is_err());

        let duplicate_kernel = NodeInventory {
            generated_at_unix: 1_000,
            devices: vec![
                DeviceInventory {
                    path: "/dev/disk/by-id/wwn-a".to_owned(),
                    aliases: vec!["/dev/disk/by-id/wwn-a".to_owned()],
                    kernel_name: "sda".to_owned(),
                    size_bytes: 100,
                    read_only: false,
                    removable: false,
                    rotational: true,
                    signature_free: false,
                    pcie: None,
                },
                DeviceInventory {
                    path: "/dev/disk/by-id/scsi-b".to_owned(),
                    aliases: vec!["/dev/disk/by-id/scsi-b".to_owned()],
                    kernel_name: "sda".to_owned(),
                    size_bytes: 100,
                    read_only: false,
                    removable: false,
                    rotational: true,
                    signature_free: false,
                    pcie: None,
                },
            ],
        };
        assert!(validate_inventory(&duplicate_kernel).is_err());
    }

    #[test]
    fn runtime_patch_owns_only_storage_nodes() {
        let patch = ceph_cluster_patch(
            "42",
            &serde_json::json!({
                "useAllNodes": false,
                "useAllDevices": false,
                "deviceFilter": "ignored",
                "nodes": [{"name":"node-a"}],
            }),
        );
        assert_eq!(patch["metadata"]["resourceVersion"], "42");
        assert_eq!(patch["spec"]["storage"]["nodes"][0]["name"], "node-a");
        assert_eq!(patch["spec"]["storage"].as_object().unwrap().len(), 1);
    }

    #[test]
    fn rook_node_identity_requires_hostname_label() {
        let mut node = Node::default();
        node.metadata.name = Some("node-a".to_owned());
        assert!(rook_node_name(&node).is_err());

        node.metadata.labels = Some(BTreeMap::from([(
            "kubernetes.io/hostname".to_owned(),
            "host-a".to_owned(),
        )]));
        assert_eq!(rook_node_name(&node).unwrap(), "host-a");
    }

    #[test]
    fn shared_inventory_identity_is_rejected_across_nodes() {
        let inventory = NodeInventory {
            generated_at_unix: 1,
            devices: vec![DeviceInventory {
                path: "/dev/disk/by-id/wwn-shared".to_owned(),
                aliases: vec!["/dev/disk/by-id/wwn-shared".to_owned()],
                kernel_name: "sda".to_owned(),
                size_bytes: 1,
                read_only: false,
                removable: false,
                rotational: false,
                signature_free: false,
                pcie: None,
            }],
        };
        let mut claims = BTreeMap::new();
        claim_inventory_identities(&mut claims, "node-a", &inventory).unwrap();
        assert!(claim_inventory_identities(&mut claims, "node-b", &inventory).is_err());

        let duplicate = NodeInventory {
            generated_at_unix: 1,
            devices: vec![inventory.devices[0].clone(), inventory.devices[0].clone()],
        };
        assert!(claim_inventory_identities(&mut BTreeMap::new(), "node-a", &duplicate).is_err());
    }

    #[test]
    fn storage_node_set_must_not_change_before_patch() {
        let node_a = (
            "node-a".to_owned(),
            "uid-a".to_owned(),
            "host-a".to_owned(),
            "Storage".to_owned(),
        );
        let expected = BTreeSet::from([node_a.clone()]);
        let mut current = expected.clone();
        assert!(node_identities_unchanged(&expected, &current));
        current.insert((
            "node-b".to_owned(),
            "uid-b".to_owned(),
            "host-b".to_owned(),
            "Storage".to_owned(),
        ));
        assert!(!node_identities_unchanged(&expected, &current));

        let eligible = BTreeSet::from(["node-a".to_owned()]);
        let existing = BTreeMap::from([(
            "retired-node".to_owned(),
            BTreeSet::from(["/dev/disk/by-id/wwn-a".to_owned()]),
        )]);
        assert!(!existing_device_nodes_are_covered(&existing, &eligible));
    }

    #[test]
    fn source_resource_version_change_invalidates_evidence() {
        let map = |resource_version: &str| ConfigMap {
            metadata: ObjectMeta {
                name: Some("local-device-node-a".to_owned()),
                uid: Some("discovery-uid".to_owned()),
                resource_version: Some(resource_version.to_owned()),
                labels: Some(BTreeMap::from([(
                    "rook.io/node".to_owned(),
                    "node-a".to_owned(),
                )])),
                ..Default::default()
            },
            ..Default::default()
        };
        let first = evidence_fingerprint(
            &BTreeMap::from([("node-a".to_owned(), vec![map("1")])]),
            &[],
            &BTreeMap::new(),
        )
        .unwrap();
        let second = evidence_fingerprint(
            &BTreeMap::from([("node-a".to_owned(), vec![map("2")])]),
            &[],
            &BTreeMap::new(),
        )
        .unwrap();
        assert_ne!(first, second);
        assert!(group_discovery_by_node(vec![ConfigMap::default()]).is_err());
    }
}
