use std::{collections::BTreeMap, string::String};

#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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
        kind = "SessionBinding",
        root = "SessionBindingCrd",
        shortname = "sb",
        printcolumn = r#"{
            "name": "created-at",
            "type": "date",
            "description": "created time of the binding",
            "jsonPath": ".metadata.creationTimestamp"
        }"#,
        printcolumn = r#"{
            "name": "version",
            "type": "integer",
            "priority": 1,
            "description": "binding version",
            "jsonPath": ".metadata.generation"
        }"#
    )
)]
pub struct SessionBindingSpec {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub enabled: Option<bool>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub node_selector: Option<BTreeMap<String, String>>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub priority: i32,

    pub profile: String,

    #[cfg_attr(feature = "serde", serde(default))]
    pub user: SessionBindingUserSpec,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct SessionBindingUserSpec {
    #[cfg_attr(feature = "serde", serde(default))]
    pub kind: SessionBindingUserKind,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub name: Option<String>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub privileged: Option<bool>,
}

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
pub enum SessionBindingUserKind {
    #[default]
    Guest,
    User,
}
