use good_lp::{
    Expression, IntoAffineExpression, LpSolver, ProblemVariables, ResolutionError, Solution,
    SolverModel, constraint,
    solvers::{
        ObjectiveDirection,
        lp_solvers::{CbcSolver, WithMaxSeconds, WithMipGap},
    },
    variable,
};

use crate::state::State;

pub(crate) struct Solver {
    pub(crate) direction: ObjectiveDirection,
    pub(crate) items: Vec<usize>,
    pub(crate) use_all: bool,
    pub(crate) use_max: bool,
    pub(crate) use_min: bool,
}

impl Solver {
    pub(crate) fn solve<T>(self, state: &mut State<'_, T>) -> Result<bool, ResolutionError> {
        let Self {
            direction,
            items: targets,
            use_all,
            use_max,
            use_min,
        } = self;

        let State {
            allocated,
            bound,
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
                let penalty = match bound[i].claim {
                    None => 0.0,
                    Some(j2) if j == j2 => 0.0,
                    Some(_) => items[j].resource.penalty,
                };
                load[col] = load[col].clone() + (weights[i] + penalty) * y[row][col];
            }
        }

        // (a) Bind each resource to an item
        let mut constraints = Vec::default();
        for row in y.iter().take(n_i) {
            let init = Expression::default();
            let expr = (0..n_j).fold(init, |acc, col| acc + row[col]);
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
                        constraints.push(constraint!(expr1.clone() - expr2.clone() <= d));
                        constraints.push(constraint!(expr2 - expr1.clone() <= d));
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
