pub mod item;
pub mod solvers;
mod state;

use std::collections::{BTreeMap, BTreeSet};

use good_lp::{ResolutionError, solvers::ObjectiveDirection};
use openark_spectrum_api::schema::{CommitState, PoolResource};
use ordered_float::OrderedFloat;

use crate::{
    item::{Item, ScheduledItem, WeightedItems},
    state::State,
};

pub fn schedule<'a, S, T>(
    items: Vec<Item<'a, T>>,
    bound: Vec<PoolResource<usize>>,
    resources: WeightedItems<S>,
) -> Result<Vec<ScheduledItem<S, T>>, ResolutionError> {
    // ****************************************
    // Step 0: Validate inputs
    // ****************************************

    // Validate resources
    assert_eq!(resources.items.len(), resources.weights.len());

    // Do nothing if either items or resources are empty
    if items.is_empty() || resources.items.is_empty() {
        return Ok(Default::default());
    }

    // Prepare common variables
    let n_j = items.len();

    // Flatten weights by average
    let weights_min = resources
        .weights
        .iter()
        .copied()
        .flatten()
        .min()
        .unwrap_or_default()
        .into_inner()
        .max(1.0);

    let weights_max = resources
        .weights
        .iter()
        .copied()
        .flatten()
        .max()
        .unwrap_or_default()
        .into_inner()
        .min(weights_min);

    let weights_avg = (weights_min + weights_max) / 2.0;
    let weights: Vec<_> = resources
        .weights
        .iter()
        .map(|opt| opt.map(OrderedFloat::into_inner).unwrap_or(weights_avg))
        .collect();

    // Prefill locked resources
    let mut allocated = vec![Vec::default(); n_j];
    let mut filled = vec![0.0f64; n_j];
    let mut remaining = BTreeSet::default();

    for (i, last) in bound.iter().enumerate() {
        let PoolResource { claim, state } = last;
        match (claim, state) {
            // Locked
            (&Some(j), CommitState::Preparing) => {
                allocated[j].push(i);
                filled[j] += weights[i];
            }
            // Free
            (_, CommitState::Pending | CommitState::Running) | (None, CommitState::Preparing) => {
                remaining.insert(i);
            }
        }
    }

    // Define state
    let mut state = State {
        allocated,
        bound,
        filled,
        items,
        remaining,
        weights,
    };

    // ****************************************
    // Step 1: Split items by resource claims
    // ****************************************

    let mut best_effort: Vec<usize> = Vec::default();
    let mut burstable: Vec<usize> = Vec::default();
    let mut guaranteed: Vec<usize> = Vec::default();

    for (index, b) in state.items.iter().enumerate() {
        if b.resource.max.is_none() {
            best_effort.push(index);
        }
        if b.resource.min.is_some() {
            burstable.push(index);
            if b.resource.max.is_some() {
                guaranteed.push(index);
            }
        }
    }

    // Group items by (priority)
    let priority: Vec<_> = state
        .items
        .iter()
        .map(|item| item.resource.priority)
        .collect();
    let build_tiers = |targets: Vec<usize>| {
        let mut tiers: BTreeMap<_, Vec<_>> = BTreeMap::default();
        for j in targets {
            tiers.entry(priority[j]).or_default().push(j);
        }
        tiers.into_values()
    };

    // ****************************************
    // Step 2-A: Guaranteed -> MILP
    // ****************************************

    if !guaranteed.is_empty() {
        for items in build_tiers(guaranteed) {
            let problem = self::solvers::milp::Solver {
                direction: ObjectiveDirection::Maximisation,
                items,
                use_all: false,
                use_max: true,
                use_min: true,
            };
            if !problem.solve(&mut state)? {
                break;
            }
        }
    }

    // ****************************************
    // Step 2-B: Burstable -> MILP
    // ****************************************

    if !burstable.is_empty() {
        for items in build_tiers(burstable) {
            let problem = self::solvers::milp::Solver {
                direction: ObjectiveDirection::Maximisation,
                items,
                use_all: false,
                use_max: true,
                use_min: true,
            };
            if !problem.solve(&mut state)? {
                break;
            }
        }
    }

    // ****************************************
    // Step 2-C: Best Effort -> Evenly
    // ****************************************

    drop(priority);
    if !state.remaining.is_empty() && !best_effort.is_empty() {
        let problem = self::solvers::milp::Solver {
            direction: ObjectiveDirection::Minimisation,
            items: best_effort,
            use_all: true,
            use_max: false,
            use_min: false,
        };
        problem.solve(&mut state)?;
    }

    // ****************************************
    // Step 3: Collect allocated resources
    // ****************************************

    Ok(state.collect(resources))
}

#[cfg(test)]
mod tests {
    use kube::api::ObjectMeta;
    use openark_spectrum_api::pool_claim::PoolClaimSpec;

    use super::{super::*, *};

    fn aggregate_resources<'a, T>(
        items: &[ScheduledItem<usize, &'a T>],
        resources: &WeightedItems<usize>,
    ) -> (Vec<Vec<usize>>, Vec<f64>)
    where
        T: ?Sized,
    {
        let indices = items.iter().map(|item| item.resources.clone()).collect();

        let weights = items
            .iter()
            .map(|item| {
                item.resources
                    .iter()
                    .map(|&index| resources.weights[index].unwrap().0)
                    .sum()
            })
            .collect();

        (indices, weights)
    }

