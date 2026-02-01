use std::net::IpAddr;

use jiff::{SignedDuration, Span, Timestamp};
#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use uuid::Uuid;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "kube", derive(CustomResource))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "kube",
    kube(
        category = "kiss",
        group = "kiss.ulagbulag.io",
        version = "v1alpha1",
        kind = "Box",
        root = "BoxCrd",
        status = "BoxStatus",
        shortname = "box",
        printcolumn = r#"{
        "name": "alias",
        "type": "string",
        "description": "box alias",
        "jsonPath": ".metadata.labels.dash\\.ulagbulag\\.io/alias"
    }"#,
        printcolumn = r#"{
        "name": "rack",
        "type": "string",
        "description": "rack name where the box is located",
        "jsonPath": ".spec.rack.name"
    }"#,
        printcolumn = r#"{
        "name": "address",
        "type": "string",
        "priority": 1,
        "description": "access address of the box",
        "jsonPath": ".status.access.primary.address"
    }"#,
        printcolumn = r#"{
        "name": "power",
        "type": "string",
        "priority": 1,
        "description": "power address of the box",
        "jsonPath": ".spec.power.address"
    }"#,
        printcolumn = r#"{
        "name": "cluster",
        "type": "string",
        "description": "cluster name where the box is located",
        "jsonPath": ".spec.group.clusterName"
    }"#,
        printcolumn = r#"{
        "name": "role",
        "type": "string",
        "description": "role of the box",
        "jsonPath": ".spec.group.role"
    }"#,
        printcolumn = r#"{
        "name": "state",
        "type": "string",
        "description": "state of the box",
        "jsonPath": ".status.state"
    }"#,
        printcolumn = r#"{
        "name": "created-at",
        "type": "date",
        "description": "created time of the box",
        "jsonPath": ".metadata.creationTimestamp"
    }"#,
        printcolumn = r#"{
        "name": "updated-at",
        "type": "date",
        "description": "updated time of the box",
        "jsonPath": ".status.lastUpdated"
    }"#,
        printcolumn = r#"{
        "name": "network-speed",
        "type": "string",
        "priority": 1,
        "description": "network interface link speed (Unit: Mbps)",
        "jsonPath": ".status.access.primary.speedMbps"
    }"#,
        printcolumn = r#"{
        "name": "version",
        "type": "integer",
        "priority": 1,
        "description": "box version",
        "jsonPath": ".metadata.generation"
    }"#
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxSpec {
    #[cfg_attr(feature = "serde", serde(default))]
    pub group: BoxGroupSpec,
    pub machine: BoxMachineSpec,
    #[cfg_attr(feature = "serde", serde(default))]
    pub power: Option<BoxPowerSpec>,
}

#[cfg(feature = "kube")]
impl BoxCrd {
    pub fn last_updated(&self) -> Option<Timestamp> {
        self.status
            .as_ref()
            .map(|status| status.last_updated)
            .or_else(|| self.metadata.creation_timestamp.as_ref().map(|time| time.0))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxStatus {
    #[cfg_attr(feature = "serde", serde(default))]
    pub state: BoxState,
    #[cfg_attr(feature = "serde", serde(default))]
    pub access: BoxAccessSpec,
    #[cfg_attr(feature = "serde", serde(default))]
    pub bind_group: Option<BoxGroupSpec>,
    pub last_updated: Timestamp,
}

#[derive(
    Copy, Clone, Debug, Display, Default, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BoxState {
    #[default]
    New,
    Commissioning,
    Ready,
    Joining,
    Running,
    GroupChanged,
    Failed,
    Disconnected,
}

impl BoxState {
    pub const fn as_task(&self) -> Option<&'static str> {
        match self {
            Self::New => None,
            Self::Commissioning => Some("commission"),
            Self::Ready => None,
            Self::Joining => Some("join"),
            Self::Running => Some("ping"),
            Self::GroupChanged | Self::Failed | Self::Disconnected => Some("reset"),
        }
    }

    pub const fn cron(&self) -> Option<&'static str> {
        match self {
            Self::Running => Some("@hourly"),
            Self::GroupChanged | Self::Failed | Self::Disconnected => Some("@hourly"),
            _ => None,
        }
    }

    pub const fn next(&self) -> Self {
        match self {
            Self::New => Self::Commissioning,
            Self::Commissioning => Self::Commissioning,
            Self::Ready => Self::Joining,
            Self::Joining => Self::Joining,
            Self::Running => Self::Running,
            Self::GroupChanged => Self::GroupChanged,
            Self::Failed => Self::Failed,
            Self::Disconnected => Self::Disconnected,
        }
    }

    pub fn timeout(&self) -> Option<Span> {
        let fallback_update = Span::new().hours(2);

        match self {
            Self::New => None,
            Self::Commissioning => Some(fallback_update),
            Self::Ready => None,
            Self::Joining => Some(fallback_update),
            Self::Running => None,
            Self::GroupChanged | Self::Failed | Self::Disconnected => None,
        }
    }

