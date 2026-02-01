use jiff::Timestamp;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{file_type::FileType, user::UserRef};

/// A file's observed timestamp.
///
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FileTimestamp {
    /// The instigator of the action.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub by: Option<UserRef>,

    /// The observed timestamp.
    #[cfg_attr(feature = "serde", serde(rename = "at"))]
    pub timestamp: Timestamp,
}

/// A file's primary metadata.
///
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FileMetadata {
    /// The file's accessed timestamp.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub accessed: Option<FileTimestamp>,

    /// The file's created timestamp.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub created: Option<FileTimestamp>,

    /// The file's modified timestamp.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub modified: Option<FileTimestamp>,

    /// The file size.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub size: Option<u64>,

    /// The file's owner.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub owner: Option<UserRef>,
}

impl FileRef {
    /// Returns the file's MIME-compatible type.
    pub fn ty(&self) -> Option<FileType> {
        let ext = self.name.split('.').skip(1).last()?;
        FileType::from_known_extensions(ext)
    }
}

/// A file's referral information without `namespace`.
///
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FileRef {
    /// The file name.
    pub name: String,

    /// The file's absolute path.
    pub path: String,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub metadata: FileMetadata,
}

impl FileRef {
    /// Returns `true` if the file is a directory.
    ///
    #[inline]
    pub fn is_dir(&self) -> bool {
        self.path.ends_with('/')
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "PascalCase"))]
pub enum FileShortcutKind {
    Favorites,
    Home,
    TimeTravel,
    Trash,
}

/// A file entry shortcut.
///
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FileShortcut {
    /// The referral information.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub r: FileRef,

    /// An optional [`FileShortcutKind`].
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub kind: Option<FileShortcutKind>,
}

/// A directory.
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct FileEntry {
    /// The referral information.
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub r: FileRef,

    /// The inner contents.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub files: Vec<FileRef>,
}
