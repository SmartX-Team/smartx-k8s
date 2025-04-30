use std::borrow::Cow;

use kube::api::ObjectMeta;
use ordered_float::OrderedFloat;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

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

#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct WeightResponse {
    pub weights: Vec<Option<OrderedFloat<f64>>>,
}
