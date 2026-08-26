use std::collections::{BTreeMap, BTreeSet};

use anyhow::{Context, Result, bail, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::model::is_by_id_path;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateDevice {
    pub path: String,
    pub aliases: Vec<String>,
    pub medium: String,
    pub hardware_class: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeCandidates {
    pub name: String,
    pub devices: Vec<CandidateDevice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NodeIdentityClaims {
    pub name: String,
    pub identities: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannerConfig {
    pub osds_per_device: u32,
    pub device_classes: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlanSummary {
    pub nodes_added: usize,
    pub devices_added: usize,
    pub devices_skipped_existing: usize,
    pub devices_skipped_unmapped: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub storage: Value,
    pub summary: PlanSummary,
}

pub fn existing_device_identities_by_node(
    storage: &Value,
) -> Result<BTreeMap<String, BTreeSet<String>>> {
    let storage = storage
        .as_object()
        .context("Ceph storage must be an object")?;
    let nodes = match storage.get("nodes") {
        None => return Ok(BTreeMap::new()),
        Some(Value::Array(nodes)) => nodes,
        Some(_) => bail!("Ceph storage nodes must be an array"),
    };
    let mut by_node = BTreeMap::new();
    for node in nodes {
        let node = node
            .as_object()
            .context("existing Ceph node must be an object")?;
        let node_name = node
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .context("existing Ceph node name must be a non-empty string")?;
        let devices = match node.get("devices") {
            None => Vec::new(),
            Some(Value::Array(devices)) => devices
                .iter()
                .map(|device| {
                    let name = device
                        .as_object()
                        .context("existing Ceph device must be an object")?
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.is_empty())
                        .context("existing Ceph device name must be a non-empty string")?;
                    Ok(normalize_device_identity(name))
                })
                .collect::<Result<Vec<_>>>()?,
            Some(_) => bail!("existing Ceph node devices must be an array"),
        };
        let identities = devices.into_iter().collect::<BTreeSet<_>>();
        ensure!(
            identities.len()
                == node
                    .get("devices")
                    .and_then(Value::as_array)
                    .map_or(0, Vec::len),
            "existing Ceph node {node_name:?} has duplicate device identities"
        );
        ensure!(
            by_node.insert(node_name.to_owned(), identities).is_none(),
            "duplicate existing Ceph node name {node_name:?}"
        );
    }
    Ok(by_node)
}

pub fn plan_storage(
    storage: &Value,
    candidates: &[NodeCandidates],
    config: &PlannerConfig,
) -> Result<Plan> {
    plan_storage_with_claims(storage, candidates, &[], config)
}

pub fn plan_storage_with_claims(
    storage: &Value,
    candidates: &[NodeCandidates],
    observed: &[NodeIdentityClaims],
    config: &PlannerConfig,
) -> Result<Plan> {
    if config.osds_per_device == 0 {
        bail!("osdsPerDevice must be greater than zero");
    }
    if config
        .device_classes
        .iter()
        .any(|(key, value)| key.is_empty() || value.is_empty())
    {
        bail!("device class mappings must have non-empty keys and values");
    }

    let mut output = storage
        .as_object()
        .context("Ceph storage must be an object")?
        .clone();
    require_false(&output, "useAllNodes")?;
    require_false(&output, "useAllDevices")?;
    reject_cluster_device_selectors(&output)?;

    let source_nodes = match output.get("nodes") {
        None => Vec::new(),
        Some(Value::Array(nodes)) => nodes.clone(),
        Some(_) => bail!("Ceph storage nodes must be an array"),
    };
    let mut existing_names = BTreeSet::new();
    let mut existing_by_node = BTreeMap::<String, BTreeSet<String>>::new();
    let mut persistent_claims = BTreeMap::<String, String>::new();

    for node in &source_nodes {
        let object = node
            .as_object()
            .context("existing Ceph node must be an object")?;
        let node_name = object
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .context("existing Ceph node name must be a string")?;
        if !existing_names.insert(node_name.to_owned()) {
            bail!("duplicate existing Ceph node name {node_name:?}");
        }
        if uses_broad_device_selector(object)? {
            bail!(
                "existing Ceph node {node_name:?} uses a device selector that \
                 conflicts with runtime-managed explicit devices"
            );
        }
        let mut identities = BTreeSet::new();
        match object.get("devices") {
            None => {}
            Some(Value::Array(devices)) => {
                for device in devices {
                    let device = device
                        .as_object()
                        .context("existing Ceph device must be an object")?;
                    let device_name = device
                        .get("name")
                        .and_then(Value::as_str)
                        .filter(|name| !name.is_empty())
                        .context("existing Ceph device name must be a non-empty string")?;
                    let identity = normalize_device_identity(device_name);
                    if !identities.insert(identity.clone()) {
                        bail!(
                            "existing Ceph node {node_name:?} contains duplicate \
                             device identity {identity:?}"
                        );
                    }
                    claim_persistent_identity(&mut persistent_claims, device_name, node_name)?;
                }
            }
            Some(_) => bail!("existing Ceph node devices must be an array"),
        }
        existing_by_node.insert(node_name.to_owned(), identities);
    }

    let mut observed_nodes = BTreeSet::new();
    for node in observed {
        if node.name.is_empty() || !observed_nodes.insert(&node.name) {
            bail!("observed identity claims have an invalid or duplicate node name");
        }
        for identity in &node.identities {
            claim_persistent_identity(&mut persistent_claims, identity, &node.name)?;
        }
    }

    validate_candidate_aliases(candidates)?;
    for node in candidates {
        for device in &node.devices {
            for alias in persistent_identities(device) {
                claim_persistent_identity(&mut persistent_claims, alias, &node.name)?;
            }
        }
    }

    let mut planned_nodes = source_nodes;
    let mut summary = PlanSummary::default();
    for node in candidates {
        if node.name.is_empty() {
            bail!("candidate node name must not be empty");
        }
        let mut additions = Vec::new();
        let existing = existing_by_node.entry(node.name.clone()).or_default();
        for device in &node.devices {
            validate_candidate(device)?;
            let aliases = candidate_identities(device);
            if aliases.iter().any(|alias| existing.contains(alias)) {
                summary.devices_skipped_existing += 1;
                continue;
            }
            let Some(device_class) = resolve_device_class(device, &config.device_classes) else {
                summary.devices_skipped_unmapped += 1;
                continue;
            };
            existing.extend(aliases);
            additions.push(json!({
                "name": device.path,
                "config": {
                    "osdsPerDevice": config.osds_per_device.to_string(),
                    "deviceClass": device_class,
                },
            }));
        }
        if additions.is_empty() {
            continue;
        }
        summary.devices_added += additions.len();

        if let Some(index) = planned_nodes
            .iter()
            .position(|value| value.get("name").and_then(Value::as_str) == Some(node.name.as_str()))
        {
            let object = planned_nodes[index]
                .as_object_mut()
                .expect("existing nodes were validated as objects");
            match object.get_mut("devices") {
                None => {
                    object.insert("devices".to_owned(), Value::Array(additions));
                }
                Some(Value::Array(devices)) => devices.extend(additions),
                Some(_) => unreachable!("existing devices were validated as arrays"),
            }
        } else {
            planned_nodes.push(json!({"name": node.name, "devices": additions}));
            summary.nodes_added += 1;
        }
    }

    output.insert("nodes".to_owned(), Value::Array(planned_nodes));
    Ok(Plan {
        storage: Value::Object(output),
        summary,
    })
}

fn require_false(storage: &Map<String, Value>, key: &str) -> Result<()> {
    match storage.get(key) {
        Some(Value::Bool(false)) => Ok(()),
        _ => bail!("Ceph storage {key} must be exactly false"),
    }
}

fn reject_cluster_device_selectors(storage: &Map<String, Value>) -> Result<()> {
    for key in ["deviceFilter", "devicePathFilter"] {
        match storage.get(key) {
            None => {}
            Some(Value::String(value)) if value.is_empty() => {}
            Some(Value::String(_)) => {
                bail!("Ceph storage {key} conflicts with runtime-managed explicit devices")
            }
            Some(_) => bail!("Ceph storage {key} must be a string"),
        }
    }
    match storage.get("devices") {
        None => Ok(()),
        Some(Value::Array(devices)) if devices.is_empty() => Ok(()),
        Some(Value::Array(_)) => {
            bail!("cluster-level Ceph devices conflict with runtime-managed explicit devices")
        }
        Some(_) => bail!("cluster-level Ceph devices must be an array"),
    }
}

fn uses_broad_device_selector(node: &Map<String, Value>) -> Result<bool> {
    for key in ["deviceFilter", "devicePathFilter"] {
        if node.contains_key(key) {
            return Ok(true);
        }
    }
    match node.get("useAllDevices") {
        None | Some(Value::Bool(false)) => Ok(false),
        Some(Value::Bool(true)) => Ok(true),
        Some(_) => bail!("existing node useAllDevices must be a boolean"),
    }
}

fn validate_candidate(device: &CandidateDevice) -> Result<()> {
    if !is_by_id_path(&device.path) {
        bail!("candidate path must be a non-empty /dev/disk/by-id path");
    }
    if device
        .aliases
        .iter()
        .any(|alias| alias.starts_with("/dev/disk/by-id/") && !is_by_id_path(alias))
    {
        bail!("candidate contains a malformed by-id alias");
    }
    if !device.aliases.iter().any(|alias| alias == &device.path) {
        bail!("candidate aliases must contain its selected path");
    }
    Ok(())
}

fn candidate_identities(device: &CandidateDevice) -> BTreeSet<String> {
    device
        .aliases
        .iter()
        .chain(std::iter::once(&device.path))
        .map(|alias| normalize_device_identity(alias))
        .collect()
}

pub fn normalize_device_identity(name: &str) -> String {
    if name.starts_with("/dev/") {
        name.to_owned()
    } else if !name.contains('/') {
        format!("/dev/{name}")
    } else {
        name.to_owned()
    }
}

fn persistent_identities(device: &CandidateDevice) -> impl Iterator<Item = &str> {
    device
        .aliases
        .iter()
        .chain(std::iter::once(&device.path))
        .map(String::as_str)
        .filter(|alias| is_by_id_path(alias))
}

fn claim_persistent_identity(
    claims: &mut BTreeMap<String, String>,
    identity: &str,
    node_name: &str,
) -> Result<()> {
    if !identity.starts_with("/dev/disk/by-id/") {
        return Ok(());
    }
    if !is_by_id_path(identity) {
        bail!("persistent device identity {identity:?} is malformed");
    }
    if let Some(existing_node) = claims.get(identity)
        && existing_node != node_name
    {
        bail!(
            "persistent device identity {identity:?} is claimed by both \
             {existing_node:?} and {node_name:?}"
        );
    }
    claims.insert(identity.to_owned(), node_name.to_owned());
    Ok(())
}

fn validate_candidate_aliases(candidates: &[NodeCandidates]) -> Result<()> {
    let mut node_names = BTreeSet::new();
    for node in candidates {
        if !node_names.insert(&node.name) {
            bail!("duplicate candidate node name {:?}", node.name);
        }
        let mut claimed = BTreeMap::<String, usize>::new();
        for (index, device) in node.devices.iter().enumerate() {
            validate_candidate(device)?;
            for alias in candidate_identities(device) {
                if let Some(previous) = claimed.insert(alias.clone(), index)
                    && previous != index
                {
                    bail!(
                        "candidate devices on node {:?} overlap at alias {alias:?}",
                        node.name
                    );
                }
            }
        }
    }
    Ok(())
}

fn resolve_device_class<'a>(
    device: &CandidateDevice,
    mappings: &'a BTreeMap<String, String>,
) -> Option<&'a str> {
    match device.hardware_class.as_deref() {
        Some(class) => mappings.get(class),
        None => mappings.get(&device.medium),
    }
    .map(String::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> PlannerConfig {
        PlannerConfig {
            osds_per_device: 1,
            device_classes: BTreeMap::from([
                ("nvme".to_owned(), "nvme-class".to_owned()),
                ("fast".to_owned(), "pcie-fast".to_owned()),
            ]),
        }
    }

    fn candidate(path: &str) -> CandidateDevice {
        CandidateDevice {
            path: path.to_owned(),
            aliases: vec![path.to_owned(), "/dev/nvme0n1".to_owned()],
            medium: "nvme".to_owned(),
            hardware_class: None,
        }
    }

    fn storage(nodes: Option<Value>) -> Value {
        let mut value = json!({"useAllNodes": false, "useAllDevices": false});
        if let Some(nodes) = nodes {
            value["nodes"] = nodes;
        }
        value
    }

    #[test]
    fn missing_nodes_are_created() {
        let plan = plan_storage(
            &storage(None),
            &[NodeCandidates {
                name: "node-a".into(),
                devices: vec![candidate("/dev/disk/by-id/a")],
            }],
            &config(),
        )
        .unwrap();
        assert_eq!(
            plan.storage["nodes"][0]["devices"][0]["name"],
            "/dev/disk/by-id/a"
        );
    }

    #[test]
    fn broad_selector_nodes_reject_the_plan() {
        for selector in [
            json!({"deviceFilter": "sd.*"}),
            json!({"devicePathFilter": "/dev/x"}),
            json!({"useAllDevices": true}),
        ] {
            let mut node = json!({"name": "node-a", "devices": []});
            node.as_object_mut()
                .unwrap()
                .extend(selector.as_object().unwrap().clone());
            assert!(
                plan_storage(
                    &storage(Some(json!([node]))),
                    &[NodeCandidates {
                        name: "node-a".into(),
                        devices: vec![candidate("/dev/disk/by-id/a")],
                    }],
                    &config(),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn cluster_selectors_reject_the_plan() {
        for (key, value) in [
            ("deviceFilter", json!("sd.*")),
            ("devicePathFilter", json!("/dev/disk/by-path/.*")),
            ("devices", json!([{"name":"/dev/disk/by-id/manual"}])),
        ] {
            let mut source = storage(None);
            source[key] = value;
            assert!(plan_storage(&source, &[], &config()).is_err());
        }
    }

    #[test]
    fn hardware_mapping_precedes_medium_without_raw_fallback() {
        let mut first = candidate("/dev/disk/by-id/a");
        first.hardware_class = Some("fast".into());
        let mut second = candidate("/dev/disk/by-id/b");
        second.aliases[1] = "/dev/nvme1n1".into();
        second.hardware_class = Some("unknown".into());
        let plan = plan_storage(
            &storage(None),
            &[NodeCandidates {
                name: "n".into(),
                devices: vec![first, second],
            }],
            &config(),
        )
        .unwrap();
        assert_eq!(
            plan.storage["nodes"][0]["devices"][0]["config"]["deviceClass"],
            "pcie-fast"
        );
        assert_eq!(
            plan.storage["nodes"][0]["devices"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert_eq!(plan.summary.devices_skipped_unmapped, 1);
    }

    #[test]
    fn bare_kernel_name_detects_duplicate() {
        let plan = plan_storage(
            &storage(Some(
                json!([{"name":"n","devices":[{"name":"nvme0n1","keep":true}]}]),
            )),
            &[NodeCandidates {
                name: "n".into(),
                devices: vec![candidate("/dev/disk/by-id/a")],
            }],
            &config(),
        )
        .unwrap();
        assert_eq!(plan.summary.devices_skipped_existing, 1);
        assert_eq!(
            plan.storage["nodes"][0]["devices"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn overlapping_candidates_are_rejected() {
        let mut other = candidate("/dev/disk/by-id/b");
        other.aliases.push("/dev/disk/by-id/a".into());
        assert!(
            plan_storage(
                &storage(None),
                &[NodeCandidates {
                    name: "n".into(),
                    devices: vec![candidate("/dev/disk/by-id/a"), other]
                }],
                &config()
            )
            .is_err()
        );
    }

    #[test]
    fn persistent_identity_cannot_cross_nodes() {
        let shared = "/dev/disk/by-id/wwn-shared";
        assert!(
            plan_storage(
                &storage(None),
                &[
                    NodeCandidates {
                        name: "node-a".into(),
                        devices: vec![candidate(shared)],
                    },
                    NodeCandidates {
                        name: "node-b".into(),
                        devices: vec![CandidateDevice {
                            path: shared.into(),
                            aliases: vec![shared.into(), "/dev/sdb".into()],
                            medium: "nvme".into(),
                            hardware_class: None,
                        }],
                    },
                ],
                &config(),
            )
            .is_err()
        );

        let existing = storage(Some(json!([{
            "name":"node-a",
            "devices":[{"name":shared}]
        }])));
        assert!(
            plan_storage(
                &existing,
                &[NodeCandidates {
                    name: "node-b".into(),
                    devices: vec![CandidateDevice {
                        path: shared.into(),
                        aliases: vec![shared.into(), "/dev/sdb".into()],
                        medium: "nvme".into(),
                        hardware_class: None,
                    }],
                }],
                &config(),
            )
            .is_err()
        );
        assert!(
            plan_storage_with_claims(
                &existing,
                &[],
                &[NodeIdentityClaims {
                    name: "node-b".into(),
                    identities: BTreeSet::from([shared.into()]),
                }],
                &config(),
            )
            .is_err()
        );
    }

    #[test]
    fn malformed_or_null_devices_are_rejected() {
        for devices in [Value::Null, json!([null]), json!([{}])] {
            assert!(
                plan_storage(
                    &storage(Some(json!([{"name":"n","devices":devices}]))),
                    &[],
                    &config()
                )
                .is_err()
            );
        }
        assert!(
            plan_storage(
                &storage(Some(json!([{
                    "name":"n",
                    "devices":[
                        {"name":"/dev/disk/by-id/a"},
                        {"name":"/dev/disk/by-id/a"}
                    ]
                }]))),
                &[],
                &config(),
            )
            .is_err()
        );
        assert!(
            plan_storage(
                &storage(Some(json!([{"name":"","devices":[]}]))),
                &[],
                &config(),
            )
            .is_err()
        );
    }

    #[test]
    fn existing_values_are_preserved_and_plan_is_idempotent() {
        let original = storage(Some(
            json!([{"name":"n","location":"rack=a","devices":[{"name":"/dev/old","config":{"deviceClass":"old"},"extra":[1,2]}]}]),
        ));
        let nodes = [NodeCandidates {
            name: "n".into(),
            devices: vec![candidate("/dev/disk/by-id/a")],
        }];
        let first = plan_storage(&original, &nodes, &config()).unwrap();
        assert_eq!(first.storage["nodes"][0]["location"], "rack=a");
        assert_eq!(
            first.storage["nodes"][0]["devices"][0],
            original["nodes"][0]["devices"][0]
        );
        let second = plan_storage(&first.storage, &nodes, &config()).unwrap();
        assert_eq!(second.storage, first.storage);
        assert_eq!(second.summary.devices_added, 0);
    }
}
