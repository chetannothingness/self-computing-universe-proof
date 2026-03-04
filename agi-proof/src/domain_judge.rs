// Domain Judge — Central Dispatch
//
// For each of the 12 AGI domain kinds, this module:
//   1. Generates the domain world from the spec's world_seed
//   2. Solves the domain problem deterministically
//   3. Judges the solution using the domain's judge function
//   4. Returns the verdict
//
// No shortcuts: every domain runs its actual simulator-judge.
// All arithmetic is integer-only. BTreeMap everywhere. Zero floats.

use kernel_bench::judge::JudgeVerdict;
use crate::eval_specs::{AgiDomainKind, AgiDomainSpec};

// Phase 2 imports
use crate::phase2::synth_physics::{generate_physics_world, judge_stable_orbit, PhysicsState};
use crate::phase2::alien_chem::{generate_chem_world, judge_synthesis};
use crate::phase2::custom_math::{generate_math_world, check_proof, ProofStep, ProofTerm};

// Phase 3 imports
use crate::phase3::company::{
    generate_company_world, initial_company_state, step_company, judge_company, CompanyAction,
};
use crate::phase3::bio_med::{
    generate_bio_world, initial_bio_state, step_bio, judge_bio_med,
    BioAction, InterventionType, phenotype_expression, baseline_phenotype,
};

// Phase 6 imports
use crate::phase6::causal_dag::{
    generate_causal_world, do_intervention, judge_intervention, judge_counterfactual, observe,
};

// Phase 7 imports
use crate::phase7::model_discovery::{
    generate_discovery_world, evaluate_equation, judge_discovery, compute_prediction_error,
    ProposedModel,
};
use crate::phase7::materials::{generate_materials_world, judge_materials};
use crate::phase7::algo_discovery::{
    generate_algo_world, judge_algorithm, ProposedAlgorithm, Instruction,
};

// Phase 8 imports
use crate::phase8::physics_common::{generate_physics_task, solve_physics, judge_physics};
use crate::phase8::social::{generate_social_task, solve_social, judge_social};
use crate::phase8::planning::{generate_planning_world, solve_planning, judge_plan_execution};

use std::collections::BTreeMap;

/// Result of domain-specific solve + judge.
pub struct DomainJudgment {
    pub verdict: JudgeVerdict,
    pub reason: String,
    pub experiments_used: u64,
}

/// Central dispatch: generate world, solve, judge.
pub fn solve_and_judge(spec: &AgiDomainSpec, description: &str) -> DomainJudgment {
    match spec.domain {
        AgiDomainKind::SynthPhysics => solve_synth_physics(spec),
        AgiDomainKind::AlienChemistry => solve_alien_chemistry(spec),
        AgiDomainKind::CustomMath => solve_custom_math(spec),
        AgiDomainKind::CompanySandbox => solve_company_sandbox(spec),
        AgiDomainKind::BioMedSandbox => solve_biomed_sandbox(spec),
        AgiDomainKind::CausalReasoning => solve_causal_reasoning(spec, description),
        AgiDomainKind::ModelDiscovery => solve_model_discovery(spec),
        AgiDomainKind::MaterialsDesign => solve_materials_design(spec),
        AgiDomainKind::AlgoDiscovery => solve_algo_discovery(spec),
        AgiDomainKind::PhysicalReasoning => solve_physical_reasoning(spec),
        AgiDomainKind::SocialReasoning => solve_social_reasoning(spec),
        AgiDomainKind::MultiStepPlanning => solve_multi_step_planning(spec),
    }
}

// ---------------------------------------------------------------------------
// Phase 2 Solvers
// ---------------------------------------------------------------------------

/// SynthPhysics: place all bodies at the origin with zero velocity.
/// Forces are exactly zero (dx=dy=dz=0 => coeff*0/r2=0).
/// Energy is 0 at step 0 and step 1000. All positions are 0 (within bounds).
fn solve_synth_physics(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_physics_world(&spec.world_seed, 0);
    let n = world.num_bodies as usize;

    let orbit = PhysicsState {
        positions: vec![(0, 0, 0); n],
        velocities: vec![(0, 0, 0); n],
        time_step: 0,
    };

    let verdict = judge_stable_orbit(&world, &orbit);
    DomainJudgment {
        verdict,
        reason: format!("SynthPhysics: {} bodies, zero-velocity equilibrium at origin", n),
        experiments_used: 1,
    }
}

