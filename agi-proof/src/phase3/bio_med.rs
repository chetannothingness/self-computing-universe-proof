// Phase 3B: Bio/Med Sandbox
//
// Deterministic gene regulatory network (GRN) simulation. The agent must
// identify regulators of a target phenotype gene and propose interventions.
// All values use integer arithmetic (i64/u64), BTreeMap for determinism,
// zero floats.

use kernel_bench::judge::JudgeVerdict;
use kernel_types::hash;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A directed interaction between two genes in the regulatory network.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GeneInteraction {
    /// Source gene index.
    pub from_gene: u32,
    /// Target gene index.
    pub to_gene: u32,
    /// Effect strength in milli-units (positive = activation, negative = repression).
    /// e.g. 500 means the source gene activates the target at 0.5x strength.
    pub effect_milli: i64,
}

/// Exogenous shocks in the bio simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BioShock {
    /// Increase noise in gene expression measurements.
    NoiseIncrease { noise_delta_milli: i64 },
    /// Budget cut reduces the number of assays that can be run.
    BudgetCut { budget_reduction_pct: i64 },
}

/// Actions the agent can take each simulation step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BioAction {
    /// Run an assay on a specific gene to measure its expression level.
    RunAssay { gene: u32 },
    /// Intervene on a gene with a specific type.
    Intervene { gene: u32, intervention: InterventionType },
    /// Allocate budget to a research direction (identified by gene index).
    AllocateBudget { gene: u32, budget_milli: i64 },
}

/// Types of gene intervention.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InterventionType {
    /// Force gene expression to maximum.
    Activate,
    /// Reduce gene expression by half.
    Repress,
    /// Completely silence the gene (expression = 0).
    Knockout,
}

/// The immutable world configuration for one bio episode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BioMedWorld {
    /// The 32-byte seed this world was generated from.
    pub seed: [u8; 32],
    /// Total number of genes in the network.
    pub num_genes: u32,
    /// Directed gene interaction graph.
    pub interactions: Vec<GeneInteraction>,
    /// The gene whose expression is the phenotype of interest.
    pub phenotype_gene: u32,
    /// Noise seed for deterministic pseudo-random noise in measurements.
    pub noise_seed: [u8; 32],
    /// Base noise level in milli-units.
    pub noise_milli: i64,
    /// Shock schedule: (step, shock).
    pub shock_schedule: Vec<(u64, BioShock)>,
    /// Simulation horizon (number of steps).
    pub horizon_steps: u64,
    /// Maximum assay budget (number of assays allowed).
    pub max_assays: u64,
    /// Baseline expression level for all genes (milli-units).
    pub baseline_expression_milli: i64,
}

/// Mutable state of the bio simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BioMedState {
    /// Current simulation step.
    pub step: u64,
    /// Gene expression levels in milli-units, keyed by gene index.
    pub expression: BTreeMap<u32, i64>,
    /// Interventions currently active: gene -> InterventionType.
    pub active_interventions: BTreeMap<u32, InterventionType>,
    /// Number of assays used so far.
    pub assays_used: u64,
    /// Current effective max assays (may be reduced by BudgetCut).
    pub effective_max_assays: u64,
    /// Current noise level in milli-units.
    pub current_noise_milli: i64,
    /// Assay results collected: (step, gene, measured_expression_milli).
    pub assay_results: Vec<(u64, u32, i64)>,
}

// ---------------------------------------------------------------------------
// Deterministic helpers
// ---------------------------------------------------------------------------

/// Derive a sub-seed by hashing seed || tag.
fn derive_seed(seed: &[u8; 32], tag: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + tag.len());
    buf.extend_from_slice(seed);
    buf.extend_from_slice(tag);
    hash::H(&buf)
}

/// Deterministic pseudo-random noise value for a given (noise_seed, step, gene).
/// Returns a value in [-noise_milli, +noise_milli].
fn noise_value(noise_seed: &[u8; 32], step: u64, gene: u32, noise_milli: i64) -> i64 {
    let mut buf = Vec::with_capacity(44);
    buf.extend_from_slice(noise_seed);
    buf.extend_from_slice(&step.to_le_bytes());
    buf.extend_from_slice(&gene.to_le_bytes());
    let h = hash::H(&buf);
    // Use first 2 bytes as u16, map to [-noise_milli, +noise_milli]
    let raw = u16::from_le_bytes([h[0], h[1]]) as i64; // 0..65535
    // Map: noise = (raw * 2 * noise_milli) / 65535 - noise_milli
    if noise_milli == 0 {
        return 0;
    }
    (raw * 2 * noise_milli) / 65535 - noise_milli
}

