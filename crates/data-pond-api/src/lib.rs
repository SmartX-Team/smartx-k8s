#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct VolumeAttributes {
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

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct VolumeParameters {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub attributes: VolumeAttributes,
}
