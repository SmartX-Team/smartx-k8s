#[cfg(feature = "patch")]
#[cfg_attr(feature = "serde", derive(::serde::Serialize, ::serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "status"))]
pub enum AdmissionResult {
    Deny { message: String },
    Pass,
    Patch { operations: ::json_patch::Patch },
}