    pub fn timeout_new() -> SignedDuration {
        SignedDuration::new(30, 0)
    }

    pub const fn complete(&self) -> Option<Self> {
        match self {
            Self::New => None,
            Self::Commissioning => None,
            Self::Ready => None,
            Self::Joining => Some(Self::Running),
            Self::Running => None,
            Self::GroupChanged | Self::Failed | Self::Disconnected => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxAccessSpec<Interface = BoxAccessInterfaceSpec> {
    pub primary: Option<Interface>,
}

impl<T> Default for BoxAccessSpec<T> {
    fn default() -> Self {
        Self {
            primary: Default::default(),
        }
    }
}

impl BoxAccessSpec {
    pub fn management(&self) -> Option<&BoxAccessInterfaceSpec> {
        self.primary.as_ref()
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxAccessInterfaceSpec {
    pub address: IpAddr,
    // Speed (Mb/s)
    #[cfg_attr(feature = "serde", serde(default))]
    pub speed_mbps: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxGroupSpec {
    pub cluster_name: String,
    pub role: BoxGroupRole,
}

impl Default for BoxGroupSpec {
    fn default() -> Self {
        Self {
            cluster_name: Self::DEFAULT_CLUSTER_NAME.into(),
            role: BoxGroupRole::default(),
        }
    }
}

impl BoxGroupSpec {
    const DEFAULT_CLUSTER_NAME: &'static str = "default";

    pub fn is_default(&self) -> bool {
        self.cluster_name == Self::DEFAULT_CLUSTER_NAME
    }

    pub fn cluster_domain(&self) -> String {
        if self.is_default() {
            "openark".into()
        } else {
            format!("{}.openark", &self.cluster_name)
        }
    }
}

#[derive(
    Copy, Clone, Debug, Display, Default, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash,
)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BoxGroupRole {
    /*
        Control Plane
    */
    ControlPlane,
    /*
        Specialized Worker
    */
    Compute,
    Dashboard,
    Desktop,
    Gateway,
    Storage,
    /*
        Domain-specific Worker
    */
    Robot,
    /*
        General Worker
    */
    #[default]
    GenericWorker,
    ExternalWorker,
}

impl BoxGroupRole {
    pub const fn is_domain_specific(&self) -> bool {
        matches!(self, Self::Robot,)
    }

    pub const fn is_member(&self) -> bool {
        !matches!(self, Self::ExternalWorker,)
    }

    pub fn to_playbook(&self) -> String {
        format!(
            "playbook-{}.yaml",
            match self {
                Self::ControlPlane => "control_plane",
                _ => "worker",
            }
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxMachineSpec {
    pub uuid: Uuid,
}

impl BoxMachineSpec {
    pub fn hostname(&self) -> String {
        self.uuid.to_string()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct BoxPowerSpec {
    #[cfg_attr(feature = "serde", serde(default))]
    pub address: Option<IpAddr>,
    pub r#type: BoxPowerType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum BoxPowerType {
    IntelAMT,
    Ipmi,
}

pub mod request {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    #[cfg_attr(feature = "schemars", derive(JsonSchema))]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    pub struct BoxAccessInterfaceQuery {
        pub address: IpAddr,
        // Speed (Mb/s)
        pub speed_mbps: Option<String>,
    }

    impl TryFrom<BoxAccessInterfaceQuery> for BoxAccessInterfaceSpec {
        type Error = <u64 as ::core::str::FromStr>::Err;

        fn try_from(value: BoxAccessInterfaceQuery) -> Result<Self, Self::Error> {
            Ok(Self {
                address: value.address,
                speed_mbps: value.speed_mbps.map(|speed| speed.parse()).transpose()?,
            })
        }
    }

    impl TryFrom<BoxAccessSpec<BoxAccessInterfaceQuery>> for BoxAccessSpec<BoxAccessInterfaceSpec> {
        type Error = <u64 as ::core::str::FromStr>::Err;

        fn try_from(value: BoxAccessSpec<BoxAccessInterfaceQuery>) -> Result<Self, Self::Error> {
            Ok(Self {
                primary: value.primary.map(TryInto::try_into).transpose()?,
            })
        }
    }

    #[derive(Clone, Debug, PartialEq)]
    #[cfg_attr(feature = "schemars", derive(JsonSchema))]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
    pub struct BoxNewQuery {
        #[cfg_attr(feature = "serde", serde(flatten))]
        pub access_primary: BoxAccessInterfaceQuery,
        #[cfg_attr(feature = "serde", serde(flatten))]
        pub machine: BoxMachineSpec,
    }

    #[derive(Clone, Debug, PartialEq)]
    #[cfg_attr(feature = "schemars", derive(JsonSchema))]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    #[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
    pub struct BoxCommissionQuery {
        pub access: BoxAccessSpec<BoxAccessInterfaceQuery>,
        pub machine: BoxMachineSpec,
        pub power: Option<BoxPowerSpec>,
        pub reset: bool,
    }
}
