#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(not(feature = "std"))]
extern crate alloc as std;

pub mod binding;
pub mod owned_profile;
pub mod profile;
pub mod session;

use std::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use chrono::{DateTime, Duration, Utc};
#[cfg(feature = "clap")]
use clap::Parser;
#[cfg(feature = "serde")]
use k8s_openapi::Resource;
use k8s_openapi::{
    api::core::v1::{Node, Taint},
    apimachinery::pkg::{
        api::resource::Quantity,
        apis::meta::v1::{ObjectMeta, Time},
    },
};
use kube::ResourceExt;
use kube_quantity::ParsedQuantity;
use regex::Regex;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "serde")]
use serde_json::{Value, json};
use strum::{Display, EnumString};
use url::Url;

#[cfg(feature = "serde")]
use crate::{
    binding::{SessionBindingCrd, SessionBindingUserKind},
    profile::SessionProfileCrd,
};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct VineSessionArgs {
    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_AUTH_DOMAIN_NAME"))]
    auth_domain_name: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_FEATURE_VM"))]
    feature_vm: bool,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_INGRESS_DOMAIN_NAME"))]
    ingress_domain_name: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_ALIAS"))]
    label_alias: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND"))]
    label_bind: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_CPU"))]
    label_bind_cpu: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_MEMORY"))]
    label_bind_memory: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_NAMESPACE"))]
    label_bind_namespace: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_NODE"))]
    label_bind_node: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_PERSISTENT"))]
    label_bind_persistent: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_PROFILE"))]
    label_bind_profile: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_REVISION"))]
    label_bind_revision: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_STORAGE"))]
    label_bind_storage: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_TIMESTAMP"))]
    label_bind_timestamp: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_BIND_USER"))]
    label_bind_user: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_COMPUTE_MODE"))]
    label_compute_mode: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_IS_PRIVATE"))]
    label_is_private: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_LABEL_SIGNED_OUT"))]
    label_signed_out: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_SOURCE_PATH"))]
    source_path: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_SOURCE_REPO_REVISION"))]
    source_repo_revision: String,

    #[cfg_attr(feature = "clap", arg(long, env = "OPENARK_SOURCE_REPO_URL"))]
    source_repo_url: Url,
}

impl VineSessionArgs {
    /// Return `true` if VM feature is enabled.
    ///
    #[must_use]
    pub const fn feature_vm(&self) -> bool {
        self.feature_vm
    }

    /// Return a SmartX source path.
    ///
    #[must_use]
    pub fn source_path(&self) -> &str {
        &self.source_path
    }

    /// Return a SmartX source repository revision.
    ///
    #[must_use]
    pub fn source_repo_revision(&self) -> &str {
        &self.source_repo_revision
    }

    /// Return a SmartX source repository URL.
    ///
    #[must_use]
    pub fn source_repo_url(&self) -> &Url {
        &self.source_repo_url
    }

    /// Convert to an owned node spec.
    ///
    #[must_use]
    pub fn to_node_spec(&self, node: &Node) -> crate::owned_profile::OwnedNodeSpec {
        crate::owned_profile::OwnedNodeSpec {
            alias: Some(
                node.labels()
                    .get(&self.label_alias)
                    .filter(|&alias| !alias.trim().is_empty())
                    .cloned()
                    .unwrap_or_else(|| node.name_any()),
            ),
            name: node.name_any(),
        }
    }

    /// Convert to an OpenARK auth object.
    ///
    #[must_use]
    pub fn to_openark_auth_spec(&self) -> crate::owned_profile::OwnedAuthSpec {
        crate::owned_profile::OwnedAuthSpec {
            domain_name: self.auth_domain_name.clone(),
        }
    }

    /// Convert to an OpenARK ingress object.
    ///
    #[must_use]
    pub fn to_openark_ingress_spec(&self) -> crate::owned_profile::OwnedIngressSpec {
        crate::owned_profile::OwnedIngressSpec {
            domain_name: self.ingress_domain_name.clone(),
        }
    }

    /// Convert to an OpenARK labels map.
    ///
    #[must_use]
    pub fn to_openark_labels(&self) -> crate::owned_profile::OwnedOpenArkLabelsSpec {
        crate::owned_profile::OwnedOpenArkLabelsSpec {
            bind: self.label_bind.clone(),
            bind_node: self.label_bind_node.clone(),
            bind_persistent: self.label_bind_persistent.clone(),
            bind_user: self.label_bind_user.clone(),
            is_private: self.label_is_private.clone(),
            signed_out: self.label_signed_out.clone(),
        }
    }
}