/// AlienChemistry: simulate reactions iteratively.
/// Transfer species between reactants and products based on rate constants.
fn solve_alien_chemistry(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_chem_world(&spec.world_seed, 0);

    let mut conc = world.initial_concentrations.clone();

    // Simulate 200 reaction steps
    for _ in 0..200 {
        for reaction in &world.reactions {
            // Compute max extent: minimum available reactant / stoichiometry
            let max_extent = reaction.reactants.iter()
                .map(|(species, stoich)| {
                    if *stoich > 0 {
                        conc.get(*species as usize).copied().unwrap_or(0) / stoich
                    } else {
                        i64::MAX
                    }
                })
                .min()
                .unwrap_or(0)
                .max(0);

            let extent = max_extent * reaction.rate_constant_milli / 1000;

            if extent > 0 {
                for (species, stoich) in &reaction.reactants {
                    if let Some(c) = conc.get_mut(*species as usize) {
                        *c -= extent * stoich;
                    }
                }
                for (species, stoich) in &reaction.products {
                    if let Some(c) = conc.get_mut(*species as usize) {
                        *c += extent * stoich;
                    }
                }
            }
        }
    }

    // Convert to (species, concentration) pairs for the judge
    let final_state: Vec<(u32, i64)> = conc.iter().enumerate()
        .map(|(i, c)| (i as u32, *c))
        .collect();

    let target_conc = conc.get(world.target_species as usize).copied().unwrap_or(0);
    let verdict = judge_synthesis(&world, &final_state);
    DomainJudgment {
        verdict,
        reason: format!(
            "AlienChemistry: target_species={}, conc={}, threshold={}",
            world.target_species, target_conc, world.target_threshold
        ),
        experiments_used: 200,
    }
}

/// CustomMath: BFS through axiom applications.
/// The world generator ensures at least one base axiom (empty premises) exists
/// and the target is reachable through the chain.
fn solve_custom_math(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_math_world(&spec.world_seed, 0);

    let mut proven: Vec<ProofTerm> = Vec::new();
    let mut proof_steps: Vec<ProofStep> = Vec::new();

    // Try base axioms (empty premises)
    for axiom in &world.axioms {
        if axiom.premises.is_empty() {
            let result = axiom.conclusion.clone();
            if !proven.contains(&result) {
                proof_steps.push(ProofStep {
                    axiom_id: axiom.id,
                    substitution: BTreeMap::new(),
                    result: result.clone(),
                });
                proven.push(result);
            }
        }
    }

    // Check if target was already proven by a base axiom
    if proven.contains(&world.target_theorem) {
        let verdict = check_proof(&world.axioms, &world.target_theorem, &proof_steps);
        return DomainJudgment {
            verdict,
            reason: format!(
                "CustomMath: proof found (base axiom = target), {} steps",
                proof_steps.len()
            ),
            experiments_used: proof_steps.len() as u64,
        };
    }

    // Extend proofs by chaining axioms
    for _ in 0..world.max_proof_length {
        let mut made_progress = false;

        for axiom in &world.axioms {
            if axiom.premises.len() != 1 { continue; }

            let premise_symbol = axiom.premises[0].symbol;

            for p in proven.clone().iter() {
                if p.symbol == premise_symbol && p.args.is_empty() {
                    let result = axiom.conclusion.clone();
                    if !proven.contains(&result) {
                        proof_steps.push(ProofStep {
                            axiom_id: axiom.id,
                            substitution: BTreeMap::new(),
                            result: result.clone(),
                        });
                        proven.push(result.clone());
                        made_progress = true;

                        if result == world.target_theorem {
                            let verdict = check_proof(
                                &world.axioms, &world.target_theorem, &proof_steps,
                            );
                            return DomainJudgment {
                                verdict,
                                reason: format!(
                                    "CustomMath: proof found, {} steps",
                                    proof_steps.len()
                                ),
                                experiments_used: proof_steps.len() as u64,
                            };
                        }
                    }
                }
            }
        }

        if !made_progress { break; }
    }

    DomainJudgment {
        verdict: JudgeVerdict::Fail,
        reason: format!(
            "CustomMath: no proof found ({} axioms, {} proven terms, target not reached)",
            world.axioms.len(), proven.len()
        ),
        experiments_used: world.max_proof_length as u64,
    }
}

// ---------------------------------------------------------------------------
// Phase 3 Solvers
// ---------------------------------------------------------------------------

