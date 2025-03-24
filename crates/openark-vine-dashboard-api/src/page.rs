use core::fmt;
use std::str::FromStr;

#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use strum::{Display, EnumString};
use thiserror::Error;

use crate::table::TableSession;

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PageMetadata {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub object: PageRef,

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
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PageRef {
    pub kind: PageKind,
    pub namespace: String,
    pub name: String,
}

impl fmt::Display for PageRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            kind,
            namespace,
            name,
        } = self;
        write!(f, "{kind}/{namespace}/{name}")
    }
}

impl FromStr for PageRef {
    type Err = PageRefParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut iter = s.split('/');

        let kind = iter
            .next()
            .and_then(|kind| kind.parse().ok())
            .ok_or(PageRefParseError::InvalidPageAddress)?;
        let namespace = iter.next().ok_or(PageRefParseError::InvalidPageAddress)?;
        let name = iter.next().ok_or(PageRefParseError::InvalidPageAddress)?;

        match iter.next() {
            Some(_) => Err(PageRefParseError::InvalidPageAddress),
            None => Ok(Self {
                kind,
                namespace: namespace.into(),
                name: name.into(),
            }),
        }
    }
}

#[derive(Clone, Debug, Error)]
pub enum PageRefParseError {
    #[error("Invalid page address")]
    InvalidPageAddress,
}

#[derive(Copy, Clone, Debug, Display, EnumString, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PageKind {
    #[strum(serialize = "tables")]
    Table,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind"))]
pub enum PageSpec {
    Table(TableSession),
}
