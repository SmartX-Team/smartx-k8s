use chrono::DateTime;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::common::ObjectReference;

#[cfg(feature = "opeartor")]
impl ::openark_core::operator::Resource for HistogramCrd {
    type Status = HistogramStatus;

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
        HistogramStatus { conditions }
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
        kind = "Histogram",
        root = "HistogramCrd",
        status = "HistogramStatus",
        printcolumn = r#"{
            "name": "metrics",
            "type": "string",
            "jsonPath": ".spec.metricsClassName"
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
            "description": "histogram version",
            "jsonPath": ".metadata.generation"
        }"#,
        selectable = ".spec.metricsClassName",
        selectable = ".spec.targetRef.group",
        selectable = ".spec.targetRef.kind",
        selectable = ".spec.targetRef.name",
        selectable = ".spec.targetRef.namespace",
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct HistogramSpec {
    /// metricsClassName is the name of the class that is managing
    /// Metrics of this class.
    pub metrics_class_name: String,

    pub target_ref: ObjectReference,

    pub histogram: HistogramSettings,
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]

pub struct HistogramSettings {
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub accumulate: Option<bool>,

    /// Poll histogram per interval
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub interval: Option<u64>,

    pub size: u8,
}

/// Status defines the current state of Histogram.
///
/// Implementations MUST populate status on all Histogram
/// resources which specify their controller name.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct HistogramStatus {
    /// Conditions is the current status from the controller for
    /// this Histogram.
    ///
    /// Controllers should prefer to publish conditions using values
    /// of HistogramConditionType for the type of each Condition.
    #[serde(default = "HistogramStatus::default_conditions")]
    pub conditions: Vec<Condition>,
}

impl Default for HistogramStatus {
    fn default() -> Self {
        Self {
            conditions: Self::default_conditions(),
        }
    }
}

impl HistogramStatus {
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
