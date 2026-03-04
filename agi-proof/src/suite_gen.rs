// Suite Generator
//
// Creates all 1,106 AGI proof task specifications from seeds.
// Every task is a JSON string ready for compile_agi_contract().
// All generation is deterministic: same seed → same suite → same hashes.
//
// Task counts per EXECUTION.md:
//   Phase 0: 1 (freeze check)
//   Phase 1: 215 (existing kernel tests, not generated here)
//   Phase 2: 50 physics + 50 chemistry + 100 math = 200
//   Phase 3: 50 company + 50 biomed = 100
//   Phase 4: 30 transfer pairs × 3 runs = 90
//   Phase 5: 50 acquisition = 50
//   Phase 6: 50 × 3 types = 150
//   Phase 7: 50 × 3 types = 150
//   Phase 8: 50 × 3 types = 150
//   Total generated: 891 (+ 215 existing = 1,106)

use kernel_types::hash;
use kernel_types::Hash32;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// The complete AGI proof suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiSuite {
    /// Master seed for all task generation.
    pub master_seed: [u8; 32],
    /// Commitment hash: H(master_seed), published before generation.
    pub seed_commitment: Hash32,
    /// Phase → task JSONs.
    pub phases: BTreeMap<u8, Vec<String>>,
    /// Total task count.
    pub total_tasks: usize,
    /// Suite Merkle root: MerkleRoot(H(task_json_i)).
    pub suite_merkle_root: Hash32,
}

/// Generate the complete AGI proof suite from a master seed.
///
/// The seed is committed (H(seed) published) before generation,
/// ensuring no task can be seen before freeze.
pub fn generate_suite(master_seed: [u8; 32]) -> AgiSuite {
    let seed_commitment = hash::H(&master_seed);
    let mut phases: BTreeMap<u8, Vec<String>> = BTreeMap::new();
    let mut all_task_hashes: Vec<Hash32> = Vec::new();

    // Phase 0: freeze check (1 task)
    let phase0 = vec![make_freeze_task(&master_seed)];
    hash_tasks(&phase0, &mut all_task_hashes);
    phases.insert(0, phase0);

    // Phase 2: domain robustness (200 tasks)
    let phase2 = generate_phase2(&master_seed);
    hash_tasks(&phase2, &mut all_task_hashes);
    phases.insert(2, phase2);

    // Phase 3: long horizon (100 tasks)
    let phase3 = generate_phase3(&master_seed);
    hash_tasks(&phase3, &mut all_task_hashes);
    phases.insert(3, phase3);

    // Phase 4: transfer (90 tasks)
    let phase4 = generate_phase4(&master_seed);
    hash_tasks(&phase4, &mut all_task_hashes);
    phases.insert(4, phase4);

    // Phase 5: knowledge acquisition (50 tasks)
    let phase5 = generate_phase5(&master_seed);
    hash_tasks(&phase5, &mut all_task_hashes);
    phases.insert(5, phase5);

    // Phase 6: causal reasoning (150 tasks)
    let phase6 = generate_phase6(&master_seed);
    hash_tasks(&phase6, &mut all_task_hashes);
    phases.insert(6, phase6);

    // Phase 7: discovery (150 tasks)
    let phase7 = generate_phase7(&master_seed);
    hash_tasks(&phase7, &mut all_task_hashes);
    phases.insert(7, phase7);

    // Phase 8: common sense (150 tasks)
    let phase8 = generate_phase8(&master_seed);
    hash_tasks(&phase8, &mut all_task_hashes);
    phases.insert(8, phase8);

    let total_tasks = all_task_hashes.len();
    let suite_merkle_root = hash::merkle_root(&all_task_hashes);

    AgiSuite {
        master_seed,
        seed_commitment,
        phases,
        total_tasks,
        suite_merkle_root,
    }
}

/// Derive an episode seed from master seed + phase + episode index.
fn derive_seed(master: &[u8; 32], phase: u8, episode: u32) -> String {
    let mut buf = Vec::new();
    buf.extend_from_slice(master);
    buf.push(phase);
    buf.extend_from_slice(&episode.to_le_bytes());
    let h = hash::H(&buf);
    hash::hex(&h)
}

fn hash_tasks(tasks: &[String], out: &mut Vec<Hash32>) {
    for t in tasks {
        out.push(hash::H(t.as_bytes()));
    }
}

// ---------------------------------------------------------------------------
// Phase 0: Freeze
// ---------------------------------------------------------------------------