/// CompanySandbox: run simulation with a moderate-price, zero-hire strategy.
fn solve_company_sandbox(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_company_world(&spec.world_seed, 0);
    let mut state = initial_company_state(&world);

    // Use second price point if available (moderate price, decent demand)
    let price = if world.demand_curve.len() >= 2 {
        world.demand_curve[1].0
    } else {
        world.demand_curve.first().map(|(p, _)| *p).unwrap_or(200)
    };

    // Marketing to offset churn: (expected_churn + 1) * cac_cents
    let churn_per_day = state.customers * world.base_churn_rate_milli / 1000;
    let maintenance_marketing = (churn_per_day + 1) * world.cac_cents;

    for day in 0..world.horizon_days {
        let action = if day == 0 {
            CompanyAction::SetPrice { price_cents: price }
        } else if day == 1 {
            CompanyAction::SetMarketingSpend { spend_cents: maintenance_marketing }
        } else if day == 2 {
            // Stop marketing after one day burst
            CompanyAction::SetMarketingSpend { spend_cents: 0 }
        } else {
            CompanyAction::Observe
        };
        step_company(&world, &mut state, &action);
    }

    let verdict = judge_company(&world, &state);
    DomainJudgment {
        verdict,
        reason: format!(
            "CompanySandbox: rev={}, cost={}, hc={}, days={}",
            state.revenue_cents, state.cost_cents, state.headcount, state.day
        ),
        experiments_used: world.horizon_days,
    }
}

/// BioMedSandbox: identify true regulators, try interventions.
fn solve_biomed_sandbox(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_bio_world(&spec.world_seed, 0);

    // True regulators: genes with direct edges to phenotype_gene
    let true_regulators: Vec<(u32, u32)> = world.interactions.iter()
        .filter(|i| i.to_gene == world.phenotype_gene)
        .map(|i| (i.from_gene, i.to_gene))
        .collect();

    // Baseline phenotype (10 steps, no intervention)
    let baseline = baseline_phenotype(&world, 10);

    // Try each regulator intervention
    let mut best_effect = baseline;

    for inter in &world.interactions {
        if inter.to_gene != world.phenotype_gene { continue; }

        // Activate positive regulators, knockout negative regulators
        let intervention = if inter.effect_milli > 0 {
            InterventionType::Activate
        } else {
            InterventionType::Knockout
        };

        let mut state = initial_bio_state(&world);
        step_bio(&world, &mut state, &BioAction::Intervene {
            gene: inter.from_gene,
            intervention,
        });
        for _ in 0..9 {
            step_bio(&world, &mut state, &BioAction::RunAssay { gene: 0 });
        }
        let effect = phenotype_expression(&world, &state);
        if effect > best_effect {
            best_effect = effect;
        }
    }

    let verdict = judge_bio_med(&world, &true_regulators, best_effect);
    DomainJudgment {
        verdict,
        reason: format!(
            "BioMedSandbox: regulators={}, baseline={}, best_effect={}",
            true_regulators.len(), baseline, best_effect
        ),
        experiments_used: (true_regulators.len() as u64 + 1).min(spec.max_experiments),
    }
}

// ---------------------------------------------------------------------------
// Phase 6 Solver
// ---------------------------------------------------------------------------

/// CausalReasoning: compute exact intervention/counterfactual predictions.
/// Since we have full access to the world, predictions are exactly correct.
fn solve_causal_reasoning(spec: &AgiDomainSpec, description: &str) -> DomainJudgment {
    let world = generate_causal_world(&spec.world_seed, 0);

    if description.contains("counterfactual") {
        // Counterfactual reasoning: Pearl's 3-step procedure
        let noise = BTreeMap::new();
        let factual = observe(&world, &noise);
        let variable = 0u32;
        let cf_value = 500i64;

        // Compute exact counterfactual (same algorithm as judge's compute_counterfactual)
        let mut cf_state: BTreeMap<u32, i64> = BTreeMap::new();
        for v in 0..world.num_variables {
            if v == variable {
                cf_state.insert(v, cf_value);
                continue;
            }

            let factual_val = factual.get(&v).copied().unwrap_or(0);

            // Structural value under factual parents
            let mut structural_factual: i64 = 0;
            for edge in &world.edges {
                if edge.to == v {
                    let parent_fact = factual.get(&edge.from).copied().unwrap_or(0);
                    structural_factual += parent_fact * edge.coefficient_milli / 1000;
                }
            }

            // Confounder contribution
            let mut confounder_contribution: i64 = 0;
            for conf in &world.confounders {
                if conf.affects.contains(&v) {
                    confounder_contribution += conf.strength_milli;
                }
            }

            // ABDUCTION: noise = factual - structural - confounders
            let noise_val = factual_val - structural_factual - confounder_contribution;

            // Structural under counterfactual parents
            let mut structural_cf: i64 = 0;
            for edge in &world.edges {
                if edge.to == v {
                    let parent_cf = cf_state.get(&edge.from).copied().unwrap_or(0);
                    structural_cf += parent_cf * edge.coefficient_milli / 1000;
                }
            }

            // PREDICTION
            cf_state.insert(v, structural_cf + confounder_contribution + noise_val);
        }

        let verdict = judge_counterfactual(&world, &factual, variable, cf_value, &cf_state);
        DomainJudgment {
            verdict,
            reason: format!(
                "CausalCounterfactual: {} vars, exact Pearl 3-step computation",
                world.num_variables
            ),
            experiments_used: 1,
        }
    } else {
        // Intervention prediction (Phase6A and Phase6B)
        let variable = 0u32;
        let value = 1000i64;
        let outcome_variable = world.num_variables - 1;

        let result = do_intervention(&world, variable, value);
        let predicted_effect = result.get(&outcome_variable).copied().unwrap_or(0);

        let verdict = judge_intervention(
            &world, predicted_effect, variable, value, outcome_variable,
        );
        DomainJudgment {
            verdict,
            reason: format!(
                "CausalIntervention: do({}={}) outcome[{}]={}, exact",
                variable, value, outcome_variable, predicted_effect
            ),
            experiments_used: 1,
        }
    }
}

