#[cfg(feature = "clap")]
use clap::Parser;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "clap", derive(Parser))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct ExecArgs {
    /// Command to be executed
    #[cfg_attr(feature = "clap", arg(last = true))]
    pub command: Vec<String>,

    /// Target session pod label selector
    #[cfg_attr(feature = "clap", arg(long))]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub label_selector: Option<String>,

    /// Target session namespace
    #[cfg_attr(
        feature = "clap",
        arg(short = 'n', long, default_value = "vine-session")
    )]
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub namespace: Option<String>,

    /// Whether to excute within a GUI terminal.
    #[cfg_attr(feature = "clap", arg(short, long))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub terminal: bool,

    /// Whether to wait the attached processes.
    #[cfg_attr(feature = "clap", arg(short, long))]
    #[cfg_attr(feature = "serde", serde(default))]
    pub wait: bool,
}
