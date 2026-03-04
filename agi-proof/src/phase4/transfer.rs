// Phase 4: Genuine Transfer Learning
//
// Proves that knowledge learned in domain A transfers to domain B:
//   - 30 deterministic transfer pairs across conservation/graph/proof categories
//   - Transfer gain computed as (score_after - score_cold) / score_cold
//   - Order effect: score_after_a != score_before_a (direction matters)
//   - All arithmetic is integer (i64/u64), zero floats.

use kernel_types::hash;
use kernel_bench::judge::JudgeVerdict;
use crate::eval_specs::AgiDomainKind;
use serde::{Serialize, Deserialize};

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A pair of domains linked by a shared principle that should enable transfer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferPair {
    pub pair_id: String,
    pub domain_a: AgiDomainKind,
    pub domain_b: AgiDomainKind,
    pub shared_principle: String,
    /// Deterministic seeds derived from the master seed for each domain.
    pub seeds: TransferSeeds,
}

/// Seeds for the two domains in a transfer pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferSeeds {
    pub world_a_seed: [u8; 32],
    pub world_b_seed: [u8; 32],
}

/// Result of measuring transfer between a pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferResult {
    pub pair_id: String,
    /// Score on domain B with no prior exposure (cold start).
    pub score_b_cold: i64,
    /// Score on domain B after training on domain A (A then B).
    pub score_b_after_a: i64,
    /// Score on domain B measured before training on domain A (B then A).
    pub score_b_before_a: i64,
    /// Transfer gain = (score_b_after_a - score_b_cold) / score_b_cold
    /// Represented as numerator / denominator to stay in integer land.
    pub transfer_gain_num: i64,
    pub transfer_gain_den: u64,
    /// True iff the order of exposure matters: score_b_after_a != score_b_before_a.
    pub order_effect: bool,
}

// ---------------------------------------------------------------------------
// Category descriptors
// ---------------------------------------------------------------------------

/// The three transfer categories and the principles within each.
/// Category 0: Conservation (physics <-> chemistry energy/mass conservation)
/// Category 1: Graph (causal reasoning <-> algo discovery graph structure)
/// Category 2: Proof (custom math <-> model discovery formal reasoning)

struct PairTemplate {
    domain_a: AgiDomainKind,
    domain_b: AgiDomainKind,
    principle: &'static str,
}

/// 10 templates per category = 30 total pairs.
fn pair_templates() -> Vec<PairTemplate> {
    let mut templates = Vec::with_capacity(30);

    // Category 0: Conservation (10 pairs)
    // SynthPhysics <-> AlienChemistry — conservation laws transfer
    let conservation_principles: [&str; 10] = [
        "energy_conservation",
        "mass_conservation",
        "momentum_invariant",
        "symmetry_noether",
        "equilibrium_balance",
        "conservation_counting",
        "stoichiometric_balance",
        "flux_continuity",
        "potential_minimum",
        "constraint_propagation",
    ];
    for p in &conservation_principles {
        templates.push(PairTemplate {
            domain_a: AgiDomainKind::SynthPhysics,
            domain_b: AgiDomainKind::AlienChemistry,
            principle: p,
        });
    }

    // Category 1: Graph (10 pairs)
    // CausalReasoning <-> AlgoDiscovery — graph structural reasoning
    let graph_principles: [&str; 10] = [
        "topological_sort",
        "reachability_closure",
        "cycle_detection",
        "path_optimization",
        "subgraph_isomorphism",
        "edge_contraction",
        "degree_distribution",
        "connected_components",
        "spanning_tree",
        "graph_coloring",
    ];
    for p in &graph_principles {
        templates.push(PairTemplate {
            domain_a: AgiDomainKind::CausalReasoning,
            domain_b: AgiDomainKind::AlgoDiscovery,
            principle: p,
        });
    }

    // Category 2: Proof (10 pairs)
    // CustomMath <-> ModelDiscovery — formal reasoning transfer
    let proof_principles: [&str; 10] = [
        "axiom_chaining",
        "substitution_unification",
        "proof_by_contradiction",
        "inductive_step",
        "lemma_reuse",
        "term_rewriting",
        "structural_induction",
        "case_analysis",
        "fixed_point_iteration",
        "abstraction_refinement",
    ];
    for p in &proof_principles {
        templates.push(PairTemplate {
            domain_a: AgiDomainKind::CustomMath,
            domain_b: AgiDomainKind::ModelDiscovery,
            principle: p,
        });
    }

    templates
}

// ---------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------