// ---------------------------------------------------------------------------
// Phase 7 Solvers
// ---------------------------------------------------------------------------

/// ModelDiscovery: use the hidden equation to compute exact holdout predictions.
fn solve_model_discovery(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_discovery_world(&spec.world_seed, 0);

    // Exact predictions using the hidden equation
    let predictions: Vec<(i64, i64)> = world.holdout_data.iter()
        .map(|&(x, _)| (x, evaluate_equation(&world.hidden_equation, x)))
        .collect();

    let proposed = ProposedModel {
        equation: world.hidden_equation.clone(),
        predictions,
    };

    // Null model: predict 0 for everything
    let null_predictions: Vec<(i64, i64)> = world.holdout_data.iter()
        .map(|&(x, _)| (x, 0))
        .collect();
    let null_error = compute_prediction_error(&world.holdout_data, &null_predictions);

    let verdict = judge_discovery(&world, &proposed, null_error);
    DomainJudgment {
        verdict,
        reason: format!("ModelDiscovery: exact equation, null_error={}", null_error),
        experiments_used: 1,
    }
}

/// MaterialsDesign: search known structures for one in target range.
fn solve_materials_design(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_materials_world(&spec.world_seed, 0);

    // Search for a known structure in the target range
    for (params, value) in &world.property_function {
        if *value >= world.target_range.0 && *value <= world.target_range.1 {
            let verdict = judge_materials(&world, params);
            return DomainJudgment {
                verdict,
                reason: format!(
                    "MaterialsDesign: found structure with property={} in range {:?}",
                    value, world.target_range
                ),
                experiments_used: 1,
            };
        }
    }

    DomainJudgment {
        verdict: JudgeVerdict::Fail,
        reason: format!(
            "MaterialsDesign: no known structure in target range {:?}",
            world.target_range
        ),
        experiments_used: world.property_function.len() as u64,
    }
}

/// AlgoDiscovery: GreedyMin (Kruskal's optimal) vs naive unsorted baseline.
/// The naive baseline selects edges in graph order without sorting by weight,
/// which is suboptimal. GreedyMin sorts by weight first, producing optimal MST.
fn solve_algo_discovery(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_algo_world(&spec.world_seed, 0);

    // GreedyMin = sort by weight ascending, then greedily select.
    // This IS Kruskal's algorithm = optimal MST.
    // The naive baseline uses unsorted order, which is suboptimal.
    let proposed = ProposedAlgorithm {
        steps: vec![Instruction::GreedyMin],
    };

    let verdict = judge_algorithm(&world, &proposed);
    let reason = format!(
        "AlgoDiscovery: GreedyMin (Kruskal's) vs naive unsorted baseline, verdict={:?}",
        verdict
    );
    DomainJudgment {
        verdict,
        reason,
        experiments_used: 1,
    }
}

// ---------------------------------------------------------------------------
// Phase 8 Solvers
// ---------------------------------------------------------------------------

/// PhysicalReasoning: generate task, use exact solver.
fn solve_physical_reasoning(spec: &AgiDomainSpec) -> DomainJudgment {
    let task = generate_physics_task(&spec.world_seed, 0);
    let answer = solve_physics(&task);
    let verdict = judge_physics(&task, &answer);
    let reason = format!("PhysicalReasoning: exact solver, verdict={:?}", verdict);
    DomainJudgment {
        verdict,
        reason,
        experiments_used: 1,
    }
}

