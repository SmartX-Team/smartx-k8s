#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use strum::{Display, EnumString};
use url::Url;

#[derive(Copy, Clone, Debug, Display, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[strum(serialize_all = "PascalCase")]
pub enum ItemFieldKind {
    Integer,
    String,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ItemField {
    pub name: String,
    pub kind: ItemFieldKind,
    pub optional: bool,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub title: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub description: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "AnyValue::is_null")
    )]
    pub default: AnyValue,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub placeholder: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub max_length: Option<usize>,

    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "is_zero"))]
    pub min_length: usize,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "AnyValue::is_null")
    )]
    pub max_value: AnyValue,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "AnyValue::is_null")
    )]
    pub min_value: AnyValue,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ItemMetadataTemplate {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub fields: Vec<ItemField>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ItemMetadata {
    pub base_url: Url,

    #[cfg_attr(feature = "serde", serde(default, flatten))]
    pub template: ItemMetadataTemplate,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Item {
    pub metadata: ItemMetadata,
    pub spec: Option<Map<String, Value>>,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "schemars", schemars(transparent, extend("x-kubernetes-preserve-unknown-fields" = true)))]
#[cfg_attr(feature = "serde", serde(transparent))]
pub struct AnyValue(pub Value);

impl AnyValue {
    #[inline]
    fn is_null(&self) -> bool {
        self.0.is_null()
    }
}

const fn is_zero(value: &usize) -> bool {
    *value == 0
}