fn make_freeze_task(master_seed: &[u8; 32]) -> String {
    let seed_hex = hash::hex(&hash::H(master_seed));
    format!(
        r#"{{"type":"agi_domain","domain":"SynthPhysics","description":"Phase0: freeze check (seed commitment={})","world_seed":"{}","max_experiments":1}}"#,
        &seed_hex[..16],
        derive_seed(master_seed, 0, 0)
    )
}

// ---------------------------------------------------------------------------
// Phase 2: Domain Robustness (200 tasks)
// ---------------------------------------------------------------------------

fn generate_phase2(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(200);

    // 50 physics episodes
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 2, ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"SynthPhysics","description":"Phase2A: physics episode {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    // 50 chemistry episodes
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 2, 100 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"AlienChemistry","description":"Phase2B: chemistry episode {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    // 100 math episodes
    for ep in 0..100u32 {
        let seed = derive_seed(master_seed, 2, 200 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"CustomMath","description":"Phase2C: math episode {}","world_seed":"{}","max_experiments":50}}"#,
            ep, seed
        ));
    }

    tasks
}

// ---------------------------------------------------------------------------
// Phase 3: Long Horizon (100 tasks)
// ---------------------------------------------------------------------------

fn generate_phase3(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(100);

    // 50 company sandbox episodes
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 3, ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"CompanySandbox","description":"Phase3A: company episode {}","world_seed":"{}","max_experiments":200}}"#,
            ep, seed
        ));
    }

    // 50 biomed sandbox episodes
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 3, 100 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"BioMedSandbox","description":"Phase3B: biomed episode {}","world_seed":"{}","max_experiments":200}}"#,
            ep, seed
        ));
    }

    tasks
}

// ---------------------------------------------------------------------------
// Phase 4: Transfer Learning (90 tasks)
// ---------------------------------------------------------------------------

fn generate_phase4(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(90);

    // 30 transfer pairs × 3 runs each (cold, A-then-B, B-then-A)
    let categories = ["conservation", "graph", "proof"];
    for (cat_idx, category) in categories.iter().enumerate() {
        for pair in 0..10u32 {
            for run in 0..3u32 {
                let ep = (cat_idx as u32) * 30 + pair * 3 + run;
                let seed = derive_seed(master_seed, 4, ep);
                let run_type = match run {
                    0 => "cold",
                    1 => "A_then_B",
                    _ => "B_then_A",
                };
                tasks.push(format!(
                    r#"{{"type":"agi_domain","domain":"SynthPhysics","description":"Phase4: transfer {} pair {} run {}","world_seed":"{}","max_experiments":100,"goal_spec":"transfer_{}_{}_{}"}}"#,
                    category, pair, run_type, seed, category, pair, run_type
                ));
            }
        }
    }

    tasks
}

// ---------------------------------------------------------------------------
// Phase 5: Knowledge Acquisition (50 tasks)
// ---------------------------------------------------------------------------

fn generate_phase5(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(50);

    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 5, ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"SynthPhysics","description":"Phase5: acquisition episode {}","world_seed":"{}","max_experiments":50}}"#,
            ep, seed
        ));
    }

    tasks
}

// ---------------------------------------------------------------------------
// Phase 6: Causal Reasoning (150 tasks)
// ---------------------------------------------------------------------------

fn generate_phase6(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(150);

    // 50 intervention prediction tasks
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 6, ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"CausalReasoning","description":"Phase6A: intervention prediction {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    // 50 optimal intervention selection tasks
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 6, 100 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"CausalReasoning","description":"Phase6B: optimal intervention {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    // 50 counterfactual explanation tasks
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 6, 200 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"CausalReasoning","description":"Phase6C: counterfactual {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    tasks
}

// ---------------------------------------------------------------------------
// Phase 7: Discovery (150 tasks)
// ---------------------------------------------------------------------------

fn generate_phase7(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(150);

    // 50 model discovery
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 7, ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"ModelDiscovery","description":"Phase7A: model discovery {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    // 50 materials design
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 7, 100 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"MaterialsDesign","description":"Phase7B: materials design {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    // 50 algorithm discovery
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 7, 200 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"AlgoDiscovery","description":"Phase7C: algo discovery {}","world_seed":"{}","max_experiments":100}}"#,
            ep, seed
        ));
    }

    tasks
}

// ---------------------------------------------------------------------------
// Phase 8: Common Sense (150 tasks)
// ---------------------------------------------------------------------------

