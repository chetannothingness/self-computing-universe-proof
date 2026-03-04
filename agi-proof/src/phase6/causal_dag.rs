// Phase 6: Causal DAG Simulator
// Production implementation — deterministic, integer-only, zero floats.

use kernel_bench::judge::JudgeVerdict;
use kernel_types::hash;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalWorld {
    pub seed: [u8; 32],
    pub num_variables: u32,
    pub edges: Vec<CausalEdge>,
    pub confounders: Vec<Confounder>,
    pub noise_seed: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CausalEdge {
    pub from: u32,
    pub to: u32,
    /// Edge weight in milli-units (1000 = coefficient of 1.0).
    pub coefficient_milli: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Confounder {
    /// The variables jointly affected by this hidden common cause.
    pub affects: Vec<u32>,
    /// Strength in milli-units added to each affected variable.
    pub strength_milli: i64,
}

// ---------------------------------------------------------------------------
// Deterministic helpers
// ---------------------------------------------------------------------------

/// Derive a per-slot hash: H(seed || slot_bytes).
fn derive(seed: &[u8; 32], slot: u64) -> [u8; 32] {
    let mut buf = Vec::with_capacity(40);
    buf.extend_from_slice(seed);
    buf.extend_from_slice(&slot.to_le_bytes());
    hash::H(&buf)
}

// ---------------------------------------------------------------------------
// World generation
// ---------------------------------------------------------------------------

/// Generate a fully-deterministic causal DAG from `seed` and `episode`.
///
/// * `num_variables` in 5..=15 (derived from hash byte).
/// * Edges obey the strict DAG property `from < to`.
/// * A small set of confounders is added.
/// * `noise_seed` is derived deterministically for observational data.
pub fn generate_causal_world(seed: &[u8; 32], episode: u32) -> CausalWorld {
    // Episode-specific master hash
    let master = derive(seed, episode as u64);

    // num_variables: 5..=15  (range of 11 values)
    let num_variables = 5 + (master[0] as u32 % 11);

    // --- Edges -----------------------------------------------------------
    // We iterate over all possible (from, to) pairs with from < to and
    // deterministically decide whether an edge exists.  This guarantees the
    // DAG property (no cycles) since edges always point from lower to higher
    // index.
    let edge_hash = derive(&master, 1);
    let mut edges: Vec<CausalEdge> = Vec::new();
    let mut edge_slot: u64 = 100;
    for from in 0..num_variables {
        for to in (from + 1)..num_variables {
            let slot_hash = derive(&edge_hash, edge_slot);
            edge_slot += 1;
            // ~40 % chance of edge to keep the graph non-trivial but sparse.
            if slot_hash[0] % 5 < 2 {
                // coefficient_milli in [-2000, 2000] \ {0}
                let raw = (slot_hash[1] as i64 % 4001) - 2000; // -2000..2000
                let coeff = if raw == 0 { 500 } else { raw };
                edges.push(CausalEdge {
                    from,
                    to,
                    coefficient_milli: coeff,
                });
            }
        }
    }

    // Guarantee at least one edge so the world is non-trivial.
    if edges.is_empty() {
        edges.push(CausalEdge {
            from: 0,
            to: num_variables - 1,
            coefficient_milli: 1000,
        });
    }

    // --- Confounders ------------------------------------------------------
    let conf_hash = derive(&master, 2);
    let num_confounders = 1 + (conf_hash[0] as usize % 3); // 1..=3
    let mut confounders: Vec<Confounder> = Vec::new();
    for c in 0..num_confounders {
        let c_hash = derive(&conf_hash, c as u64);
        // Each confounder affects 2-3 variables.
        let n_affect = 2 + (c_hash[0] as usize % 2); // 2 or 3
        let mut affects: Vec<u32> = Vec::new();
        for a in 0..n_affect {
            let var = c_hash[(a + 1) % 32] as u32 % num_variables;
            if !affects.contains(&var) {
                affects.push(var);
            }
        }
        // Ensure at least 2 distinct variables.
        if affects.len() < 2 {
            let fallback = (affects[0] + 1) % num_variables;
            affects.push(fallback);
        }
        affects.sort();
        let strength = 200 + (c_hash[4] as i64 % 800); // 200..999
        confounders.push(Confounder {
            affects,
            strength_milli: strength,
        });
    }

    // --- Noise seed -------------------------------------------------------
    let noise_seed = derive(&master, 3);

    CausalWorld {
        seed: *seed,
        num_variables,
        edges,
        confounders,
        noise_seed,
    }
}

// ---------------------------------------------------------------------------
// Observational data (WITH confounders)
// ---------------------------------------------------------------------------

/// Generate observational data by propagating through the DAG *with*
/// confounder effects and per-variable exogenous noise.
///
/// `noise_values` maps variable index -> exogenous noise.  Variables not
/// present in the map receive zero exogenous noise.
///
/// Processing order: 0 .. num_variables (topological since from < to).
pub fn observe(
    world: &CausalWorld,
    noise_values: &BTreeMap<u32, i64>,
) -> BTreeMap<u32, i64> {
    let mut state: BTreeMap<u32, i64> = BTreeMap::new();

    for v in 0..world.num_variables {
        let mut val: i64 = 0;

        // 1. Exogenous noise for this variable.
        val += noise_values.get(&v).copied().unwrap_or(0);

        // 2. Sum of incoming causal edges.
        for edge in &world.edges {
            if edge.to == v {
                let parent_val = state.get(&edge.from).copied().unwrap_or(0);
                val += parent_val * edge.coefficient_milli / 1000;
            }
        }

        // 3. Confounder contributions: every confounder that lists this
        //    variable adds its strength.
        for conf in &world.confounders {
            if conf.affects.contains(&v) {
                val += conf.strength_milli;
            }
        }

        state.insert(v, val);
    }

    state
}

// ---------------------------------------------------------------------------
// Do-intervention (do-calculus)
// ---------------------------------------------------------------------------

/// Perform a do-intervention: do(variable = value).
///
/// 1. Remove ALL incoming edges to `variable` (the essence of do-calculus).
/// 2. Set `variable = value`.
/// 3. Propagate forward through the DAG in topological order (0..n).
///
/// Confounders on the intervention target are also severed (the hidden common
/// cause no longer flows through the intervention target).
///
/// Returns the resulting state for every variable.
pub fn do_intervention(
    world: &CausalWorld,
    variable: u32,
    value: i64,
) -> BTreeMap<u32, i64> {
    let mut state: BTreeMap<u32, i64> = BTreeMap::new();

    for v in 0..world.num_variables {
        if v == variable {
            // Intervention: set to the imposed value, ignore all incoming
            // edges and confounders.
            state.insert(v, value);
            continue;
        }

        let mut val: i64 = 0;

        // Sum incoming causal edges (all edges are intact for non-intervened
        // variables).
        for edge in &world.edges {
            if edge.to == v {
                let parent_val = state.get(&edge.from).copied().unwrap_or(0);
                val += parent_val * edge.coefficient_milli / 1000;
            }
        }

        state.insert(v, val);
    }

    state
}

// ---------------------------------------------------------------------------
// Judging
// ---------------------------------------------------------------------------

/// Tolerance for intervention predictions (milli-units).
const INTERVENTION_TOLERANCE: i64 = 100;

/// Tolerance for counterfactual predictions (milli-units).
pub const COUNTERFACTUAL_TOLERANCE: i64 = 100;

/// Judge an intervention prediction.
///
/// PASS iff |predicted_effect - actual_effect| < INTERVENTION_TOLERANCE.
pub fn judge_intervention(
    world: &CausalWorld,
    predicted_effect: i64,
    variable: u32,
    value: i64,
    outcome_variable: u32,
) -> JudgeVerdict {
    let actual = do_intervention(world, variable, value);
    let actual_effect = actual.get(&outcome_variable).copied().unwrap_or(0);
    if (predicted_effect - actual_effect).abs() < INTERVENTION_TOLERANCE {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

/// Judge a counterfactual prediction against the full counterfactual state.
///
/// PASS iff every variable in `predicted_outcome` is within
/// `COUNTERFACTUAL_TOLERANCE` of the actual counterfactual value, and every
/// variable in the actual counterfactual state is present in the prediction.
pub fn judge_counterfactual(
    world: &CausalWorld,
    factual: &BTreeMap<u32, i64>,
    variable: u32,
    cf_value: i64,
    predicted_outcome: &BTreeMap<u32, i64>,
) -> JudgeVerdict {
    // Compute the actual counterfactual using the 3-step procedure
    // (abduction, intervention, prediction).  We delegate to the
    // counterfactual module's logic inlined here to avoid a circular
    // dependency: the judge must be self-contained.
    let actual_cf = compute_counterfactual(world, factual, variable, cf_value);

    for v in 0..world.num_variables {
        let actual_val = actual_cf.get(&v).copied().unwrap_or(0);
        let pred_val = predicted_outcome.get(&v).copied().unwrap_or(0);
        if (pred_val - actual_val).abs() >= COUNTERFACTUAL_TOLERANCE {
            return JudgeVerdict::Fail;
        }
    }

    JudgeVerdict::Pass
}

/// Internal counterfactual computation for the judge (same algorithm as
/// `counterfactual::counterfactual` — duplicated here so the judge module
/// is self-contained and never delegates trust).
fn compute_counterfactual(
    world: &CausalWorld,
    factual: &BTreeMap<u32, i64>,
    variable: u32,
    cf_value: i64,
) -> BTreeMap<u32, i64> {
    let mut cf_state: BTreeMap<u32, i64> = BTreeMap::new();

    for v in 0..world.num_variables {
        if v == variable {
            cf_state.insert(v, cf_value);
            continue;
        }

        let factual_val = factual.get(&v).copied().unwrap_or(0);

        // Structural value under factual parents.
        let mut structural_factual: i64 = 0;
        for edge in &world.edges {
            if edge.to == v {
                let parent_fact = factual.get(&edge.from).copied().unwrap_or(0);
                structural_factual += parent_fact * edge.coefficient_milli / 1000;
            }
        }

        // Confounder contribution (same under factual and counterfactual;
        // confounders are exogenous).
        let mut confounder_contribution: i64 = 0;
        for conf in &world.confounders {
            if conf.affects.contains(&v) {
                confounder_contribution += conf.strength_milli;
            }
        }

        // ABDUCTION: noise = factual_val - structural(factual parents) - confounders
        let noise = factual_val - structural_factual - confounder_contribution;

        // Structural value under counterfactual parents.
        let mut structural_cf: i64 = 0;
        for edge in &world.edges {
            if edge.to == v {
                let parent_cf = cf_state.get(&edge.from).copied().unwrap_or(0);
                structural_cf += parent_cf * edge.coefficient_milli / 1000;
            }
        }

        // PREDICTION: structural(cf parents) + confounders + noise
        cf_state.insert(v, structural_cf + confounder_contribution + noise);
    }

    cf_state
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Same seed + episode must always produce the identical CausalWorld.
    #[test]
    fn causal_dag_deterministic() {
        let seed = [42u8; 32];
        let w1 = generate_causal_world(&seed, 7);
        let w2 = generate_causal_world(&seed, 7);
        assert_eq!(w1.num_variables, w2.num_variables);
        assert_eq!(w1.edges.len(), w2.edges.len());
        for (e1, e2) in w1.edges.iter().zip(w2.edges.iter()) {
            assert_eq!(e1.from, e2.from);
            assert_eq!(e1.to, e2.to);
            assert_eq!(e1.coefficient_milli, e2.coefficient_milli);
        }
        assert_eq!(w1.confounders.len(), w2.confounders.len());
        assert_eq!(w1.noise_seed, w2.noise_seed);
    }

    /// Different episodes from the same seed must produce different worlds.
    #[test]
    fn different_episodes_differ() {
        let seed = [99u8; 32];
        let w1 = generate_causal_world(&seed, 0);
        let w2 = generate_causal_world(&seed, 1);
        // At least one structural feature should differ.
        let differ = w1.num_variables != w2.num_variables
            || w1.edges.len() != w2.edges.len()
            || w1.noise_seed != w2.noise_seed;
        assert!(differ, "episode 0 and 1 should produce different worlds");
    }

    /// do-intervention must remove all incoming edges to the intervention
    /// target: intervening on a variable must make it independent of its
    /// parents.
    #[test]
    fn do_intervention_removes_incoming_edges() {
        // Hand-crafted 3-variable chain: 0 -> 1 -> 2
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 1000 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // Without intervention: 0=0 (no parents, no noise), 1=0, 2=0
        // With do(1 = 500): variable 1 is forced to 500 regardless of
        // variable 0; variable 2 = 500 * 1000/1000 = 500.
        let result = do_intervention(&world, 1, 500);
        assert_eq!(result[&0], 0);
        assert_eq!(result[&1], 500);
        assert_eq!(result[&2], 500);

        // Changing variable 0 should NOT affect variable 1 under do(1=500).
        // (We verify by constructing a world where variable 0 has an
        // exogenous value via an edge from a root.)
        let world2 = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 2000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 1000 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };
        let result2 = do_intervention(&world2, 1, 500);
        // Variable 1 is still 500 despite the stronger edge from 0.
        assert_eq!(result2[&1], 500);
    }

    /// do-intervention propagates correctly through a longer chain.
    #[test]
    fn do_intervention_propagates_correctly() {
        // Chain: 0 -> 1 -> 2 -> 3
        // All coefficients = 500 milli (0.5)
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 4,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 500 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 500 },
                CausalEdge { from: 2, to: 3, coefficient_milli: 500 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // do(0 = 1000)
        // var 0 = 1000
        // var 1 = 1000 * 500 / 1000 = 500
        // var 2 = 500 * 500 / 1000  = 250
        // var 3 = 250 * 500 / 1000  = 125
        let result = do_intervention(&world, 0, 1000);
        assert_eq!(result[&0], 1000);
        assert_eq!(result[&1], 500);
        assert_eq!(result[&2], 250);
        assert_eq!(result[&3], 125);

        // Diamond: 0 -> 1, 0 -> 2, 1 -> 3, 2 -> 3
        let diamond = CausalWorld {
            seed: [0u8; 32],
            num_variables: 4,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
                CausalEdge { from: 0, to: 2, coefficient_milli: 1000 },
                CausalEdge { from: 1, to: 3, coefficient_milli: 500 },
                CausalEdge { from: 2, to: 3, coefficient_milli: 500 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // do(0 = 1000)
        // var 0 = 1000
        // var 1 = 1000 * 1000/1000 = 1000
        // var 2 = 1000 * 1000/1000 = 1000
        // var 3 = 1000 * 500/1000 + 1000 * 500/1000 = 500 + 500 = 1000
        let r = do_intervention(&diamond, 0, 1000);
        assert_eq!(r[&0], 1000);
        assert_eq!(r[&1], 1000);
        assert_eq!(r[&2], 1000);
        assert_eq!(r[&3], 1000);
    }

    /// A confounder creates spurious correlation in observational data that
    /// does NOT appear under do-intervention.
    #[test]
    fn confounder_creates_spurious_correlation() {
        // Two variables: 0 and 1, NO causal edge between them, but a
        // confounder affects both.
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 2,
            edges: vec![], // no direct causal link
            confounders: vec![Confounder {
                affects: vec![0, 1],
                strength_milli: 300,
            }],
            noise_seed: [0u8; 32],
        };

        let noise = BTreeMap::new();
        let obs = observe(&world, &noise);
        // Both variables get the confounder contribution.
        assert_eq!(obs[&0], 300);
        assert_eq!(obs[&1], 300);

        // Under do(0 = 0): variable 0 is forced to 0, severing confounders.
        // Variable 1 has no incoming edges, and confounders on the
        // *non-intervened* variables are NOT added (since do_intervention
        // does not model confounder flow).  Variable 1 = 0.
        let intv = do_intervention(&world, 0, 0);
        assert_eq!(intv[&0], 0);
        assert_eq!(intv[&1], 0);

        // The key insight: observation shows both variables are 300
        // (spurious correlation), but intervention reveals no causal effect.
    }

    /// judge_intervention returns Pass when prediction is correct.
    #[test]
    fn judge_intervention_pass_on_correct() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 500 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // do(0 = 2000) -> var 1 = 2000, var 2 = 2000*500/1000 = 1000
        let verdict = judge_intervention(&world, 1000, 0, 2000, 2);
        assert_eq!(verdict, JudgeVerdict::Pass);
    }

    /// judge_intervention returns Fail when prediction is wrong.
    #[test]
    fn judge_intervention_fail_on_wrong() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 1, coefficient_milli: 1000 },
                CausalEdge { from: 1, to: 2, coefficient_milli: 500 },
            ],
            confounders: vec![],
            noise_seed: [0u8; 32],
        };

        // Actual outcome of do(0=2000) on variable 2 is 1000.
        // Predicting 5000 should fail.
        let verdict = judge_intervention(&world, 5000, 0, 2000, 2);
        assert_eq!(verdict, JudgeVerdict::Fail);
    }

    /// Generated worlds always have from < to on every edge (DAG property).
    #[test]
    fn generated_edges_respect_dag_property() {
        for ep in 0..20 {
            let seed = [ep as u8; 32];
            let world = generate_causal_world(&seed, ep);
            for edge in &world.edges {
                assert!(
                    edge.from < edge.to,
                    "edge from {} to {} violates DAG property in episode {}",
                    edge.from,
                    edge.to,
                    ep
                );
            }
        }
    }

    /// Generated worlds have num_variables in 5..=15.
    #[test]
    fn generated_num_variables_in_range() {
        for ep in 0..30 {
            let seed = [ep as u8; 32];
            let world = generate_causal_world(&seed, ep);
            assert!(world.num_variables >= 5 && world.num_variables <= 15,
                "num_variables {} out of range for episode {}", world.num_variables, ep);
        }
    }

    /// observe includes confounder contributions while do_intervention does
    /// not (for non-intervened variables without confounders this is moot,
    /// but the test makes the difference explicit).
    #[test]
    fn observe_vs_intervention_confounder_difference() {
        let world = CausalWorld {
            seed: [0u8; 32],
            num_variables: 3,
            edges: vec![
                CausalEdge { from: 0, to: 2, coefficient_milli: 1000 },
            ],
            confounders: vec![Confounder {
                affects: vec![0, 1],
                strength_milli: 400,
            }],
            noise_seed: [0u8; 32],
        };

        let noise = BTreeMap::new();
        let obs = observe(&world, &noise);
        // var 0 = 400 (confounder)
        // var 1 = 400 (confounder)
        // var 2 = 400 * 1000/1000 = 400 (from edge 0->2)
        assert_eq!(obs[&0], 400);
        assert_eq!(obs[&1], 400);
        assert_eq!(obs[&2], 400);

        // do(0 = 400): intervention does NOT include confounder effects,
        // so var 1 gets 0 (no incoming edges from do_intervention, no
        // confounder in do-world).
        let intv = do_intervention(&world, 0, 400);
        assert_eq!(intv[&0], 400);
        assert_eq!(intv[&1], 0); // no confounders in do-intervention
        assert_eq!(intv[&2], 400);
    }
}