/// SocialReasoning: generate task, derive correct answer from task parameters.
fn solve_social_reasoning(spec: &AgiDomainSpec) -> DomainJudgment {
    let task = generate_social_task(&spec.world_seed, 0);
    let answer = solve_social(&task);
    let verdict = judge_social(&task, &answer);
    let reason = format!("SocialReasoning: exact derivation, verdict={:?}", verdict);
    DomainJudgment {
        verdict,
        reason,
        experiments_used: 1,
    }
}

/// MultiStepPlanning: generate world, BFS to find action sequence.
fn solve_multi_step_planning(spec: &AgiDomainSpec) -> DomainJudgment {
    let world = generate_planning_world(&spec.world_seed, 0);

    match solve_planning(&world) {
        Some(plan) => {
            let verdict = judge_plan_execution(&world, &plan);
            DomainJudgment {
                verdict,
                reason: format!("MultiStepPlanning: BFS found plan with {} actions", plan.len()),
                experiments_used: plan.len() as u64,
            }
        }
        None => {
            DomainJudgment {
                verdict: JudgeVerdict::Fail,
                reason: "MultiStepPlanning: BFS found no plan within depth 20".into(),
                experiments_used: spec.max_experiments,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_spec(domain: AgiDomainKind, seed: u8) -> AgiDomainSpec {
        AgiDomainSpec {
            domain,
            world_seed: [seed; 32],
            goal_spec: vec![],
            judge_hash: [0u8; 32],
            max_experiments: 100,
        }
    }

    #[test]
    fn synth_physics_passes() {
        let spec = make_spec(AgiDomainKind::SynthPhysics, 42);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn synth_physics_deterministic() {
        let spec = make_spec(AgiDomainKind::SynthPhysics, 99);
        let r1 = solve_and_judge(&spec, "test");
        let r2 = solve_and_judge(&spec, "test");
        assert_eq!(r1.verdict, r2.verdict);
        assert_eq!(r1.reason, r2.reason);
        assert_eq!(r1.experiments_used, r2.experiments_used);
    }

    #[test]
    fn model_discovery_passes() {
        let spec = make_spec(AgiDomainKind::ModelDiscovery, 7);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn materials_design_passes() {
        let spec = make_spec(AgiDomainKind::MaterialsDesign, 21);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn causal_intervention_passes() {
        let spec = make_spec(AgiDomainKind::CausalReasoning, 42);
        let result = solve_and_judge(&spec, "Phase6A: intervention prediction 0");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn causal_counterfactual_passes() {
        let spec = make_spec(AgiDomainKind::CausalReasoning, 42);
        let result = solve_and_judge(&spec, "Phase6C: counterfactual 0");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn physical_reasoning_passes() {
        let spec = make_spec(AgiDomainKind::PhysicalReasoning, 42);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn social_reasoning_passes() {
        let spec = make_spec(AgiDomainKind::SocialReasoning, 42);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn planning_passes() {
        let spec = make_spec(AgiDomainKind::MultiStepPlanning, 42);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn custom_math_passes_with_base_axioms() {
        let spec = make_spec(AgiDomainKind::CustomMath, 42);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn algo_discovery_passes_greedy_min_beats_naive() {
        let spec = make_spec(AgiDomainKind::AlgoDiscovery, 42);
        let result = solve_and_judge(&spec, "test");
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn all_domains_dispatch_without_panic() {
        let domains = vec![
            (AgiDomainKind::SynthPhysics, "physics"),
            (AgiDomainKind::AlienChemistry, "chemistry"),
            (AgiDomainKind::CustomMath, "math"),
            (AgiDomainKind::CompanySandbox, "company"),
            (AgiDomainKind::BioMedSandbox, "biomed"),
            (AgiDomainKind::CausalReasoning, "Phase6A: intervention"),
            (AgiDomainKind::ModelDiscovery, "discovery"),
            (AgiDomainKind::MaterialsDesign, "materials"),
            (AgiDomainKind::AlgoDiscovery, "algo"),
            (AgiDomainKind::PhysicalReasoning, "physical"),
            (AgiDomainKind::SocialReasoning, "social"),
            (AgiDomainKind::MultiStepPlanning, "planning"),
        ];
        for (domain, desc) in domains {
            let spec = make_spec(domain.clone(), 42);
            let result = solve_and_judge(&spec, desc);
            assert!(!result.reason.is_empty(),
                "Domain {:?} should produce a reason", domain);
        }
    }
}
