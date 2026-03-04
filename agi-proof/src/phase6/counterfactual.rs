// Phase 6: Counterfactual Engine
// Production implementation — deterministic, integer-only, zero floats.
//
// Implements Pearl's 3-step counterfactual procedure:
//   1. ABDUCTION  — infer exogenous noise from the factual state.
//   2. INTERVENTION — set the counterfactual variable to a new value.
//   3. PREDICTION — propagate through the modified DAG with inferred noise.

use crate::phase6::causal_dag::{CausalWorld, COUNTERFACTUAL_TOLERANCE};
use kernel_bench::judge::JudgeVerdict;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Core counterfactual computation
// ---------------------------------------------------------------------------

/// Compute a counterfactual: "What would the world state have been if
/// `variable` had taken `counterfactual_value` instead of its factual value?"
///
/// The three steps:
///
/// **Abduction** — For every non-intervened variable v, recover its exogenous
/// noise: noise_v = factual_v - structural(factual parents of v) - confounder(v).
///
/// **Intervention** — Set `variable = counterfactual_value`, severing all
/// incoming edges to that variable.
///
/// **Prediction** — Propagate in topological order (0..num_variables) using
/// counterfactual parent values, confounders, and the inferred noise.
///
/// Processing order 0..num_variables is valid because every edge satisfies
/// `from < to` (DAG invariant enforced by world generation).
pub fn counterfactual(
    world: &CausalWorld,
    factual_state: &BTreeMap<u32, i64>,
    variable: u32,
    counterfactual_value: i64,
) -> BTreeMap<u32, i64> {
    let mut cf_state: BTreeMap<u32, i64> = BTreeMap::new();

    for v in 0..world.num_variables {
        // INTERVENTION: the target variable is forced to the counterfactual
        // value; all incoming edges and confounders are severed.
        if v == variable {
            cf_state.insert(v, counterfactual_value);
            continue;
        }

        // --- ABDUCTION: infer noise for variable v -----------------------

        let factual_val = factual_state.get(&v).copied().unwrap_or(0);

        // Structural contribution under factual parents.
        let mut structural_factual: i64 = 0;
        for edge in &world.edges {
            if edge.to == v {
                let parent_fact = factual_state.get(&edge.from).copied().unwrap_or(0);
                structural_factual += parent_fact * edge.coefficient_milli / 1000;
            }
        }

        // Confounder contribution (exogenous — same in factual and
        // counterfactual worlds).
        let mut confounder_contribution: i64 = 0;
        for conf in &world.confounders {
            if conf.affects.contains(&v) {
                confounder_contribution += conf.strength_milli;
            }
        }

        let noise = factual_val - structural_factual - confounder_contribution;

        // --- PREDICTION: compute v under counterfactual parents ----------

        let mut structural_cf: i64 = 0;
        for edge in &world.edges {
            if edge.to == v {
                let parent_cf = cf_state.get(&edge.from).copied().unwrap_or(0);
                structural_cf += parent_cf * edge.coefficient_milli / 1000;
            }
        }

        cf_state.insert(v, structural_cf + confounder_contribution + noise);
    }

    cf_state
}

// ---------------------------------------------------------------------------
// Judge
// ---------------------------------------------------------------------------

