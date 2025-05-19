use std::collections::BTreeSet;

use openark_spectrum_api::schema::PoolResource;
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::item::{Item, ScheduledItem, WeightedItems};

pub(crate) struct State<'a, T> {
    pub(crate) allocated: Vec<Vec<usize>>,
    pub(crate) binded: Vec<PoolResource<usize>>,
    pub(crate) filled: Vec<f64>,
    pub(crate) items: Vec<Item<'a, T>>,
    pub(crate) remaining: BTreeSet<usize>,
    pub(crate) weights: Vec<f64>,
}

impl<'a, T> State<'a, T> {
    pub(super) fn collect<S>(self, resources: WeightedItems<S>) -> Vec<ScheduledItem<S, T>> {
        let Self {
            allocated,
            filled,
            items,
            ..
        } = self;

        let mut resources: Vec<_> = resources.items.into_iter().map(Some).collect();
        #[cfg(feature = "tracing")]
        {
            debug!("Scheduled: {filled:?}");
        }
        #[cfg(not(feature = "tracing"))]
        {
            let _ = filled;
        }

        items
            .into_iter()
            .zip(allocated)
            .map(
                |(
                    Item {
                        claim,
                        item,
                        resource,
                    },
                    allocated,
                )| ScheduledItem {
                    lifecycle: claim.spec.lifecycle.clone(),
                    item,
                    priority: resource.priority,
                    resources: allocated
                        .into_iter()
                        .filter_map(|index| {
                            resources.get_mut(index).and_then(|option| option.take())
                        })
                        .collect(),
                },
            )
            .collect()
    }
}
