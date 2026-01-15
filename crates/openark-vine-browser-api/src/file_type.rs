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

impl AppType {
    /// Returns the `MIME` type.
    ///
    pub const fn mime_type(&self) -> &str {
        match self {
            Self::OctetStream => "application/octet-stream",
            Self::Other(ty) => ty.as_str(),
        }
    }
}

/// A file's available MIME-compatible audio types.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum AudioType {
    #[cfg_attr(feature = "serde", serde(rename = "audio/mpeg", alias = "audio/mpeg3"))]
    Mp3,
    #[cfg_attr(feature = "serde", serde(rename = "audio/ogg"))]
    Ogg,
    #[cfg_attr(feature = "serde", serde(rename = "audio"))]
    #[default]
    Other,
}

impl AudioType {
    /// Returns the `MIME` type.
    ///
    pub const fn mime_type(&self) -> &str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Ogg => "audio/ogg",
            Self::Other => "audio",
        }
    }
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

impl DocumentType {
    /// Returns the `MIME` type.
    ///
    pub const fn mime_type(&self) -> &str {
        match self {
            Self::Pdf => "application/pdf",
            Self::Other => "text",
        }
    }
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

impl ImageType {
    /// Returns the `MIME` type.
    ///
    pub const fn mime_type(&self) -> &str {
        match self {
            Self::Jpeg => "image/jpeg",
            Self::Other => "image",
        }
    }
}

/// A file's available MIME-compatible video types.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum VideoType {
    #[cfg_attr(
        feature = "serde",
        serde(rename = "video/mp4", alias = "application/mp4")
    )]
    Mp4,
    #[cfg_attr(feature = "serde", serde(rename = "video"))]
    #[default]
    Other,
}

impl VideoType {
    /// Returns the `MIME` type.
    ///
    pub const fn mime_type(&self) -> &str {
        match self {
            Self::Mp4 => "video/mp4",
            Self::Other => "video",
        }
    }
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

impl FileType {
    /// Returns the `MIME` type.
    ///
    pub const fn mime_type(&self) -> &str {
        match self {
            Self::Audio(ty) => ty.mime_type(),
            Self::Document(ty) => ty.mime_type(),
            Self::Image(ty) => ty.mime_type(),
            Self::Video(ty) => ty.mime_type(),
            Self::App(ty) => ty.mime_type(),
        }
    }
}
