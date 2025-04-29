use chrono::DateTime;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{Condition, Time};
#[cfg(feature = "schemars")]
use schemars::JsonSchema;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Status defines the current state of RouteClaim.
///
/// Implementations MUST populate status on all RouteClaim
/// resources which specify their class name.
///
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "schemars", derive(JsonSchema))]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct RouteClaimStatus {
    /// Conditions is the current status from the class for
    /// this RouteClaim.
    ///
    /// Classes should prefer to publish conditions using values
    /// of RouteClaim for the type of each Condition.
    #[serde(default = "RouteClaimStatus::default_conditions")]
    pub conditions: Vec<Condition>,
}

impl Default for RouteClaimStatus {
    fn default() -> Self {
        Self {
            conditions: Self::default_conditions(),
        }
    }
}

impl RouteClaimStatus {
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