// ---------------------------------------------------------------------------
// World generation
// ---------------------------------------------------------------------------

/// Generate a deterministic `BioMedWorld` from seed and episode index.
///
/// The regulatory network is a sparse directed graph with deterministic
/// edge weights. All parameters derived from `H(seed || episode_le_bytes)`.
pub fn generate_bio_world(seed: &[u8; 32], episode: u32) -> BioMedWorld {
    // Episode seed
    let mut ep_buf = Vec::with_capacity(36);
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep = hash::H(&ep_buf);

    // Number of genes: 8..24
    let num_genes = 8 + (ep[0] as u32 % 17);

    // Phenotype gene: pick one
    let phenotype_gene = ep[1] as u32 % num_genes;

    // Build interaction graph: each gene has 1-3 outgoing edges.
    let interaction_seed = derive_seed(&ep, b"interactions");
    let mut interactions = Vec::new();

    for g in 0..num_genes {
        let g_seed = derive_seed(&interaction_seed, &g.to_le_bytes());
        let num_targets = 1 + (g_seed[0] as u32 % 3); // 1..3 outgoing edges

        for t in 0..num_targets {
            let t_seed = derive_seed(&g_seed, &t.to_le_bytes());
            let target = t_seed[0] as u32 % num_genes;
            if target == g {
                continue; // no self-loops
            }
            // Effect: range [-800, +800] milli, but never zero
            let raw_effect = (t_seed[1] as i64 % 160) * 10 - 800; // [-800, 790]
            let effect_milli = if raw_effect == 0 { 100 } else { raw_effect };

            interactions.push(GeneInteraction {
                from_gene: g,
                to_gene: target,
                effect_milli,
            });
        }
    }

    // Deduplicate: keep first interaction for each (from, to) pair.
    // Use BTreeMap for deterministic ordering.
    let mut seen: BTreeMap<(u32, u32), usize> = BTreeMap::new();
    let mut deduped = Vec::new();
    for inter in &interactions {
        let key = (inter.from_gene, inter.to_gene);
        if !seen.contains_key(&key) {
            seen.insert(key, deduped.len());
            deduped.push(inter.clone());
        }
    }
    let interactions = deduped;

    // Noise
    let noise_seed = derive_seed(&ep, b"noise");
    let noise_milli = 10 + (ep[2] as i64 % 50); // 10..59 milli

    // Shocks: 0..2
    let shock_seed = derive_seed(&ep, b"shocks");
    let num_shocks = shock_seed[0] as usize % 3;
    let horizon_steps = 20 + (ep[3] as u64 % 40); // 20..59 steps

    let mut shock_schedule = Vec::new();
    for i in 0..num_shocks {
        let s = derive_seed(&shock_seed, &(i as u32).to_le_bytes());
        let shock_step = 3 + (s[0] as u64 % (horizon_steps.saturating_sub(3).max(1)));
        let shock = if s[1] % 2 == 0 {
            BioShock::NoiseIncrease {
                noise_delta_milli: 10 + (s[2] as i64 % 40),
            }
        } else {
            BioShock::BudgetCut {
                budget_reduction_pct: 20 + (s[2] as i64 % 50),
            }
        };
        shock_schedule.push((shock_step, shock));
    }
    shock_schedule.sort_by_key(|(step, _)| *step);

    // Max assays
    let assay_seed = derive_seed(&ep, b"assays");
    let max_assays = 10 + (assay_seed[0] as u64 % 30); // 10..39

    // Baseline expression
    let baseline_expression_milli = 500; // 0.5 in milli-units

    BioMedWorld {
        seed: *seed,
        num_genes,
        interactions,
        phenotype_gene,
        noise_seed,
        noise_milli,
        shock_schedule,
        horizon_steps,
        max_assays,
        baseline_expression_milli,
    }
}

/// Create the initial `BioMedState` for a given world.
pub fn initial_bio_state(world: &BioMedWorld) -> BioMedState {
    let mut expression = BTreeMap::new();
    for g in 0..world.num_genes {
        expression.insert(g, world.baseline_expression_milli);
    }

    BioMedState {
        step: 0,
        expression,
        active_interventions: BTreeMap::new(),
        assays_used: 0,
        effective_max_assays: world.max_assays,
        current_noise_milli: world.noise_milli,
        assay_results: Vec::new(),
    }
}

// ---------------------------------------------------------------------------
// Simulation step
// ---------------------------------------------------------------------------

