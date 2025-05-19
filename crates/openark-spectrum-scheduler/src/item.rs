use std::borrow::Cow;

use openark_spectrum_api::pool_claim::{PoolClaimCrd, PoolResourceLifecycle};
use ordered_float::OrderedFloat;

#[derive(Debug)]
pub struct Resource {
    pub penalty: f64,
    pub priority: i32,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub weight: u64,
}

impl Default for Resource {
    fn default() -> Self {
        Self {
            penalty: Default::default(),
            priority: Default::default(),
            min: Default::default(),
            max: Default::default(),
            weight: 1,
        }
    }
}

#[derive(Debug)]
pub struct Item<'a, T> {
    pub claim: Cow<'a, PoolClaimCrd>,
    pub resource: Resource,
    pub item: T,
}

#[derive(Clone, Debug)]
pub struct WeightedItems<T> {
    pub items: Vec<T>,
    pub weights: Vec<Option<OrderedFloat<f64>>>,
}

#[derive(Debug)]
pub struct ScheduledItem<S, T> {
    pub item: T,
    pub lifecycle: PoolResourceLifecycle,
    pub priority: i32,
    pub resources: Vec<S>,
}
