use openark_vine_oauth::User;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use url::Url;

use crate::file::FileShortcut;

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

/// A user's subscription information.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct UserSubscription {
    /// Whether the user's subscription is activated.
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_active: Option<bool>,

    /// The user's tier name.
    #[cfg_attr(
        feature = "serde",
        serde(default = "UserSubscription::default_tier_name")
    )]
    pub tier_name: String,

    /// The total available size of storage.
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_capacity: u64,

    /// The total used size of storage.
    #[cfg_attr(feature = "serde", serde(default))]
    pub total_used: u64,
}

impl UserSubscription {
    fn default_tier_name() -> String {
        "Free".into()
    }

    /// Returns the total usage per capacity as percent `%`.
    pub fn total_usage_percent(&self) -> f32 {
        if self.total_capacity > 0 {
            let used = self.total_used as f32;
            let capacity = self.total_capacity as f32;
            100.0 * (used / capacity).min(1.0)
        } else {
            0.0
        }
    }
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
    pub shortcuts: Vec<FileShortcut>,

    /// User subscription informantion.
    pub subscription: UserSubscription,

    /// A `JWT`.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub token: User,
}
