#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A file's available MIME-compatible audio types.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum AppType {
    #[cfg_attr(feature = "serde", serde(rename = "application/octet-stream"))]
    OctetStream,
    Other(String),
}

/// A file's available MIME-compatible audio types.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AudioType {
    #[cfg_attr(feature = "serde", serde(rename = "audio"))]
    #[default]
    Other,
}

/// A file's available MIME-compatible document types.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum DocumentType {
    #[cfg_attr(feature = "serde", serde(rename = "application/pdf"))]
    Pdf,
    #[cfg_attr(feature = "serde", serde(rename = "text"))]
    #[default]
    Other,
}

/// A file's available MIME-compatible image types.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum ImageType {
    #[cfg_attr(feature = "serde", serde(rename = "image/jpeg"))]
    Jpeg,
    #[cfg_attr(feature = "serde", serde(rename = "image"))]
    #[default]
    Other,
}

/// A file's available MIME-compatible video types.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VideoType {
    #[cfg_attr(feature = "serde", serde(rename = "video"))]
    #[default]
    Other,
}

/// A file's available MIME-compatible types.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(untagged))]
pub enum FileType {
    Audio(AudioType),
    Document(DocumentType),
    Image(ImageType),
    Video(VideoType),
    App(AppType),
}
