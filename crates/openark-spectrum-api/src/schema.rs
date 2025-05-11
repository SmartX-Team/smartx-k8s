use std::borrow::Cow;

use kube::api::ObjectMeta;
use ordered_float::OrderedFloat;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::pool_claim::PoolResourceLifecycle;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
#[repr(u8)]
pub enum CommitState {
    #[default]
    Pending = 0,
    Preparing,
    Running,
}

impl CommitState {
    pub const fn from_raw(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Pending),
            1 => Some(Self::Preparing),
            2 => Some(Self::Running),
            _ => None,
        }
    }
}

#[derive(Copy, Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolResource<T = String> {
    pub claim: Option<T>,
    pub state: CommitState,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolRequest<'a> {
    pub namespace: String,
    pub resources: Vec<Cow<'a, str>>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolResponse {
    pub binded: Vec<PoolResource>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolCommitRequestItem<'a> {
    pub lifecycle: PoolResourceLifecycle,
    pub name: Cow<'a, str>,
    #[serde(flatten)]
    pub pool: PoolRequest<'a>,
    pub priority: i32,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct PoolCommitRequest<'a> {
    pub items: Vec<PoolCommitRequestItem<'a>>,
}

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct WeightRequest<'a, T>
where
    T: Clone,
{
    pub metadata: ObjectMeta,
    pub list: Cow<'a, [T]>,
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct WeightResponse {
    pub weights: Vec<Option<OrderedFloat<f64>>>,
}