/// An enumeration of available computing modes.
///
#[derive(Copy, Clone, Debug, Display, Default, EnumString, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "kebab-case"))]
#[strum(serialize_all = "kebab-case")]
pub enum ComputeMode {
    Container,
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "vm"))]
    #[strum(serialize = "vm")]
    VM,
}

impl ComputeMode {
    #[cfg(feature = "serde")]
    #[must_use]
    const fn as_nvidia_gpu_workload_config(&self) -> &str {
        match self {
            Self::Container => "container",
            Self::VM => "vm-passthrough",
        }
    }
}

/// A struct storing an session metadata.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Metadata {
    alias: Option<String>,
    bind: Option<bool>,
    bind_cpu: Option<Quantity>,
    bind_memory: Option<Quantity>,
    bind_namespace: Option<String>,
    bind_node: Option<String>,
    bind_persistent: Option<bool>,
    bind_profile: Option<String>,
    bind_revision: Option<String>,
    bind_storage: Option<Quantity>,
    bind_timestamp: Option<Time>,
    bind_user: Option<String>,
    compute_mode: Option<ComputeMode>,
    name: Option<String>,
    signed_out: Option<bool>,
}

impl Metadata {
    /// Load session metadata from kubernetes object metadata.
    ///
    pub fn load(args: &VineSessionArgs, metadata: &ObjectMeta) -> Self {
        let labels = metadata.labels.as_ref();
        Self {
            alias: labels
                .and_then(|map| map.get(&args.label_alias))
                .filter(|&value| !value.is_empty())
                .cloned(),
            bind: labels
                .and_then(|map| map.get(&args.label_bind))
                .and_then(|value| value.parse().ok()),
            bind_cpu: labels
                .and_then(|map| map.get(&args.label_bind_cpu))
                .filter(|&value| !value.is_empty())
                .cloned()
                .map(Quantity),
            bind_memory: labels
                .and_then(|map| map.get(&args.label_bind_memory))
                .filter(|&value| !value.is_empty())
                .cloned()
                .map(Quantity),
            bind_namespace: labels
                .and_then(|map| map.get(&args.label_bind_namespace))
                .filter(|&value| !value.is_empty())
                .cloned(),
            bind_node: labels
                .and_then(|map| map.get(&args.label_bind_node))
                .filter(|&value| !value.is_empty())
                .cloned(),
            bind_persistent: labels
                .and_then(|map| map.get(&args.label_bind_persistent))
                .and_then(|value| value.parse().ok()),
            bind_profile: labels
                .and_then(|map| map.get(&args.label_bind_profile))
                .filter(|&value| !value.is_empty())
                .cloned(),
            bind_revision: labels
                .and_then(|map| map.get(&args.label_bind_revision))
                .filter(|&value| !value.is_empty())
                .cloned(),
            bind_storage: labels
                .and_then(|map| map.get(&args.label_bind_storage))
                .filter(|&value| !value.is_empty())
                .cloned()
                .map(Quantity),
            bind_timestamp: labels
                .and_then(|map| map.get(&args.label_bind_timestamp))
                .and_then(|value| value.parse::<i64>().ok())
                .and_then(DateTime::from_timestamp_millis)
                .map(Time),
            bind_user: labels
                .and_then(|map| map.get(&args.label_bind_user))
                .filter(|&value| !value.is_empty())
                .cloned(),
            compute_mode: labels
                .and_then(|map| map.get(&args.label_compute_mode))
                .and_then(|value| value.parse().ok()),
            name: metadata.name.clone(),
            signed_out: labels
                .and_then(|map| map.get(&args.label_signed_out))
                .and_then(|value| value.parse().ok()),
        }
    }

