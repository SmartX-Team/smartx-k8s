use chrono::DateTime;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::common::ServiceReference;

#[cfg(feature = "opeartor")]
impl ::openark_core::operator::Resource for MetricsClassCrd {
    type Status = MetricsClassStatus;

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
        MetricsClassStatus { conditions }
    }
}

/// Spec defines the desired state of MetricsClass.
///
#[derive(Clone, Debug)]
#[cfg_attr(feature = "kube", derive(CustomResource))]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "kube",
    kube(
        category = "org",
        group = "org.ulagbulag.io",
        version = "v1alpha1",
        kind = "MetricsClass",
        root = "MetricsClassCrd",
        status = "MetricsClassStatus",
        printcolumn = r#"{
            "name": "controller",
            "type": "string",
            "jsonPath": ".spec.controllerName"
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
            "name": "description",
            "type": "string",
            "priority": 1,
            "jsonPath": ".spec.description"
        }"#,
        printcolumn = r#"{
            "name": "version",
            "type": "integer",
            "priority": 1,
            "description": "class version",
            "jsonPath": ".metadata.generation"
        }"#,
        selectable = ".spec.backendRef.group",
        selectable = ".spec.backendRef.kind",
        selectable = ".spec.backendRef.name",
        selectable = ".spec.backendRef.namespace",
        selectable = ".spec.backendRef.port",
        selectable = ".spec.controllerName",
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MetricsClassSpec {
    /// ControllerName is the name of the controller that is managing
    /// Metrics of this class. The value of this field MUST be a domain
    /// prefixed path.
    ///
    /// Example: "example.com/metrics-controller".
    ///
    /// This field is not mutable and cannot be empty.
    pub controller_name: String,

    /// Description helps describe a class with more details.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "String::is_empty")
    )]
    pub description: String,

    pub backend_ref: ServiceReference,
}

impl MetricsClassSpec {
    pub const FIELD_CONTROLLER_NAME: &'static str = "controllerName";
}

/// Status defines the current state of Class.
///
/// Implementations MUST populate status on all Class
/// resources which specify their controller name.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct MetricsClassStatus {
    /// Conditions is the current status from the controller for
    /// this Class.
    ///
    /// Controllers should prefer to publish conditions using values
    /// of MetricsClassConditionType for the type of each Condition.
    #[serde(default = "MetricsClassStatus::default_conditions")]
    pub conditions: Vec<Condition>,
}

impl Default for MetricsClassStatus {
    fn default() -> Self {
        Self {
            conditions: Self::default_conditions(),
        }
    }
}

impl MetricsClassStatus {
    fn default_conditions() -> Vec<Condition> {
        vec![Condition {
            last_transition_time: Time(DateTime::default()),
            message: "Waiting for controller".into(),
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
