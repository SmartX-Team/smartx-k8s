use std::{
    borrow::Cow,
    collections::{BTreeMap, BTreeSet},
    convert::identity,
};

use good_lp::{
    Expression, IntoAffineExpression, LpSolver, ProblemVariables, ResolutionError, Solution,
    SolverModel, constraint,
    solvers::{
        ObjectiveDirection,
        lp_solvers::{CbcSolver, WithMaxSeconds, WithMipGap},
    },
    variable,
};
use openark_spectrum_api::{
    pool_claim::{PoolClaimCrd, PoolResourceLifecycle},
    schema::{CommitState, PoolResource},
};
use ordered_float::OrderedFloat;
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::targets::WeightedItems;

#[derive(Debug)]
pub(crate) struct Resource {
    pub(crate) penalty: f64,
    pub(crate) priority: i32,
    pub(crate) min: Option<f64>,
    pub(crate) max: Option<f64>,
    pub(crate) weight: u64,
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
pub(crate) struct Item<'a, T> {
    pub(crate) claim: Cow<'a, PoolClaimCrd>,
    pub(crate) resource: Resource,
    pub(crate) item: T,
}

struct State<'a, T> {
    allocated: Vec<Vec<usize>>,
    binded: Vec<PoolResource<usize>>,
    filled: Vec<f64>,
    items: Vec<Item<'a, T>>,
    remaining: BTreeSet<usize>,
    weights: Vec<f64>,
}

