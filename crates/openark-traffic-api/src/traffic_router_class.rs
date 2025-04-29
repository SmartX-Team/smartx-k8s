use chrono::DateTime;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
#[cfg(feature = "kube")]
use kube::CustomResource;
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Spec defines the desired state of TrafficRouterClass.
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
        kind = "TrafficRouterClass",
        root = "TrafficRouterClassCrd",
        status = "TrafficRouterClassStatus",
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
        }"#
    )
)]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TrafficRouterClassSpec {
    /// ControllerName is the name of the controller that is managing Routers
    /// of this class. The value of this field MUST be a domain prefixed path.
    ///
    /// Example: "example.com/tranffc-router-controller".
    ///
    /// This field is not mutable and cannot be empty.
    ///
    /// Support: Core
    pub controller_name: String,

    /// Description helps describe a TrafficRouterClass with more details.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "String::is_empty")
    )]
    pub description: String,

    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub parameters_ref: Option<PartialObjectReference>,
}

/// PartialObjectReference is a reference to a resource.
///
/// PartialObjectReference can reference a standard Kubernetes resource, i.e. ConfigMap,
/// or an implementation-specific custom resource. The resource can be
/// cluster-scoped or namespace-scoped.
///
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PartialObjectReference {
    /// Group is the group of the referent.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "String::is_empty")
    )]
    pub group: String,

    /// Kind is kind of the referent.
    pub kind: String,

    /// Name is the name of the referent.
    pub name: String,

    /// Namespace is the namespace of the referent.
    /// This field is required when referring to a Namespace-scoped resource and
    /// MUST be unset when referring to a Cluster-scoped resource.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub namespace: Option<String>,
}

/// Status defines the current state of TrafficRouterClass.
///
/// Implementations MUST populate status on all TrafficRouterClass
/// resources which specify their controller name.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct TrafficRouterClassStatus {
    /// Conditions is the current status from the controller for
    /// this TrafficRouterClass.
    ///
    /// Controllers should prefer to publish conditions using values
    /// of TrafficRouterClassConditionType for the type of each Condition.
    #[serde(default = "TrafficRouterClassStatus::default_conditions")]
    pub conditions: Vec<Condition>,
}

impl Default for TrafficRouterClassStatus {
    fn default() -> Self {
        Self {
            conditions: Self::default_conditions(),
        }
    }
}

impl TrafficRouterClassStatus {
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