    #[must_use]
    fn build_labels(&self, args: &VineSessionArgs) -> BTreeMap<String, String> {
        fn to_patch_resource(res: &Option<Quantity>) -> String {
            res.as_ref().map(|res| res.0.clone()).unwrap_or_default()
        }

        let mut map = BTreeMap::default();
        map.insert(
            args.label_alias.clone(),
            self.alias.clone().unwrap_or_default(),
        );
        map.insert(
            args.label_bind.clone(),
            self.bind.unwrap_or(false).to_string(),
        );
        map.insert(
            args.label_bind_cpu.clone(),
            to_patch_resource(&self.bind_cpu),
        );
        map.insert(
            args.label_bind_memory.clone(),
            to_patch_resource(&self.bind_memory),
        );
        map.insert(
            args.label_bind_namespace.clone(),
            self.bind_namespace.clone().unwrap_or_default(),
        );
        map.insert(
            args.label_bind_node.clone(),
            self.bind_node.clone().unwrap_or_default(),
        );
        map.insert(
            args.label_bind_persistent.clone(),
            self.bind_persistent.unwrap_or(false).to_string(),
        );
        map.insert(
            args.label_bind_profile.clone(),
            self.bind_profile.clone().unwrap_or_default(),
        );
        map.insert(
            args.label_bind_revision.clone(),
            self.bind_revision.clone().unwrap_or_default(),
        );
        map.insert(
            args.label_bind_storage.clone(),
            to_patch_resource(&self.bind_storage),
        );
        map.insert(
            args.label_bind_timestamp.clone(),
            self.bind_timestamp
                .as_ref()
                .map(|time| time.0.timestamp_millis().to_string())
                .unwrap_or_default(),
        );
        map.insert(
            args.label_bind_user.clone(),
            self.bind_user.clone().unwrap_or_default(),
        );
        map.insert(
            args.label_compute_mode.clone(),
            self.compute_mode.unwrap_or_default().to_string(),
        );
        map.insert(
            args.label_signed_out.clone(),
            self.signed_out.unwrap_or(true).to_string(),
        );
        map.insert("nvidia.com/gpu.replicas".into(), "256".into());
        map.insert("nvidia.com/gpu.sharing-strategy".into(), "256".into());
        map.insert(
            "nvidia.com/gpu.workload.config".into(),
            self.compute_mode
                .unwrap_or_default()
                .as_nvidia_gpu_workload_config()
                .into(),
        );
        map
    }

    #[cfg(feature = "serde")]
    #[must_use]
    fn to_patch(&self, args: &VineSessionArgs) -> Value {
        json!({
            "labels": self.build_labels(args),
            "name": &self.name,
        })
    }
}

/// A struct storing a node's session state.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct NodeSession {
    metadata: Metadata,
    taints: Vec<Taint>,
    unreachable: bool,
}

impl NodeSession {
    /// Duration for signing out nodes.
    pub const DURATION_SIGN_OUT: Duration = Duration::seconds(Self::DURATION_SIGN_OUT_SECONDS as _);

    /// Duration for signing out nodes.
    pub const DURATION_SIGN_OUT_STD: ::core::time::Duration =
        ::core::time::Duration::from_secs(Self::DURATION_SIGN_OUT_SECONDS as _);

    /// Duration for signing out nodes as seconds.
    pub const DURATION_SIGN_OUT_SECONDS: u32 = 35;

    /// Load node state from kubernetes object.
    ///
    pub fn load(args: &VineSessionArgs, node: &Node) -> Self {
        let metadata = &node.metadata;
        let spec = node.spec.as_ref();
        Self {
            metadata: Metadata::load(args, metadata),
            taints: spec
                .and_then(|spec| spec.taints.clone())
                .unwrap_or_default(),
            unreachable: spec
                .and_then(|spec| spec.taints.as_ref())
                .map(|taints| {
                    taints
                        .iter()
                        .any(|taint| filter_taint(taint, "node.kubernetes.io/unreachable"))
                })
                // By default, node is reachable
                .unwrap_or(false),
        }
    }

    /// Get the active profile's name.
    #[must_use]
    pub fn get_profile(&self) -> Option<&str> {
        self.metadata.bind_profile.as_deref()
    }

    /// Get the active user's name.
    #[must_use]
    pub fn get_user(&self) -> Option<&str> {
        self.metadata
            .bind_user
            .as_deref()
            .filter(|&s| !s.is_empty())
    }

    /// Get remaining signing-out duration.
    #[must_use]
    pub fn signing_out(
        &self,
        args: &VineSessionArgs,
        timestamp: DateTime<Utc>,
    ) -> Option<Duration> {
        let time_added = self
            .taints
            .iter()
            .find(|&taint| filter_taint(taint, &args.label_signed_out))?
            .time_added
            .as_ref()?
            .0;

        let time_completed = time_added + Self::DURATION_SIGN_OUT;
        if time_completed > timestamp {
            Some(time_completed - timestamp)
        } else {
            None
        }
    }

    /// Return `true` if the node has been signed out.
    #[must_use]
    pub fn signed_out(&self) -> bool {
        self.metadata.signed_out.unwrap_or(true)
    }

    /// Get whether the node is unreachable.
    #[must_use]
    pub const fn unreachable(&self) -> bool {
        self.unreachable
    }

    /// Append node labels.
    ///
    #[must_use]
    pub fn append_labels(
        &self,
        args: &VineSessionArgs,
        mut labels: BTreeMap<String, String>,
    ) -> BTreeMap<String, String> {
        for (key, value) in self.metadata.build_labels(args) {
            labels.insert(key, value);
        }
        labels
    }