    fn build_resources_unbound<'a>(
        weights: Vec<Option<f64>>,
    ) -> (Vec<PoolResource<usize>>, WeightedItems<usize>) {
        let bound = (0..weights.len())
            .map(|_| PoolResource::default())
            .collect();
        let resources = WeightedItems {
            items: (0..weights.len()).collect(),
            weights: weights.into_iter().map(|x| x.map(OrderedFloat)).collect(),
        };
        (bound, resources)
    }

    #[inline]
    fn define_item<'a>(name: &'a str) -> Item<'a, &'a str> {
        let resource = Resource::default();
        define_item_with_resource(name, resource)
    }

    #[inline]
    fn define_item_with_priority<'a>(name: &'a str, priority: i32) -> Item<'a, &'a str> {
        let resource = Resource {
            priority,
            ..Default::default()
        };
        define_item_with_resource(name, resource)
    }

    #[inline]
    fn define_item_with_weight<'a>(name: &'a str, weight: u64) -> Item<'a, &'a str> {
        let resource = Resource {
            weight,
            ..Default::default()
        };
        define_item_with_resource(name, resource)
    }

    fn define_item_with_resource<'a>(name: &'a str, resource: Resource) -> Item<'a, &'a str> {
        Item {
            claim: Cow::Owned(PoolClaimCrd {
                metadata: ObjectMeta {
                    name: Some(name.into()),
                    ..Default::default()
                },
                spec: PoolClaimSpec {
                    pool_name: "test".into(),
                    lifecycle: Default::default(),
                    resources: Default::default(),
                },
                status: None,
            }),
            resource,
            item: name,
        }
    }

    #[test]
    fn test_distribute_best_effort() {
        let items = vec![define_item("a"), define_item("b")];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 1, 3], vec![2, 4]]);
        assert_eq!(weights, &[700.0, 800.0]);
    }

    #[test]
    fn test_distribute_best_effort_with_priority() {
        let items = vec![
            define_item_with_priority("a", 0),
            define_item_with_priority("b", 1),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 1, 3], vec![2, 4]]);
        assert_eq!(weights, &[700.0, 800.0]);
    }

    #[test]
    fn test_distribute_best_effort_with_weight_1() {
        let items = vec![
            define_item_with_weight("a", 1),
            define_item_with_weight("b", 2),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![1, 2], vec![0, 3, 4]]);
        assert_eq!(weights, &[500.0, 1000.0]);
    }

    #[test]
    fn test_distribute_best_effort_with_weight_2() {
        let items = vec![
            define_item_with_weight("a", 1),
            define_item_with_weight("b", 3),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 2], vec![1, 3, 4]]);
        assert_eq!(weights, &[400.0, 1100.0]);
    }

    #[test]
    fn test_distribute_best_effort_with_weight_3() {
        let items = vec![
            define_item_with_weight("a", 1),
            define_item_with_weight("b", 4),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 1], vec![2, 3, 4]]);
        assert_eq!(weights, &[300.0, 1200.0]);
    }

    #[test]
    fn test_distribute_burstable_1() {
        let items = vec![
            define_item_with_resource(
                "a",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(800.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "b",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(500.0),
                    max: None,
                    weight: 1,
                },
            ),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 2, 3], vec![4, 1]]);
        assert_eq!(weights, &[800.0, 700.0]);
    }

    #[test]
    fn test_distribute_burstable_2() {
        let items = vec![
            define_item_with_resource(
                "a",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(400.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "b",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(300.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "c",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(300.0),
                    max: None,
                    weight: 1,
                },
            ),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![3], vec![0, 1, 4], vec![2]]);
        // + Step 2-B: [[3], [0, 1], [2]] -> [400.0, 300.0, 300.0]
        // + Step 2-C: [[ ], [   4], [ ]] -> [  0.0, 500.0,   0.0]
        assert_eq!(weights, &[400.0, 800.0, 300.0]);
    }

    #[test]
    fn test_distribute_burstable_infeasible() {
        let items = vec![
            define_item_with_resource(
                "a",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(800.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "b",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(500.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "c",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(500.0),
                    max: None,
                    weight: 1,
                },
            ),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![4], vec![0, 3], vec![1, 2]]);
        assert_eq!(weights, &[500.0, 500.0, 500.0]);
    }

    #[test]
    fn test_distribute_burstable_infeasible_priority() {
        let items = vec![
            define_item_with_resource(
                "a",
                Resource {
                    penalty: 0.0,
                    priority: -1,
                    min: Some(800.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "b",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(500.0),
                    max: None,
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "c",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(500.0),
                    max: None,
                    weight: 1,
                },
            ),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![2, 4], vec![3], vec![0, 1]]);
        assert_eq!(weights, &[800.0, 400.0, 300.0]);
    }

    #[test]
    fn test_distribute_guaranteed() {
        let items = vec![
            define_item_with_resource(
                "a",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(1000.0),
                    max: Some(1000.0),
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "b",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(500.0),
                    max: Some(500.0),
                    weight: 1,
                },
            ),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 3, 4], vec![1, 2]]);
        assert_eq!(weights, &[1000.0, 500.0]);
    }

    #[test]
    fn test_distribute_guaranteed_with_priority() {
        let items = vec![
            define_item_with_resource(
                "a",
                Resource {
                    penalty: 0.0,
                    priority: 0,
                    min: Some(1000.0),
                    max: Some(1000.0),
                    weight: 1,
                },
            ),
            define_item_with_resource(
                "b",
                Resource {
                    penalty: 0.0,
                    priority: 1,
                    min: Some(500.0),
                    max: Some(500.0),
                    weight: 1,
                },
            ),
        ];
        let (bound, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, bound, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 3, 4], vec![1, 2]]);
        assert_eq!(weights, &[1000.0, 500.0]);
    }
}
