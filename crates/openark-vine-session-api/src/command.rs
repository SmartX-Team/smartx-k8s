use std::{collections::BTreeMap, string::String};

#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SessionCommandView {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub alias: Option<String>,

    pub name: String,

    pub spec: SessionCommandSpec,
}

/// A struct storing user session.
/// A binding can apply to many sessions.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "kube", derive(CustomResource))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[cfg_attr(
    feature = "kube",
    kube(
        namespaced,
        category = "org",
        group = "org.ulagbulag.io",
        version = "v1alpha1",
        kind = "SessionCommand",
        root = "SessionCommandCrd",
        shortname = "sc",
        printcolumn = r#"{
            "name": "created-at",
            "type": "date",
            "description": "created time of the command",
            "jsonPath": ".metadata.creationTimestamp"
        }"#,
        printcolumn = r#"{
            "name": "version",
            "type": "integer",
            "priority": 1,
            "description": "command version",
            "jsonPath": ".metadata.generation"
        }"#
    )
)]
pub struct SessionCommandSpec {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub enabled: Option<bool>,

    pub command: Vec<String>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub features: SessionCommandFeaturesSpec,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub node_selector: Option<BTreeMap<String, String>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SessionCommandFeaturesSpec {
    #[cfg_attr(feature = "serde", serde(default))]
    pub linux: bool,

    #[cfg_attr(feature = "serde", serde(default))]
    pub windows: bool,
}