impl<'a, T> State<'a, T> {
    fn collect<S>(self, resources: WeightedItems<S>) -> Vec<ScheduledItem<S, T>> {
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

struct SingleLayerProblem {
    direction: ObjectiveDirection,
    items: Vec<usize>,
    use_all: bool,
    use_max: bool,
    use_min: bool,
}

impl SingleLayerProblem {
    fn solve<T>(self, state: &mut State<'_, T>) -> Result<bool, ResolutionError> {
        let SingleLayerProblem {
            direction,
            items: targets,
            use_all,
            use_max,
            use_min,
        } = self;

        let State {
            allocated,
            binded,
            filled,
            items,
            remaining,
            weights,
        } = state;

        // Define constants
        let n_i = remaining.len();
        let n_j = targets.len();

        // Stop if no more items or remaining resources
        if n_i == 0 || n_j == 0 {
            return Ok(false);
        }

        // Define variable matrix [remaining, targets]
        let mut vars = ProblemVariables::default();
        let y: Vec<Vec<_>> = (0..n_i)
            .map(|_| {
                targets
                    .iter()
                    .map(|_| vars.add(variable().binary()))
                    .collect()
            })
            .collect();

        // Define `load` = prefilled + upcoming
        let mut load: Vec<Expression> = vec![Expression::default(); n_j];
        for (row, &i) in remaining.iter().enumerate() {
            for (col, &j) in targets.iter().enumerate() {
                let penalty = match binded[i].claim {
                    None => 0.0,
                    Some(j2) if j == j2 => 0.0,
                    Some(_) => items[j].resource.penalty,
                };
                load[col] = load[col].clone() + (weights[i] + penalty) * y[row][col];
            }
        }

        // (a) Bind each resource to an item
        let mut constraints = Vec::default();
        for row in 0..n_i {
            let init = Expression::default();
            let expr = (0..n_j).fold(init, |acc, col| acc + y[row][col]);
            let constraint = if use_all {
                // Required
                constraint!(expr == 1.0)
            } else {
                // Optional
                constraint!(expr <= 1.0)
            };
            constraints.push(constraint);
        }

        // (b) Guaranteed item min / max
        if use_min || use_max {
            for (col, &j) in targets.iter().enumerate() {
                let load_j = filled[j] + load[col].clone();
                if use_min {
                    if let Some(min) = items[j].resource.min {
                        let constraint = constraint!(load_j.clone() >= min);
                        constraints.push(constraint)
                    }
                }
                if use_max {
                    match items[j].resource.max {
                        Some(max) => {
                            let constraint = constraint!(load_j <= max);
                            constraints.push(constraint)
                        }
                        None => {
                            // max 없음 → "min만 배정"
                            let min = items[j].resource.min.expect("guaranteed item");
                            let constraint = constraint!(load_j == min);
                            constraints.push(constraint)
                        }
                    }
                }
            }
        }

        let objective = match direction {
            // (c) Just use resources as much as possible
            ObjectiveDirection::Maximisation => {
                let init = Expression::default();
                (0..n_j).fold(init, |acc, j| acc + load[j].clone())
            }
            // (c) Max/Min deviation variable d
            ObjectiveDirection::Minimisation => {
                let d = vars.add(variable().min(0.0));
                for (col_j1, &j1) in targets.iter().enumerate() {
                    let weight1 = items[j1].resource.weight as f64;
                    let expr1 = (filled[j1] + load[col_j1].clone()) / weight1;
                    for (col_j2, &j2) in targets.iter().enumerate() {
                        let weight2 = items[j2].resource.weight as f64;
                        let expr2 = (filled[j2] + load[col_j2].clone()) / weight2;
                        constraints.push(constraint!(expr1.clone() - expr2.clone() <= d.clone()));
                        constraints.push(constraint!(expr2 - expr1.clone() <= d.clone()));
                    }
                }
                d.into_expression()
            }
        };

        // Define solver & Bind to a model
        let solver = LpSolver(
            CbcSolver::default()
                .with_mip_gap(1.0 + 1e-6)?
                .with_max_seconds(1),
        );
        let model = vars
            .clone()
            .optimise(direction, objective)
            .using(solver)
            .with_all(constraints);

        let solution = match model.solve() {
            Ok(solver) => solver,
            // No more resources are left; stopping
            Err(ResolutionError::Infeasible) => return Ok(false),
            Err(error) => return Err(error),
        };

        // Store resources
        for (row, i) in remaining.clone().into_iter().enumerate() {
            for (col, &j) in targets.iter().enumerate() {
                if solution.value(y[row][col]) > 0.5 {
                    allocated[j].push(i);
                    filled[j] += weights[i];
                    remaining.remove(&i);
                }
            }
        }
        Ok(true)
    }
}

#[derive(Debug)]
pub(crate) struct ScheduledItem<S, T> {
    pub(crate) item: T,
    pub(crate) lifecycle: PoolResourceLifecycle,
    pub(crate) priority: i32,
    pub(crate) resources: Vec<S>,
}

pub(crate) fn schedule<'a, S, T>(
    items: Vec<Item<'a, T>>,
    binded: Vec<PoolResource<usize>>,
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
        .filter_map(identity)
        .min()
        .unwrap_or_default()
        .into_inner()
        .max(1.0);

    let weights_max = resources
        .weights
        .iter()
        .copied()
        .filter_map(identity)
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

    for (i, last) in binded.iter().enumerate() {
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
        binded,
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
            let problem = SingleLayerProblem {
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
            let problem = SingleLayerProblem {
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
        let problem = SingleLayerProblem {
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
        let binded = (0..weights.len())
            .map(|_| PoolResource::default())
            .collect();
        let resources = WeightedItems {
            items: (0..weights.len()).collect(),
            weights: weights.into_iter().map(|x| x.map(OrderedFloat)).collect(),
        };
        (binded, resources)
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
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
        let (binded, resources) = build_resources_unbound(vec![
            Some(100.0),
            Some(200.0),
            Some(300.0),
            Some(400.0),
            Some(500.0),
        ]);

        let items = schedule(items, binded, resources.clone()).unwrap();
        let (indices, weights) = aggregate_resources(&items, &resources);
        assert_eq!(indices, &[vec![0, 3, 4], vec![1, 2]]);
        assert_eq!(weights, &[1000.0, 500.0]);
    }
}
