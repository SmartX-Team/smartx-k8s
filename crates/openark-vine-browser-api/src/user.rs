use openark_vine_oauth::User;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

use crate::file::FileRef;

/// A file's metadata.
///
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserMetadata {
    /// The user's optional initial name.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub initial: Option<String>,

    /// The user's optional thumbnail [`Url`].
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub thumbnail_url: Option<Url>,
}

/// A file's referral information.
///
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserRef {
    /// The user's unique ID.
    pub id: String,

    /// The user name.
    pub name: String,

    /// The user's metadata.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub metadata: UserMetadata,
}

/// A browser's global configuration.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserConfiguration {
    /// The current user's metadata.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub metadata: UserMetadata,

    /// Registered shortcuts.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub shortcuts: Vec<FileRef>,

    /// A `JWT`.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub token: User,
}