fn generate_phase8(master_seed: &[u8; 32]) -> Vec<String> {
    let mut tasks = Vec::with_capacity(150);

    // 50 physical reasoning
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 8, ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"PhysicalReasoning","description":"Phase8A: physical reasoning {}","world_seed":"{}","max_experiments":10}}"#,
            ep, seed
        ));
    }

    // 50 social reasoning
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 8, 100 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"SocialReasoning","description":"Phase8B: social reasoning {}","world_seed":"{}","max_experiments":10}}"#,
            ep, seed
        ));
    }

    // 50 multi-step planning
    for ep in 0..50u32 {
        let seed = derive_seed(master_seed, 8, 200 + ep);
        tasks.push(format!(
            r#"{{"type":"agi_domain","domain":"MultiStepPlanning","description":"Phase8C: planning {}","world_seed":"{}","max_experiments":20}}"#,
            ep, seed
        ));
    }

    tasks
}

/// Serialize the suite to JSON.
pub fn suite_to_json(suite: &AgiSuite) -> String {
    serde_json::to_string_pretty(suite).expect("suite serialization must succeed")
}

/// Compute the suite manifest for signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuiteManifest {
    pub seed_commitment: String,
    pub total_tasks: usize,
    pub suite_merkle_root: String,
    pub phase_counts: BTreeMap<u8, usize>,
}

pub fn build_manifest(suite: &AgiSuite) -> SuiteManifest {
    let mut phase_counts = BTreeMap::new();
    for (phase, tasks) in &suite.phases {
        phase_counts.insert(*phase, tasks.len());
    }
    SuiteManifest {
        seed_commitment: hash::hex(&suite.seed_commitment),
        total_tasks: suite.total_tasks,
        suite_merkle_root: hash::hex(&suite.suite_merkle_root),
        phase_counts,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_deterministic() {
        let seed = [42u8; 32];
        let s1 = generate_suite(seed);
        let s2 = generate_suite(seed);
        assert_eq!(s1.total_tasks, s2.total_tasks);
        assert_eq!(s1.suite_merkle_root, s2.suite_merkle_root);
        assert_eq!(s1.seed_commitment, s2.seed_commitment);
    }

    #[test]
    fn suite_different_seed_different_suite() {
        let s1 = generate_suite([1u8; 32]);
        let s2 = generate_suite([2u8; 32]);
        assert_ne!(s1.suite_merkle_root, s2.suite_merkle_root);
    }

    #[test]
    fn suite_total_task_count() {
        let suite = generate_suite([99u8; 32]);
        // Phase 0: 1, Phase 2: 200, Phase 3: 100, Phase 4: 90,
        // Phase 5: 50, Phase 6: 150, Phase 7: 150, Phase 8: 150
        // Total = 891
        assert_eq!(suite.total_tasks, 891);
    }

    #[test]
    fn suite_phase_counts_correct() {
        let suite = generate_suite([77u8; 32]);
        assert_eq!(suite.phases[&0].len(), 1);
        assert_eq!(suite.phases[&2].len(), 200);
        assert_eq!(suite.phases[&3].len(), 100);
        assert_eq!(suite.phases[&4].len(), 90);
        assert_eq!(suite.phases[&5].len(), 50);
        assert_eq!(suite.phases[&6].len(), 150);
        assert_eq!(suite.phases[&7].len(), 150);
        assert_eq!(suite.phases[&8].len(), 150);
    }

    #[test]
    fn suite_commitment_is_hash_of_seed() {
        let seed = [55u8; 32];
        let suite = generate_suite(seed);
        assert_eq!(suite.seed_commitment, hash::H(&seed));
    }

    #[test]
    fn suite_all_tasks_parse_as_json() {
        let suite = generate_suite([33u8; 32]);
        for (phase, tasks) in &suite.phases {
            for (i, task) in tasks.iter().enumerate() {
                let parsed: Result<serde_json::Value, _> = serde_json::from_str(task);
                assert!(parsed.is_ok(), "Phase {} task {} failed JSON parse: {}", phase, i, task);
            }
        }
    }

    #[test]
    fn suite_all_tasks_compilable() {
        use crate::compiler_ext::compile_agi_contract;
        let suite = generate_suite([44u8; 32]);
        for (phase, tasks) in &suite.phases {
            for (i, task) in tasks.iter().enumerate() {
                let result = compile_agi_contract(task);
                assert!(result.is_ok(), "Phase {} task {} failed compile: {:?}", phase, i, result.err());
            }
        }
    }

    #[test]
    fn suite_manifest_builds() {
        let suite = generate_suite([11u8; 32]);
        let manifest = build_manifest(&suite);
        assert_eq!(manifest.total_tasks, 891);
        assert_eq!(manifest.phase_counts[&2], 200);
    }

    #[test]
    fn suite_merkle_root_nonzero() {
        let suite = generate_suite([88u8; 32]);
        assert_ne!(suite.suite_merkle_root, [0u8; 32]);
    }
}