/// Judge a counterfactual prediction.
///
/// PASS iff for every variable 0..num_variables, the predicted value and the
/// actual counterfactual value differ by less than `COUNTERFACTUAL_TOLERANCE`.
pub fn judge_counterfactual_prediction(
    world: &CausalWorld,
    factual: &BTreeMap<u32, i64>,
    variable: u32,
    cf_value: i64,
    predicted: &BTreeMap<u32, i64>,
) -> JudgeVerdict {
    let actual_cf = counterfactual(world, factual, variable, cf_value);

    for v in 0..world.num_variables {
        let actual_val = actual_cf.get(&v).copied().unwrap_or(0);
        let pred_val = predicted.get(&v).copied().unwrap_or(0);
        if (pred_val - actual_val).abs() >= COUNTERFACTUAL_TOLERANCE {
            return JudgeVerdict::Fail;
        }
    }

    JudgeVerdict::Pass
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phase6::causal_dag::{
        CausalEdge, CausalWorld, Confounder, observe,
    };

    /// Verify that the abduction step correctly recovers exogenous noise.
    ///
    /// Setup: 2-variable chain 0 -> 1, coefficient 1000 (identity).
    /// Observe with noise_0 = 100, noise_1 = 50.
    /// Factual: var 0 = 100, var 1 = 100 * 1000/1000 + 50 = 150.
    /// Counterfactual do(0 = 200): var 1 should be 200 + 50 = 250.
    ///
    /// The noise for var 1 (50) must be recovered during abduction and
    /// carried through to the counterfactual.
    #[test]
    fn counterfactual_recovers_noise() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 2,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // Generate factual via observe with known noise.
        let mut noise = BTreeMap::new();
        noise.insert(0, 100);
        noise.insert(1, 50);
        let factual = observe(&world, &noise);
        assert_eq!(factual[&0], 100);
        assert_eq!(factual[&1], 150); // 100 * 1.0 + 50

        // Counterfactual: what if var 0 had been 200?
        let cf = counterfactual(&world, &factual, 0, 200);
        assert_eq!(cf[&0], 200);
        // var 1 = structural(200 * 1000/1000) + noise(50) = 250
        assert_eq!(cf[&1], 250);
    }

    /// Verify that the abduction step works with confounders.
    ///
    /// The confounder contribution is exogenous — the same in both factual
    /// and counterfactual worlds — so it must not be double-counted or lost.
    #[test]
    fn counterfactual_with_confounders() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 500 },
            ],
            confounders: vec![Confounder {
                affects: vec![0, 2],
                strength_milli: 200,
            }],
            noise_seed: [0u8; 32],
        };

        // Factual observation with zero exogenous noise.
        let noise = BTreeMap::new();
        let factual = observe(&world, &noise);
        // var 0 = 0 + confounder(200) = 200
        // var 1 = 200 * 1000/1000 = 200 (no confounder on var 1)
        // var 2 = 200 * 500/1000 + confounder(200) = 100 + 200 = 300
        assert_eq!(factual[&0], 200);
        assert_eq!(factual[&1], 200);
        assert_eq!(factual[&2], 300);

        // Counterfactual: what if var 0 had been 0?
        let cf = counterfactual(&world, &factual, 0, 0);
        assert_eq!(cf[&0], 0);
        // var 1: noise = factual(200) - structural(200*1.0) - conf(0) = 0
        //   cf_structural = 0 * 1.0 = 0, + conf(0) + noise(0) = 0
        assert_eq!(cf[&1], 0);
        // var 2: noise = factual(300) - structural(200*0.5) - conf(200) = 300-100-200 = 0
        //   cf_structural = 0 * 0.5 = 0, + conf(200) + noise(0) = 200
        assert_eq!(cf[&2], 200);
    }

    /// The judge should pass when predictions exactly match.
    #[test]
    fn judge_counterfactual_pass_on_correct() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 2000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 500 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // Factual with noise.
        let mut noise = BTreeMap::new();
        noise.insert(0, 100);
        noise.insert(1, 30);
        noise.insert(2, 10);
        let factual = observe(&world, &noise);
        // var 0 = 100
        // var 1 = 100 * 2000/1000 + 30 = 230
        // var 2 = 230 * 500/1000 + 10 = 115 + 10 = 125
        assert_eq!(factual[&0], 100);
        assert_eq!(factual[&1], 230);
        assert_eq!(factual[&2], 125);

        // Counterfactual: what if var 0 = 500?
        let actual_cf = counterfactual(&world, &factual, 0, 500);
        // var 0 = 500
        // var 1: noise = 230 - 100*2 - 0 = 30, cf = 500*2 + 30 = 1030
        // var 2: noise = 125 - 230*0.5 - 0 = 125-115 = 10, cf = 1030*0.5 + 10 = 525
        assert_eq!(actual_cf[&0], 500);
        assert_eq!(actual_cf[&1], 1030);
        assert_eq!(actual_cf[&2], 525);

        // Prediction matches exactly -> Pass.
        let verdict = judge_counterfactual_prediction(
            &world, &factual, 0, 500, &actual_cf,
        );
        assert_eq!(verdict, JudgeVerdict::Pass);
    }

    /// The judge should fail when predictions are too far off.
    #[test]
    fn judge_counterfactual_fail_on_wrong() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 2,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        let mut noise = BTreeMap::new();
        noise.insert(0, 100);
        noise.insert(1, 0);
        let factual = observe(&world, &noise);

        // Wildly wrong prediction.
        let mut wrong_pred = BTreeMap::new();
        wrong_pred.insert(0, 200);
        wrong_pred.insert(1, 9999);

        let verdict = judge_counterfactual_prediction(
            &world, &factual, 0, 200, &wrong_pred,
        );
        assert_eq!(verdict, JudgeVerdict::Fail);
    }

    /// Counterfactual on a non-ancestor variable should leave downstream
    /// variables unchanged (up to noise preservation).
    #[test]
    fn counterfactual_non_ancestor_unchanged() {
        // Graph: 0 -> 2, 1 -> 2. Variables 0 and 1 are independent roots.
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 2, coefficient_milli: 1000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 1000 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        let mut noise = BTreeMap::new();
        noise.insert(0, 100);
        noise.insert(1, 200);
        noise.insert(2, 0);
        let factual = observe(&world, &noise);
        // var 0 = 100, var 1 = 200, var 2 = 100 + 200 = 300
        assert_eq!(factual[&0], 100);
        assert_eq!(factual[&1], 200);
        assert_eq!(factual[&2], 300);

        // Counterfactual: what if var 0 = 400?
        let cf = counterfactual(&world, &factual, 0, 400);
        assert_eq!(cf[&0], 400);
        // var 1 is not a descendant of 0 and is not intervened upon ->
        // stays at factual value (noise preserved).
        assert_eq!(cf[&1], 200);
        // var 2: noise = 300 - (100+200) = 0, cf = 400 + 200 + 0 = 600
        assert_eq!(cf[&2], 600);
    }

    /// Verify determinism: running counterfactual twice produces the same
    /// result.
    #[test]
    fn counterfactual_deterministic() {
        let world = CausalWorld {
            seed: [7u8; 32],
            num_variables: 4,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 800 },
                CausalEdge { from: 1, to: 3, coefficient_milli: 600 },
                CausalEdge { from: 2, to: 3, coefficient_milli: 400 },
            ],
            confounders: vec![Confounder {
                affects: vec![0, 2],
                strength_milli: 150,
            }],
            noise_seed: [0u8; 32],
        };

        let mut noise = BTreeMap::new();
        noise.insert(0, 50);
        noise.insert(1, 20);
        noise.insert(2, 80);
        noise.insert(3, 10);
        let factual = observe(&world, &noise);

        let cf1 = counterfactual(&world, &factual, 1, 999);
        let cf2 = counterfactual(&world, &factual, 1, 999);
        assert_eq!(cf1, cf2);
    }
}