    /// Set the node's name.
    ///
    pub fn apply_node(&mut self, node: &Node) {
        if let Some(allocatable) = node
            .status
            .as_ref()
            .and_then(|status| status.allocatable.as_ref())
        {
            // Bind CPU
            if let Some(cpu_cores) = allocatable
                .get("cpu")
                .and_then(|q| ParsedQuantity::try_from(q).ok())
                .and_then(|q| q.to_bytes_u32())
            {
                // Subtract required cores
                let cpu_cores = cpu_cores.checked_sub(3).unwrap_or_default(); // hypervisor + 
                self.metadata.bind_cpu = Some(Quantity(cpu_cores.to_string()));
            }

            // Bind Memory
            if let Some(memory) = allocatable
                .get("memory")
                .and_then(|q| ParsedQuantity::try_from(q).ok())
                .and_then(|q| q.to_bytes_u128())
            {
                // Subtract required memory
                let memory = memory.checked_sub(7 << 30 /* 7 Gi */).unwrap_or_default();
                self.metadata.bind_memory = Some(Quantity(memory.to_string()));
            }

            // Bind Storage
            if let Some(storage) = allocatable
                .get("ephemeral-storage")
                .and_then(|q| ParsedQuantity::try_from(q).ok())
                .and_then(|q| q.to_bytes_f64())
            {
                // Subtract required storage size
                let storage = (storage * 0.8 * 0.5).floor(); // both Container & VM
                let storage = if storage.is_finite() && storage.is_sign_positive() {
                    storage as u128
                } else {
                    0
                };
                self.metadata.bind_storage = Some(Quantity(storage.to_string()));
            }
        }
        self.metadata.bind_node = Some(node.name_any());
    }