/// Generate 30 transfer pairs deterministically from a master seed.
///
/// Each pair gets its own world_a_seed and world_b_seed derived via:
///   world_a_seed = H(master_seed || pair_index_le || b"domain_a")
///   world_b_seed = H(master_seed || pair_index_le || b"domain_b")
pub fn generate_transfer_pairs(seed: &[u8; 32]) -> Vec<TransferPair> {
    let templates = pair_templates();
    let mut pairs = Vec::with_capacity(30);

    for (idx, tmpl) in templates.iter().enumerate() {
        let idx_bytes = (idx as u32).to_le_bytes();

        let mut buf_a = Vec::with_capacity(32 + 4 + 8);
        buf_a.extend_from_slice(seed);
        buf_a.extend_from_slice(&idx_bytes);
        buf_a.extend_from_slice(b"domain_a");
        let world_a_seed = hash::H(&buf_a);

        let mut buf_b = Vec::with_capacity(32 + 4 + 8);
        buf_b.extend_from_slice(seed);
        buf_b.extend_from_slice(&idx_bytes);
        buf_b.extend_from_slice(b"domain_b");
        let world_b_seed = hash::H(&buf_b);

        let category = idx / 10;
        let within = idx % 10;
        let pair_id = format!("TP-{}-{}", category, within);

        pairs.push(TransferPair {
            pair_id,
            domain_a: tmpl.domain_a.clone(),
            domain_b: tmpl.domain_b.clone(),
            shared_principle: tmpl.principle.to_string(),
            seeds: TransferSeeds {
                world_a_seed,
                world_b_seed,
            },
        });
    }

    pairs
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Compute transfer result from the three raw scores.
///
/// transfer_gain = (score_b_after_a - score_b_cold) / |score_b_cold|
///   numerator   = score_b_after_a - score_b_cold
///   denominator = |score_b_cold|  (clamped to >= 1 to avoid division by zero)
///
/// order_effect = score_b_after_a != score_b_before_a
pub fn compute_transfer_result(
    pair_id: &str,
    score_b_cold: i64,
    score_b_after_a: i64,
    score_b_before_a: i64,
) -> TransferResult {
    let gain_num = score_b_after_a - score_b_cold;
    // Denominator is |score_b_cold|, clamped to >= 1 so we never divide by zero.
    let gain_den = if score_b_cold == 0 { 1u64 } else { score_b_cold.unsigned_abs() };
    let order_effect = score_b_after_a != score_b_before_a;

    TransferResult {
        pair_id: pair_id.to_string(),
        score_b_cold,
        score_b_after_a,
        score_b_before_a,
        transfer_gain_num: gain_num,
        transfer_gain_den: gain_den,
        order_effect,
    }
}

// ---------------------------------------------------------------------------
// Judge
// ---------------------------------------------------------------------------

/// PASS iff:
///   1. transfer_gain >= 30%  (gain_num / gain_den >= 30/100)
///      i.e. gain_num * 100 >= 30 * gain_den  (cross-multiply, safe for i64)
///   2. order_effect == true
pub fn judge_transfer(result: &TransferResult) -> JudgeVerdict {
    let gain_threshold_num: i64 = 30;
    let gain_threshold_den: u64 = 100;

    // Cross-multiply: gain_num * threshold_den >= threshold_num * gain_den
    // gain_num can be negative (no transfer), in which case this fails.
    let gain_sufficient = result.transfer_gain_num * gain_threshold_den as i64
        >= gain_threshold_num * result.transfer_gain_den as i64;

    if gain_sufficient && result.order_effect {
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
    fn transfer_pair_deterministic() {
        let seed = [42u8; 32];
        let pairs_a = generate_transfer_pairs(&seed);
        let pairs_b = generate_transfer_pairs(&seed);
        assert_eq!(pairs_a.len(), 30);
        assert_eq!(pairs_b.len(), 30);
        for (a, b) in pairs_a.iter().zip(pairs_b.iter()) {
            assert_eq!(a.pair_id, b.pair_id);
            assert_eq!(a.domain_a, b.domain_a);
            assert_eq!(a.domain_b, b.domain_b);
            assert_eq!(a.shared_principle, b.shared_principle);
            assert_eq!(a.seeds.world_a_seed, b.seeds.world_a_seed);
            assert_eq!(a.seeds.world_b_seed, b.seeds.world_b_seed);
        }
    }

    #[test]
    fn transfer_pair_categories_correct() {
        let seed = [1u8; 32];
        let pairs = generate_transfer_pairs(&seed);
        // First 10: conservation (SynthPhysics -> AlienChemistry)
        for p in &pairs[0..10] {
            assert_eq!(p.domain_a, AgiDomainKind::SynthPhysics);
            assert_eq!(p.domain_b, AgiDomainKind::AlienChemistry);
        }
        // Next 10: graph (CausalReasoning -> AlgoDiscovery)
        for p in &pairs[10..20] {
            assert_eq!(p.domain_a, AgiDomainKind::CausalReasoning);
            assert_eq!(p.domain_b, AgiDomainKind::AlgoDiscovery);
        }
        // Last 10: proof (CustomMath -> ModelDiscovery)
        for p in &pairs[20..30] {
            assert_eq!(p.domain_a, AgiDomainKind::CustomMath);
            assert_eq!(p.domain_b, AgiDomainKind::ModelDiscovery);
        }
    }

    #[test]
    fn transfer_pair_seeds_differ_per_pair() {
        let seed = [7u8; 32];
        let pairs = generate_transfer_pairs(&seed);
        // All world_a_seeds should be distinct
        for i in 0..pairs.len() {
            for j in (i + 1)..pairs.len() {
                assert_ne!(
                    pairs[i].seeds.world_a_seed,
                    pairs[j].seeds.world_a_seed,
                    "pair {} and {} have same world_a_seed",
                    i, j
                );
            }
        }
    }

    #[test]
    fn transfer_gain_computed_correctly() {
        // cold=100, after_a=150, before_a=110
        // gain = (150-100)/100 = 50/100 = 50%
        let result = compute_transfer_result("TP-0-0", 100, 150, 110);
        assert_eq!(result.transfer_gain_num, 50);
        assert_eq!(result.transfer_gain_den, 100);
        assert!(result.order_effect);

        // Verify cross-multiply: 50 * 100 >= 30 * 100 => 5000 >= 3000 => true
        assert_eq!(judge_transfer(&result), JudgeVerdict::Pass);
    }

    #[test]
    fn transfer_gain_negative_fails() {
        // cold=100, after_a=80, before_a=70  -> negative transfer
        let result = compute_transfer_result("TP-0-1", 100, 80, 70);
        assert_eq!(result.transfer_gain_num, -20);
        assert_eq!(result.transfer_gain_den, 100);
        assert!(result.order_effect);
        assert_eq!(judge_transfer(&result), JudgeVerdict::Fail);
    }

    #[test]
    fn transfer_gain_zero_cold_uses_den_one() {
        // cold=0, after_a=50, before_a=30
        let result = compute_transfer_result("TP-0-2", 0, 50, 30);
        assert_eq!(result.transfer_gain_num, 50);
        assert_eq!(result.transfer_gain_den, 1);
        assert!(result.order_effect);
        // 50 * 100 >= 30 * 1 => 5000 >= 30 => pass
        assert_eq!(judge_transfer(&result), JudgeVerdict::Pass);
    }

    #[test]
    fn order_effect_detected() {
        // after_a=150, before_a=120 => order_effect=true (they differ)
        let result = compute_transfer_result("TP-1-0", 100, 150, 120);
        assert!(result.order_effect);

        // after_a=150, before_a=150 => order_effect=false (no directional effect)
        let result_no = compute_transfer_result("TP-1-1", 100, 150, 150);
        assert!(!result_no.order_effect);
    }

    #[test]
    fn judge_transfer_pass_on_sufficient_gain() {
        // Exactly 30%: gain_num=30, gain_den=100
        // cross-multiply: 30 * 100 >= 30 * 100 => 3000 >= 3000 => true
        let result = TransferResult {
            pair_id: "TP-2-0".to_string(),
            score_b_cold: 100,
            score_b_after_a: 130,
            score_b_before_a: 110,
            transfer_gain_num: 30,
            transfer_gain_den: 100,
            order_effect: true,
        };
        assert_eq!(judge_transfer(&result), JudgeVerdict::Pass);
    }

    #[test]
    fn judge_transfer_fail_insufficient_gain() {
        // 29% gain: gain_num=29, gain_den=100
        // cross-multiply: 29 * 100 >= 30 * 100 => 2900 >= 3000 => false
        let result = TransferResult {
            pair_id: "TP-2-1".to_string(),
            score_b_cold: 100,
            score_b_after_a: 129,
            score_b_before_a: 110,
            transfer_gain_num: 29,
            transfer_gain_den: 100,
            order_effect: true,
        };
        assert_eq!(judge_transfer(&result), JudgeVerdict::Fail);
    }

    #[test]
    fn judge_transfer_fail_no_order_effect() {
        // 50% gain but no order effect
        let result = TransferResult {
            pair_id: "TP-2-2".to_string(),
            score_b_cold: 100,
            score_b_after_a: 150,
            score_b_before_a: 150,
            transfer_gain_num: 50,
            transfer_gain_den: 100,
            order_effect: false,
        };
        assert_eq!(judge_transfer(&result), JudgeVerdict::Fail);
    }

    #[test]
    fn different_seed_different_pairs() {
        let pairs_a = generate_transfer_pairs(&[1u8; 32]);
        let pairs_b = generate_transfer_pairs(&[2u8; 32]);
        // Same structure but different seeds
        assert_eq!(pairs_a[0].pair_id, pairs_b[0].pair_id);
        assert_ne!(pairs_a[0].seeds.world_a_seed, pairs_b[0].seeds.world_a_seed);
    }
}
