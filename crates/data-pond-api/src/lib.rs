use std::fmt;

use data_pond_csi::pond;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[inline]
const fn default_device_layer() -> pond::device_layer::Type {
    pond::device_layer::Type::Lvm
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumeAttributes {
    #[cfg_attr(
        feature = "serde",
        serde(
            default = "default_device_layer",
            rename = "data-pond.csi.ulagbulag.io/layer"
        )
    )]
    pub layer: pond::device_layer::Type,

    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "csi.storage.k8s.io/pv/name",
            skip_serializing_if = "Option::is_none",
        )
    )]
    pub pv_name: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "csi.storage.k8s.io/pvc/name",
            skip_serializing_if = "Option::is_none",
        )
    )]
    pub pvc_name: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "csi.storage.k8s.io/pvc/namespace",
            skip_serializing_if = "Option::is_none",
        )
    )]
    pub pvc_namespace: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            rename = "org.ulagbulag.io/num-replicas",
            skip_serializing_if = "Option::is_none",
        )
    )]
    pub num_replicas: Option<String>,
}

#[derive(Clone, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumeSecrets {}

impl fmt::Debug for VolumeSecrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("VolumeSecrets").finish()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumeParameters {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub attributes: VolumeAttributes,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub secrets: VolumeSecrets,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize))]
pub struct VolumeAllocateContext<'a> {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub binding: &'a VolumeBindingContext,

    pub options: &'a pond::VolumeOptions,

    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: &'a VolumeParameters,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumeBindingContext {
    #[cfg_attr(feature = "serde", serde(default))]
    pub device: pond::Device,

    #[cfg_attr(feature = "serde", serde(default))]
    pub layer: pond::device_layer::Type,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub metadata: pond::VolumeBindingMetadata,

    #[cfg_attr(feature = "serde", serde(default))]
    pub source: pond::device_source::Type,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumePublishControllerContext {
    #[cfg_attr(feature = "serde", serde(default))]
    pub bindings: Vec<VolumeBindingContext>,
}

impl VolumePublishControllerContext {
    pub const LABEL_BINDINGS: &'static str = "data-pond.csi.ulagbulag.io/bindings";
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumePublishContext {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub controller: VolumePublishControllerContext,

    pub options: pond::VolumeOptions,

    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: VolumeParameters,

    pub read_only: bool,

    pub staging_target_path: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub target_path: Option<String>,

    pub volume_id: String,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct VolumeUnpublishContext {
    pub target_path: String,

    pub volume_id: String,
}
