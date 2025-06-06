use std::fmt;

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// ObjectReference is a reference to an Object.
///
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ObjectReference {
    /// Group is the group of the referent.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "String::is_empty")
    )]
    pub group: String,

    /// Kind is kind of the referent.
    pub kind: String,

    /// Name is the name of the referent.
    pub name: String,

    /// Namespace is the namespace of the referent.
    /// This field is required when referring to a Namespace-scoped resource and
    /// MUST be unset when referring to a Cluster-scoped resource.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub namespace: Option<String>,
}

impl fmt::Display for ObjectReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            group,
            kind,
            name,
            namespace,
        } = self;

        match (group.as_str(), namespace.as_deref()) {
            ("", None) => write!(f, "{kind}/{name}"),
            ("", Some(namespace)) => write!(f, "{kind}/{namespace}/{name}"),
            (group, None) => write!(f, "{group}/{kind}//{name}"),
            (group, Some(namespace)) => write!(f, "{group}/{kind}/{namespace}/{name}"),
        }
    }
}

/// ServiceReference is a reference to a Service.
///
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ServiceReference {
    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub object: ObjectReference,

    /// Port is the port of the referent.
    pub port: u16,
}
