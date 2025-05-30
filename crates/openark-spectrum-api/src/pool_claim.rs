use std::collections::BTreeMap;

use chrono::DateTime;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[cfg(feature = "opeartor")]
impl ::openark_core::operator::Resource for PoolClaimCrd {
    type Status = PoolClaimStatus;

    fn conditions(&self) -> Option<&[Condition]> {
        self.status
            .as_ref()
            .map(|status| status.conditions.as_slice())
    }

    #[inline]
    fn build_status(
        &self,
        conditions: Vec<Condition>,
    ) -> <Self as ::openark_core::operator::Resource>::Status {
        PoolClaimStatus { conditions }
    }
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
        kind = "PoolClaim",
        root = "PoolClaimCrd",
        status = "PoolClaimStatus",
        printcolumn = r#"{
            "name": "pool",
            "type": "string",
            "jsonPath": ".spec.poolName"
        }"#,
        printcolumn = r#"{
            "name": "accepted",
            "type": "string",
            "jsonPath": ".status.conditions[?(@.type==\"Accepted\")].status"
        }"#,
        printcolumn = r#"{
            "name": "age",
            "type": "date",
            "jsonPath": ".metadata.creationTimestamp"
        }"#,
        printcolumn = r#"{
            "name": "version",
            "type": "integer",
            "priority": 1,
            "description": "claim version",
            "jsonPath": ".metadata.generation"
        }"#,
        selectable = ".spec.poolName",
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolClaimSpec {
    /// poolName is the name of the pool that is managing
    /// resources of this claim.
    pub pool_name: String,

    #[cfg_attr(feature = "serde", serde(default))]
    pub lifecycle: PoolResourceLifecycle,

    #[cfg_attr(feature = "serde", serde(default))]
    pub resources: PoolResourceSettings,
}

impl PoolClaimSpec {
    pub const FIELD_POOL_NAME: &'static str = "poolName";
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolResourceSettings {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub penalty: Option<f64>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub priority: Option<i32>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub max: Option<f64>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub min: Option<f64>,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub weight: Option<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct PoolResourceLifecycle {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Vec::is_empty")
    )]
    pub pre_start: Vec<PoolResourceProbe>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub enum PoolResourceProbe {
    Http(PoolResourceHttpProbe),
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct PoolResourceHttpProbe {
    pub method: PoolResourceHttpMethod,

    #[cfg_attr(feature = "serde", serde(default))]
    pub path: String,

    pub port: u16,

    #[cfg_attr(feature = "serde", serde(default))]
    pub secure: bool,

    #[cfg_attr(feature = "serde", serde(flatten))]
    pub body: Option<PoolResourceHttpBody>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub enum PoolResourceHttpMethod {
    DELETE,
    GET,
    PATCH,
    POST,
}

#[cfg(feature = "client")]
impl From<PoolResourceHttpMethod> for ::http::Method {
    fn from(value: PoolResourceHttpMethod) -> Self {
        match value {
            PoolResourceHttpMethod::DELETE => Self::DELETE,
            PoolResourceHttpMethod::GET => Self::GET,
            PoolResourceHttpMethod::PATCH => Self::PATCH,
            PoolResourceHttpMethod::POST => Self::POST,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub enum PoolResourceHttpBody {
    JsonBody(BTreeMap<String, Value>),
}

/// Status defines the current state of PoolClaim.
///
/// Implementations MUST populate status on all PoolClaim
/// resources which specify their controller name.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolClaimStatus {
    /// Conditions is the current status from the controller for
    /// this PoolClaim.
    ///
    /// Controllers should prefer to publish conditions using values
    /// of PoolClaimConditionType for the type of each Condition.
    #[serde(default = "PoolClaimStatus::default_conditions")]
    pub conditions: Vec<Condition>,
}

impl Default for PoolClaimStatus {
    fn default() -> Self {
        Self {
            conditions: Self::default_conditions(),
        }
    }
}

impl PoolClaimStatus {
    fn default_conditions() -> Vec<Condition> {
        vec![Condition {
            last_transition_time: Time(DateTime::default()),
            message: "Waiting for class".into(),
            observed_generation: None,
            reason: "Pending".into(),
            status: "Unknown".into(),
            type_: "Accepted".into(),
        }]
    }

    /// Return `true` if the resource is accepted.
    ///
    pub fn is_accepted(&self) -> bool {
        self.conditions.iter().any(|condition| {
            condition.type_ == "Accepted"
                && condition.reason == "Accepted"
                && condition.status == "True"
        })
    }
}
