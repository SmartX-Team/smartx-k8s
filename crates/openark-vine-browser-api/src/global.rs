#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

use crate::user::UserConfiguration;

/// A browser's global configuration.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GlobalConfigurationSpec {
    /// The browser's name.
    pub title: String,

    /// The browser's logo image `[Url]`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub logo_url: Option<Url>,

    /// The browser's redirect `[Url]`.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub redirect_url: Option<Url>,
}

/// A browser's global configuration.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct GlobalConfiguration {
    /// The browser's specification.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub spec: GlobalConfigurationSpec,

    /// The current user's configuration.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub user: Option<UserConfiguration>,
}
