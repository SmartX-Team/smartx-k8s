use core::ops;
use std::string::String;

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{
    binding::SessionBindingUserSpec,
    profile::{
        FeaturesSpec, GreeterSpec, PersistenceSpec, RegionSpec, ServicesSpec, SessionSpec,
        UserSpec, VMSpec, VolumesSpec,
    },
};

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedSessionProfileSpec {
    #[cfg_attr(feature = "serde", serde(default))]
    pub auth: OwnedAuthSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub features: OwnedFeaturesSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub greeter: GreeterSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub ingress: OwnedIngressSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub node: OwnedNodeSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub openark: OwnedOpenArkSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub persistence: PersistenceSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub region: RegionSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub services: ServicesSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub session: SessionSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub user: OwnedUserSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub vm: OwnedVMSpec,

    #[cfg_attr(feature = "serde", serde(default))]
    pub volumes: VolumesSpec,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedAuthSpec {
    pub domain_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedFeaturesSpec {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub data: FeaturesSpec,

    pub gateway: bool,
    pub ingress: bool,
    pub vm: bool,
}

impl ops::Deref for OwnedFeaturesSpec {
    type Target = FeaturesSpec;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ops::DerefMut for OwnedFeaturesSpec {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedIngressSpec {
    pub domain_name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedNodeSpec {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub alias: Option<String>,

    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedOpenArkSpec {
    pub labels: OwnedOpenArkLabelsSpec,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedOpenArkLabelsSpec {
    #[cfg_attr(feature = "serde", serde(rename = "org.ulagbulag.io/bind"))]
    pub bind: String,

    #[cfg_attr(feature = "serde", serde(rename = "org.ulagbulag.io/bind.node"))]
    pub bind_node: String,

    #[cfg_attr(feature = "serde", serde(rename = "org.ulagbulag.io/bind.persistent"))]
    pub bind_persistent: String,

    #[cfg_attr(feature = "serde", serde(rename = "org.ulagbulag.io/bind.user"))]
    pub bind_user: String,

    #[cfg_attr(feature = "serde", serde(rename = "org.ulagbulag.io/is-private"))]
    pub is_private: String,

    #[cfg_attr(feature = "serde", serde(rename = "org.ulagbulag.io/signed-out"))]
    pub signed_out: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedUserSpec {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub binding: SessionBindingUserSpec,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub data: UserSpec,
}

impl ops::Deref for OwnedUserSpec {
    type Target = UserSpec;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ops::DerefMut for OwnedUserSpec {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedVMSpec {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub data: VMSpec,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub host_devices: Option<Vec<OwnedVMHostDeviceSpec>>,
}

impl ops::Deref for OwnedVMSpec {
    type Target = VMSpec;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl ops::DerefMut for OwnedVMSpec {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.data
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct OwnedVMHostDeviceSpec {
    pub api_group: String,
    pub kind: String,
    pub vendor: String,
    pub product: String,
}