    /// Set the profile.
    /// Return `true` if the profile has been changed.
    ///
    #[cfg(feature = "kube")]
    pub fn apply_profile<'a>(
        &mut self,
        profile: Option<&'a (SessionBindingCrd, SessionProfileCrd)>,
        timestamp: DateTime<Utc>,
    ) -> ProfileState<'a> {
        match profile {
            Some((binding, profile)) => {
                let next_revision = &profile.metadata.resource_version;
                let state = match self.metadata.bind_profile.as_deref() {
                    Some(last_profile) => {
                        if last_profile == binding.spec.profile.as_str()
                            && self.metadata.bind_revision == *next_revision
                        {
                            ProfileState::Unchanged { binding, profile }
                        } else {
                            self.metadata.bind_timestamp = None;
                            ProfileState::Changed(last_profile.into())
                        }
                    }
                    None => {
                        self.metadata.bind_timestamp = None;
                        ProfileState::Created { binding, profile }
                    }
                };

                self.metadata.bind = Some(true);
                self.metadata.bind_namespace = None;
                self.metadata.bind_persistent = Some(
                    profile
                        .spec
                        .persistence
                        .as_ref()
                        .and_then(|p| p.enabled)
                        .unwrap_or(false),
                );
                self.metadata.bind_profile = Some(binding.spec.profile.clone());
                self.metadata.bind_revision = next_revision.clone();
                self.metadata.bind_timestamp.get_or_insert(Time(timestamp));
                self.metadata.bind_user = match binding.spec.user.kind {
                    SessionBindingUserKind::Guest => None,
                    SessionBindingUserKind::User => binding.spec.user.name.clone(),
                };
                self.metadata.compute_mode =
                    Some(match profile.spec.vm.as_ref().and_then(|vm| vm.enabled) {
                        Some(true) => ComputeMode::VM,
                        Some(false) | None => ComputeMode::Container,
                    });
                state
            }
            None => {
                self.metadata.bind = Some(false);
                self.metadata.bind_namespace = None;
                self.metadata.bind_persistent = Some(false);
                let last_profile = self.metadata.bind_profile.take();
                self.metadata.bind_revision = None;
                self.metadata.bind_timestamp = None;
                self.metadata.bind_user = None;
                ProfileState::Deleted(last_profile)
            }
        }
    }

    /// Return the session revision to ensure future updates.
    ///
    pub fn remove_session_revision(&mut self) {
        self.metadata.bind_revision = None;
    }

    /// Set the session's signing state.
    ///
    #[must_use]
    pub fn set_sign_out(
        &mut self,
        args: &VineSessionArgs,
        timestamp: DateTime<Utc>,
        sign_out: bool,
    ) -> Option<Duration> {
        if sign_out {
            self.metadata.bind = Some(false);
            self.metadata.bind_revision = None;
            self.metadata.bind_namespace = None;
            self.metadata.bind_persistent = None;
            self.metadata.bind_profile = None;
            self.metadata.bind_timestamp = None;
            self.metadata.bind_user = None;
            self.metadata.compute_mode = None;
        } else {
            self.unreachable = false;
        }
        self.metadata.signed_out = Some(sign_out);

        // Update taints
        {
            let last_taint = self
                .taints
                .iter_mut()
                .find(|taint| taint.key == args.label_signed_out && taint.effect == "NoExecute");

            if sign_out {
                match last_taint {
                    Some(taint) => {
                        taint.value = Some(true.to_string());
                        match taint.time_added.as_ref().map(|time| time.0) {
                            Some(time_added) => {
                                let time_completed = time_added + Self::DURATION_SIGN_OUT;
                                if time_completed > timestamp {
                                    Some(time_completed - timestamp)
                                } else {
                                    None
                                }
                            }
                            None => None,
                        }
                    }
                    None => {
                        let time_added = Some(Time(timestamp));
                        self.taints.push(Taint {
                            key: args.label_signed_out.clone(),
                            value: Some(true.to_string()),
                            effect: "NoExecute".into(),
                            time_added: time_added.clone(),
                        });
                        Some(Self::DURATION_SIGN_OUT)
                    }
                }
            } else {
                // NOTE: untainting should be handled by handler daemonset
                None
            }
        }
    }

    /// Convert to a server-side patch.
    ///
    #[cfg(feature = "serde")]
    #[must_use]
    pub fn to_patch(&self, args: &VineSessionArgs) -> Value {
        json!({
            "apiVersion": Node::API_VERSION,
            "kind": Node::KIND,
            "metadata": self.metadata.to_patch(args),
            "spec": {
                "taints": &self.taints,
                "unschedulable": self.unreachable,
            },
        })
    }

    /// Convert to a computing resource limits.
    ///
    #[must_use]
    pub fn to_resources_compute(&self, node: &Node) -> BTreeMap<String, Quantity> {
        let mut map = BTreeMap::default();
        if let Some(value) = self.metadata.bind_cpu.clone() {
            map.insert("cpu".into(), value);
        }
        if let Some(value) = self.metadata.bind_memory.clone() {
            map.insert("memory".into(), value);
        }

        // Attach NVIDIA GPU
        let labels = node.labels();
        if labels
            .get("nvidia.com/gpu.present")
            .and_then(|value| value.parse().ok())
            .unwrap_or(false)
        {
            match self.metadata.compute_mode {
                Some(ComputeMode::Container) => {
                    map.insert("nvidia.com/gpu".into(), Quantity("1".into()));
                }
                Some(ComputeMode::VM) => {
                    if let Some(allocatable) = node
                        .status
                        .as_ref()
                        .and_then(|status| status.allocatable.as_ref())
                    {
                        let re = Regex::new(r"^nvidia\.com/[A-Z0-9_]+$").unwrap();
                        let devices = allocatable
                            .iter()
                            .filter(|(key, _)| re.is_match(key) && !key.ends_with("_Audio"));

                        for (key, _value) in devices {
                            map.insert(key.clone(), Quantity("1".into()));
                        }
                    }
                }
                None => (),
            }
        }
        map
    }

    /// Convert to a local storage resource capacity.
    ///
    #[must_use]
    pub fn to_resources_local_storage(&self) -> BTreeMap<String, Quantity> {
        let mut map = BTreeMap::default();
        if let Some(value) = self.metadata.bind_storage.clone() {
            map.insert("storage".into(), value);
        }
        map
    }
}

/// An enumeration of available profile states.
///
#[cfg(feature = "kube")]
#[derive(Clone, Debug)]
#[must_use]
pub enum ProfileState<'a> {
    Changed(String),
    Created {
        binding: &'a SessionBindingCrd,
        profile: &'a SessionProfileCrd,
    },
    Deleted(Option<String>),
    Unchanged {
        binding: &'a SessionBindingCrd,
        profile: &'a SessionProfileCrd,
    },
}

#[cfg(feature = "kube")]
impl ProfileState<'_> {
    /// Return `true` if the state has been changed (**NOT** created).
    #[must_use]
    pub const fn has_changed(&self) -> bool {
        matches!(self, Self::Changed(_) | Self::Deleted(_))
    }
}

/// Return `true` if the taint is a kind of `key:_=NoExecute`
#[must_use]
pub fn filter_taint(taint: &Taint, key: &str) -> bool {
    taint.key == key && taint.effect == "NoExecute"
}
