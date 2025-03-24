#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use url::Url;

use crate::item::ItemMetadataTemplate;

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableSession {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub total_rows: Option<usize>,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub spec: TableSpec,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "kube", derive(CustomResource))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "kube",
    kube(
        namespaced,
        category = "org",
        group = "org.ulagbulag.io",
        version = "v1alpha1",
        kind = "Table",
        root = "TableCrd",
        printcolumn = r#"{
            "name": "alias",
            "type": "string",
            "description": "table alias",
            "jsonPath": ".metadata.annotations.dash\\.ulagbulag\\.io/alias"
        }"#,
        printcolumn = r#"{
            "name": "version",
            "type": "integer",
            "description": "table version",
            "jsonPath": ".metadata.generation"
        }"#
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableSpec {
    pub base_url: Url,

    #[cfg_attr(feature = "serde", serde(default))]
    pub services: TableServices,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub extra_services: Option<Vec<TableExtraService>>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub printer_columns: Option<Vec<TablePrinterColumn>>,

    #[cfg_attr(feature = "serde", serde(default))]
    pub schema: ItemMetadataTemplate,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableServices {
    #[cfg_attr(feature = "serde", serde(default))]
    pub create: TableService,

    #[cfg_attr(feature = "serde", serde(default))]
    pub delete: TableService,

    #[cfg_attr(feature = "serde", serde(default))]
    pub get: TableService,

    #[cfg_attr(feature = "serde", serde(default, rename = "list"))]
    pub get_collection: TableService,

    #[cfg_attr(feature = "serde", serde(default))]
    pub update: TableService,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableService {
    #[cfg_attr(feature = "serde", serde(default))]
    pub enabled: bool,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TableExtraService {
    pub name: String,
    pub kind: TableExtraServiceKind,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub json_path: Option<String>,

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

    #[cfg_attr(feature = "serde", serde(default))]
    pub single: bool,

    #[cfg_attr(feature = "serde", serde(default))]
    pub multiple: bool,

    #[cfg_attr(feature = "serde", serde(default))]
    pub side_effect: bool,
}

#[derive(Copy, Clone, Debug, Display, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[strum(serialize_all = "PascalCase")]
pub enum TableExtraServiceKind {
    Navigate,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TablePrinterColumn {
    pub name: String,
    pub kind: TablePrinterColumnKind,
    pub json_path: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub description: Option<String>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub prefixes: Option<Vec<TablePrinterColumnPrefix>>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub secondary: Option<TablePrinterColumnSecondary>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub tags: Option<TablePrinterColumnTags>,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TablePrinterColumnPrefix {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub name: Option<String>,
    pub kind: TablePrinterColumnKind,
    pub json_path: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TablePrinterColumnSecondary {
    pub json_path: String,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TablePrinterColumnTags {
    pub json_path: String,
}

#[derive(Copy, Clone, Debug, Display, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
#[strum(serialize_all = "PascalCase")]
pub enum TablePrinterColumnKind {
    ElapsedTime,
    ImageUrl,
    Level,
    String,
}