/// Advance the bio simulation by one step.
///
/// Processing order each step:
/// 1. Apply scheduled shocks.
/// 2. Execute the agent's action.
/// 3. Propagate gene regulatory network one step (integer arithmetic).
/// 4. Apply interventions (override expression for intervened genes).
/// 5. Advance step counter.
pub fn step_bio(
    world: &BioMedWorld,
    state: &mut BioMedState,
    action: &BioAction,
) {
    let step = state.step;

    // -----------------------------------------------------------------------
    // 1. Apply shocks
    // -----------------------------------------------------------------------
    for (shock_step, shock) in &world.shock_schedule {
        if *shock_step == step {
            match shock {
                BioShock::NoiseIncrease { noise_delta_milli } => {
                    state.current_noise_milli += noise_delta_milli;
                }
                BioShock::BudgetCut { budget_reduction_pct } => {
                    let reduction =
                        state.effective_max_assays as i64 * budget_reduction_pct / 100;
                    state.effective_max_assays = state
                        .effective_max_assays
                        .saturating_sub(reduction as u64)
                        .max(1); // at least 1 assay
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // 2. Execute action
    // -----------------------------------------------------------------------
    match action {
        BioAction::RunAssay { gene } => {
            if *gene < world.num_genes && state.assays_used < state.effective_max_assays {
                let true_expr = *state.expression.get(gene).unwrap_or(&0);
                let noise = noise_value(
                    &world.noise_seed,
                    step,
                    *gene,
                    state.current_noise_milli,
                );
                let measured = (true_expr + noise).max(0);
                state.assay_results.push((step, *gene, measured));
                state.assays_used += 1;
            }
        }
        BioAction::Intervene { gene, intervention } => {
            if *gene < world.num_genes {
                state.active_interventions.insert(*gene, intervention.clone());
            }
        }
        BioAction::AllocateBudget { gene: _, budget_milli: _ } => {
            // Budget allocation is a strategic signal; it doesn't directly
            // change gene expression. It could be used by a planning layer
            // to prioritize future assays. For the simulation, this is a no-op
            // on the expression state.
        }
    }

    // -----------------------------------------------------------------------
    // 3. Propagate GRN one step
    // -----------------------------------------------------------------------
    // For each gene, compute the new expression level:
    //   new_expr[g] = baseline + sum_over_regulators(expr[from] * effect_milli / 1000)
    //
    // This models a simple linear regulatory network with one-step propagation.
    // Build a map of (to_gene -> list of (from_gene, effect_milli)) for efficiency.
    let mut incoming: BTreeMap<u32, Vec<(u32, i64)>> = BTreeMap::new();
    for inter in &world.interactions {
        incoming
            .entry(inter.to_gene)
            .or_insert_with(Vec::new)
            .push((inter.from_gene, inter.effect_milli));
    }

    let mut new_expression: BTreeMap<u32, i64> = BTreeMap::new();
    for g in 0..world.num_genes {
        let base = world.baseline_expression_milli;
        let regulation: i64 = incoming
            .get(&g)
            .map(|regs| {
                regs.iter()
                    .map(|(from, effect)| {
                        let from_expr = *state.expression.get(from).unwrap_or(&0);
                        from_expr * effect / 1000
                    })
                    .sum()
            })
            .unwrap_or(0);
        // Clamp to non-negative
        let raw = base + regulation;
        new_expression.insert(g, raw.max(0));
    }

    state.expression = new_expression;

    // -----------------------------------------------------------------------
    // 4. Apply interventions (override expression)
    // -----------------------------------------------------------------------
    for (gene, intervention) in &state.active_interventions {
        match intervention {
            InterventionType::Activate => {
                // Set to maximum expression: 1000 milli (= 1.0)
                state.expression.insert(*gene, 1000);
            }
            InterventionType::Repress => {
                // Halve current expression
                let current = *state.expression.get(gene).unwrap_or(&0);
                state.expression.insert(*gene, current / 2);
            }
            InterventionType::Knockout => {
                // Silence completely
                state.expression.insert(*gene, 0);
            }
        }
    }

    // -----------------------------------------------------------------------
    // 5. Advance step
    // -----------------------------------------------------------------------
    state.step += 1;
}

// ---------------------------------------------------------------------------
// Phenotype measurement
// ---------------------------------------------------------------------------

/// Compute the phenotype expression level at the current state.
/// Returns the expression of the phenotype gene in milli-units.
pub fn phenotype_expression(world: &BioMedWorld, state: &BioMedState) -> i64 {
    *state.expression.get(&world.phenotype_gene).unwrap_or(&0)
}

/// Compute the baseline phenotype by simulating the network with no interventions
/// for the given number of steps. Returns the phenotype gene expression.
pub fn baseline_phenotype(world: &BioMedWorld, steps: u64) -> i64 {
    let mut state = initial_bio_state(world);
    for _ in 0..steps {
        step_bio(world, &mut state, &BioAction::RunAssay { gene: 0 });
    }
    phenotype_expression(world, &state)
}

// ---------------------------------------------------------------------------
// Judge
// ---------------------------------------------------------------------------

/// Judge the bio/med result.
///
/// PASS iff:
///   - The agent identified >= 50% of the true regulators of the phenotype gene.
///     A "true regulator" is any gene with a direct interaction edge to the
///     phenotype gene.
///   - The proposed intervention improves phenotype expression by > 30%
///     relative to the baseline (no-intervention) phenotype.
///     improvement_milli = (intervention_effect - baseline) * 1000 / baseline
///     PASS requires improvement_milli > 300.
pub fn judge_bio_med(
    world: &BioMedWorld,
    identified_mechanism: &[(u32, u32)],
    intervention_effect: i64,
) -> JudgeVerdict {
    // -----------------------------------------------------------------------
    // 1. Check mechanism identification (>= 50% overlap with true regulators)
    // -----------------------------------------------------------------------
    let true_regulators: Vec<(u32, u32)> = world
        .interactions
        .iter()
        .filter(|i| i.to_gene == world.phenotype_gene)
        .map(|i| (i.from_gene, i.to_gene))
        .collect();

    if true_regulators.is_empty() {
        // If no regulators exist, mechanism identification is trivially correct
        // only if the agent also identified none.
        if !identified_mechanism.is_empty() {
            return JudgeVerdict::Fail;
        }
    } else {
        let overlap = identified_mechanism
            .iter()
            .filter(|m| true_regulators.contains(m))
            .count();

        // >= 50% overlap: overlap * 2 >= true_regulators.len()
        if overlap * 2 < true_regulators.len() {
            return JudgeVerdict::Fail;
        }
    }

    // -----------------------------------------------------------------------
    // 2. Check intervention effect (> 30% improvement)
    // -----------------------------------------------------------------------
    // Compute baseline phenotype by running 10 steps with no intervention.
    let baseline = baseline_phenotype(world, 10);

    if baseline <= 0 {
        // If baseline is zero or negative, any positive intervention passes.
        if intervention_effect > 0 {
            return JudgeVerdict::Pass;
        } else {
            return JudgeVerdict::Fail;
        }
    }

    // improvement_milli = (intervention_effect - baseline) * 1000 / baseline
    let improvement_milli = (intervention_effect - baseline) * 1000 / baseline;

    if improvement_milli > 300 {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bio_med_world_deterministic() {
        let seed = [42u8; 32];
        let w1 = generate_bio_world(&seed, 3);
        let w2 = generate_bio_world(&seed, 3);

        assert_eq!(w1.num_genes, w2.num_genes);
        assert_eq!(w1.phenotype_gene, w2.phenotype_gene);
        assert_eq!(w1.interactions.len(), w2.interactions.len());
        assert_eq!(w1.noise_milli, w2.noise_milli);
        assert_eq!(w1.horizon_steps, w2.horizon_steps);
        assert_eq!(w1.max_assays, w2.max_assays);
        assert_eq!(w1.shock_schedule.len(), w2.shock_schedule.len());

        for (a, b) in w1.interactions.iter().zip(w2.interactions.iter()) {
            assert_eq!(a.from_gene, b.from_gene);
            assert_eq!(a.to_gene, b.to_gene);
            assert_eq!(a.effect_milli, b.effect_milli);
        }

        // Different episode => different world
        let w3 = generate_bio_world(&seed, 4);
        assert_ne!(w1.num_genes, w3.num_genes);
    }

    #[test]
    fn bio_med_intervention_effect() {
        // Create a small network where we know the phenotype gene and its regulators.
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 4,
            interactions: vec![
                GeneInteraction { from_gene: 0, to_gene: 2, effect_milli: 600 },
                GeneInteraction { from_gene: 1, to_gene: 2, effect_milli: 400 },
                GeneInteraction { from_gene: 2, to_gene: 3, effect_milli: 300 },
            ],
            phenotype_gene: 2,
            noise_seed: [1u8; 32],
            noise_milli: 0, // no noise for deterministic test
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 20,
            baseline_expression_milli: 500,
        };

        // Run baseline (no interventions) for 5 steps
        let mut state_baseline = initial_bio_state(&world);
        for _ in 0..5 {
            step_bio(&world, &mut state_baseline, &BioAction::RunAssay { gene: 0 });
        }
        let phenotype_baseline = phenotype_expression(&world, &state_baseline);

        // Run with Activate intervention on gene 0 (a regulator of gene 2)
        let mut state_intervened = initial_bio_state(&world);
        step_bio(
            &world,
            &mut state_intervened,
            &BioAction::Intervene {
                gene: 0,
                intervention: InterventionType::Activate,
            },
        );
        for _ in 0..4 {
            step_bio(&world, &mut state_intervened, &BioAction::RunAssay { gene: 2 });
        }
        let phenotype_intervened = phenotype_expression(&world, &state_intervened);

        // Activating a positive regulator should increase or at least not decrease
        // the phenotype expression. The exact values depend on the network dynamics.
        // Gene 0 was activated (set to 1000), gene 0->2 has effect 600,
        // so gene 2 gets an extra boost of 1000*600/1000 = 600 from gene 0
        // compared to baseline where gene 0 is at 500 (boost = 500*600/1000 = 300).
        // The intervention should produce a higher phenotype.
        assert!(
            phenotype_intervened >= phenotype_baseline,
            "Activating a positive regulator should increase phenotype: \
             intervened={}, baseline={}",
            phenotype_intervened,
            phenotype_baseline,
        );
    }

    #[test]
    fn bio_med_knockout_reduces_expression() {
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 3,
            interactions: vec![
                GeneInteraction { from_gene: 0, to_gene: 1, effect_milli: 800 },
                GeneInteraction { from_gene: 1, to_gene: 2, effect_milli: 600 },
            ],
            phenotype_gene: 2,
            noise_seed: [0u8; 32],
            noise_milli: 0,
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 20,
            baseline_expression_milli: 500,
        };

        // Run baseline
        let mut state_base = initial_bio_state(&world);
        for _ in 0..5 {
            step_bio(&world, &mut state_base, &BioAction::RunAssay { gene: 0 });
        }
        let base_pheno = phenotype_expression(&world, &state_base);

        // Knockout gene 1 (regulator of gene 2)
        let mut state_ko = initial_bio_state(&world);
        step_bio(
            &world,
            &mut state_ko,
            &BioAction::Intervene {
                gene: 1,
                intervention: InterventionType::Knockout,
            },
        );
        for _ in 0..4 {
            step_bio(&world, &mut state_ko, &BioAction::RunAssay { gene: 2 });
        }
        let ko_pheno = phenotype_expression(&world, &state_ko);

        // Knocking out a positive regulator should reduce phenotype expression
        assert!(
            ko_pheno <= base_pheno,
            "Knockout of positive regulator should reduce phenotype: ko={}, base={}",
            ko_pheno,
            base_pheno,
        );
    }

    #[test]
    fn bio_med_judge_correct_mechanism_passes() {
        // Build a world with known regulators of the phenotype gene.
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 5,
            interactions: vec![
                GeneInteraction { from_gene: 0, to_gene: 4, effect_milli: 600 },
                GeneInteraction { from_gene: 1, to_gene: 4, effect_milli: 400 },
                GeneInteraction { from_gene: 2, to_gene: 3, effect_milli: 500 },
                GeneInteraction { from_gene: 3, to_gene: 4, effect_milli: -200 },
            ],
            phenotype_gene: 4,
            noise_seed: [0u8; 32],
            noise_milli: 0,
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 20,
            baseline_expression_milli: 500,
        };

        // True regulators of gene 4: genes 0, 1, 3 => (0,4), (1,4), (3,4)
        // Identify 2 out of 3 => 66% overlap >= 50% => pass
        let identified = vec![(0, 4), (1, 4)];

        // Compute baseline phenotype
        let baseline = baseline_phenotype(&world, 10);

        // Provide intervention effect > 30% above baseline
        let intervention_effect = baseline * 140 / 100; // 40% improvement

        assert_eq!(
            judge_bio_med(&world, &identified, intervention_effect),
            JudgeVerdict::Pass,
        );
    }

    #[test]
    fn bio_med_judge_wrong_mechanism_fails() {
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 5,
            interactions: vec![
                GeneInteraction { from_gene: 0, to_gene: 4, effect_milli: 600 },
                GeneInteraction { from_gene: 1, to_gene: 4, effect_milli: 400 },
                GeneInteraction { from_gene: 2, to_gene: 4, effect_milli: 500 },
                // 3 true regulators: 0, 1, 2
            ],
            phenotype_gene: 4,
            noise_seed: [0u8; 32],
            noise_milli: 0,
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 20,
            baseline_expression_milli: 500,
        };

        // Only identify 1 out of 3 => 33% < 50% => fail
        let identified = vec![(0, 4)];
        let baseline = baseline_phenotype(&world, 10);
        let intervention_effect = baseline * 200 / 100; // big improvement, but mechanism wrong

        assert_eq!(
            judge_bio_med(&world, &identified, intervention_effect),
            JudgeVerdict::Fail,
        );
    }

    #[test]
    fn bio_med_judge_insufficient_intervention_fails() {
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 4,
            interactions: vec![
                GeneInteraction { from_gene: 0, to_gene: 3, effect_milli: 600 },
                GeneInteraction { from_gene: 1, to_gene: 3, effect_milli: 400 },
            ],
            phenotype_gene: 3,
            noise_seed: [0u8; 32],
            noise_milli: 0,
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 20,
            baseline_expression_milli: 500,
        };

        // Correct mechanism (100% overlap)
        let identified = vec![(0, 3), (1, 3)];
        let baseline = baseline_phenotype(&world, 10);

        // Intervention effect only 10% above baseline (< 30% threshold)
        let intervention_effect = baseline * 110 / 100;

        assert_eq!(
            judge_bio_med(&world, &identified, intervention_effect),
            JudgeVerdict::Fail,
        );
    }

    #[test]
    fn bio_med_assay_respects_budget() {
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 4,
            interactions: vec![],
            phenotype_gene: 0,
            noise_seed: [0u8; 32],
            noise_milli: 10,
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 3,
            baseline_expression_milli: 500,
        };

        let mut state = initial_bio_state(&world);

        // Run 5 assays, but budget is only 3
        for _ in 0..5 {
            step_bio(&world, &mut state, &BioAction::RunAssay { gene: 0 });
        }

        // Only 3 assay results should be recorded
        assert_eq!(state.assays_used, 3);
        assert_eq!(state.assay_results.len(), 3);
    }

    #[test]
    fn bio_med_noise_is_deterministic() {
        let noise_seed = [7u8; 32];
        let v1 = noise_value(&noise_seed, 5, 3, 100);
        let v2 = noise_value(&noise_seed, 5, 3, 100);
        assert_eq!(v1, v2);

        // Different step => different noise
        let v3 = noise_value(&noise_seed, 6, 3, 100);
        assert_ne!(v1, v3);
    }

    #[test]
    fn bio_med_grn_propagation() {
        // Verify that gene expression propagates through the network.
        // Gene 0 -> Gene 1 with positive effect, Gene 1 -> Gene 2 with positive effect.
        let world = BioMedWorld {
            seed: [0u8; 32],
            num_genes: 3,
            interactions: vec![
                GeneInteraction { from_gene: 0, to_gene: 1, effect_milli: 1000 },
                GeneInteraction { from_gene: 1, to_gene: 2, effect_milli: 1000 },
            ],
            phenotype_gene: 2,
            noise_seed: [0u8; 32],
            noise_milli: 0,
            shock_schedule: vec![],
            horizon_steps: 10,
            max_assays: 20,
            baseline_expression_milli: 500,
        };

        let mut state = initial_bio_state(&world);

        // All genes start at 500 milli.
        assert_eq!(*state.expression.get(&0).unwrap(), 500);
        assert_eq!(*state.expression.get(&1).unwrap(), 500);
        assert_eq!(*state.expression.get(&2).unwrap(), 500);

        // Step 1: propagate
        step_bio(&world, &mut state, &BioAction::RunAssay { gene: 0 });

        // Gene 1 = baseline(500) + gene0_expr(500) * effect(1000) / 1000 = 500 + 500 = 1000
        assert_eq!(*state.expression.get(&1).unwrap(), 1000);
        // Gene 2 = baseline(500) + gene1_expr_before_update(500) * effect(1000) / 1000 = 500 + 500 = 1000
        // Note: we use the PREVIOUS step's expression for propagation input, but
        // in our implementation we read from `state.expression` which was the old values.
        // Actually we compute new_expression from old state.expression, so gene 2 sees
        // gene 1's OLD value (500), not the new value (1000).
        assert_eq!(*state.expression.get(&2).unwrap(), 1000);
    }
}
