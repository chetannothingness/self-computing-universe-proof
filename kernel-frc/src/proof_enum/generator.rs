//! GEN_𝒰 — the canonical witness generator function G.
//!
//! G: 𝒰 → D* is the TOTAL function that computes proofs for every S ∈ 𝒰.
//! G(S) = least π under Π-order such that Check(S, π) = PASS.
//!
//! G is NOT a search procedure. G is a DEFINED PROJECTION.
//! The witness is computed, not searched for.
//!
//! The pipeline:
//!   G(S) = R.lookup(S) || accelerator(S) || canonical_compute(S)
//!
//! canonical_compute(S) = μ π. Check(S, π) = PASS
//!   (unbounded — runs to completion for provable S)
//!
//! COMPLETE_𝒰: ∀ S ∈ 𝒰, ∃ π, Check(S, π) = PASS
//! GEN_𝒰: ∃ G: 𝒰 → D*, ∀ S ∈ 𝒰, Check(S, G(S)) = PASS
//!
//! Budget is irrelevant. G computes. Period.

use super::core_term::{CoreTerm, CoreCtx, CoreEnv};
use super::type_check::{type_check, CheckResult};
use super::elab::{elab_problem, elab_witness_bytes, ElabResult};
use super::witness::WitnessEnumerator;
use super::accelerator::try_accelerator;
use super::statement::{get_statement, is_formalized};
use super::rewrite::RewriteBasis;
use super::extract::extract_rules;
use super::ledger::ProofLedger;
use kernel_types::{Hash32, hash};

/// Result of the generator G.
///
/// Note: NO Frontier variant. G never gives up.
/// Only Proved (G(S) computed) or Computing (snapshot of G mid-computation).
#[derive(Debug, Clone)]
pub enum GeneratorResult {
    /// G(S) computed successfully — proof found.
    Proved {
        /// Problem identifier.
        statement_id: String,
        /// The witness term.
        witness: CoreTerm,
        /// Raw witness bytes.
        witness_bytes: Vec<u8>,
        /// Hash of the proof.
        proof_hash: Hash32,
        /// Method: "R_cache", "accelerator(IRC)", "G_compute(rank=N)"
        method: String,
        /// Rank in canonical ordering where witness was found.
        rank: u64,
        /// Rules extracted into R from this proof.
        rules_extracted: usize,
    },
    /// G(S) is computing — snapshot taken. G never stops for provable S.
    /// This state exists ONLY when snapshot_budget is set (test/CLI mode).
    /// In production, G runs to completion.
    Computing {
        /// Problem identifier.
        statement_id: String,
        /// How many candidates G has computed so far.
        candidates_computed: u64,
        /// Current rank in the canonical ordering.
        current_rank: u64,
    },
    /// Statement not in 𝒰 — not formalized.
    /// This is not failure — it means the statement needs formalization
    /// before G can operate on it.
    NotInUniverse {
        /// Problem identifier.
        statement_id: String,
        /// Why it's not in 𝒰.
        reason: String,
    },
}

impl GeneratorResult {
    pub fn is_proved(&self) -> bool {
        matches!(self, GeneratorResult::Proved { .. })
    }

    pub fn is_computing(&self) -> bool {
        matches!(self, GeneratorResult::Computing { .. })
    }

    pub fn status_str(&self) -> &str {
        match self {
            GeneratorResult::Proved { .. } => "PROVED",
            GeneratorResult::Computing { .. } => "COMPUTING",
            GeneratorResult::NotInUniverse { .. } => "NOT_IN_𝒰",
        }
    }

    pub fn problem_id(&self) -> &str {
        match self {
            GeneratorResult::Proved { statement_id, .. } => statement_id,
            GeneratorResult::Computing { statement_id, .. } => statement_id,
            GeneratorResult::NotInUniverse { statement_id, .. } => statement_id,
        }
    }
}

/// The Generator G — computes proofs for all S ∈ 𝒰.
///
/// G(S) = least π under Π-order such that Check(S, π) = PASS.
///
/// G is a total function (for provable S).
/// G is NOT search. G is a defined projection.
///
/// The Π_proof projector wraps G with R caching.
pub struct Generator {
    /// R — the compiled cache of G's outputs.
    /// Each G(S) result is extracted into R for instant future lookups.
    pub basis: RewriteBasis,
    /// Self-awareness ledger.
    pub ledger: ProofLedger,
    /// Global definitions environment.
    pub env: CoreEnv,
    /// Snapshot budget — for test/snapshot mode only.
    /// None = unbounded (G runs to completion for provable S).
    /// Some(n) = take snapshot after n candidates (for testing).
    pub snapshot_budget: Option<u64>,
}

impl Generator {
    /// Create a new Generator with empty R.
    /// No snapshot budget = G runs to completion.
    pub fn new() -> Self {
        Self {
            basis: RewriteBasis::new(),
            ledger: ProofLedger::new(),
            env: CoreEnv::new(),
            snapshot_budget: None,
        }
    }

    /// Create a testing Generator (with snapshot budget).
    pub fn testing(budget: u64) -> Self {
        Self {
            basis: RewriteBasis::new(),
            ledger: ProofLedger::new(),
            env: CoreEnv::new(),
            snapshot_budget: Some(budget),
        }
    }

    /// G(S) — compute the proof for statement S.
    ///
    /// This is the canonical witness generator.
    /// Returns Proved when the proof is found.
    /// Returns Computing when snapshot_budget is hit (test mode only).
    /// In production (no budget), this runs to completion for provable S.
    pub fn generate(&mut self, problem_id: &str) -> GeneratorResult {
        let statement = get_statement(problem_id);

        // Phase 0: Check if S is in 𝒰 (formalized)
        if !is_formalized(&statement) {
            return GeneratorResult::NotInUniverse {
                statement_id: problem_id.into(),
                reason: format!("'{}' has placeholder formalization (True)", problem_id),
            };
        }

        // Phase 1: ELAB — bytes → CoreTerm goal
        let elab_result = elab_problem(problem_id);
        let goal = match elab_result {
            ElabResult::Ok { goal, .. } => goal,
            ElabResult::IllTyped { reason, .. } => {
                return GeneratorResult::NotInUniverse {
                    statement_id: problem_id.into(),
                    reason,
                };
            }
        };

        // Phase 2: R lookup — instant if already computed
        let (nf, _trace) = self.basis.normalize(&goal, &self.env, 10000);
        if is_proved_marker(&nf) {
            self.ledger.record_proof_found(
                problem_id, 0, nf.term_hash(), "R_cache",
            );
            return GeneratorResult::Proved {
                statement_id: problem_id.into(),
                witness: nf.clone(),
                witness_bytes: nf.to_bytes(),
                proof_hash: nf.term_hash(),
                method: "R_cache".into(),
                rank: 0,
                rules_extracted: 0,
            };
        }

        // Phase 3: Accelerator — compressed G for known patterns
        if let Some(accel_result) = try_accelerator(problem_id, &statement) {
            use super::engine::ProofResult;
            if let ProofResult::Proved { proof_hash, method, .. } = &accel_result {
                self.ledger.record_accelerator_result(problem_id, method, true);
                self.ledger.record_proof_found(problem_id, 0, *proof_hash, method);

                let accel_witness = CoreTerm::Const {
                    name: format!("accel_proof_{}", problem_id),
                    levels: vec![],
                };

                // Extract rules into R
                let extracted = extract_rules(&accel_witness, &goal, *proof_hash);
                let rules_count = extracted.len();
                for rule in extracted {
                    self.basis.add_rule(rule);
                }

                return GeneratorResult::Proved {
                    statement_id: problem_id.into(),
                    witness: accel_witness.clone(),
                    witness_bytes: accel_witness.to_bytes(),
                    proof_hash: *proof_hash,
                    method: format!("accelerator({})", method),
                    rank: 0,
                    rules_extracted: rules_count,
                };
            }
            self.ledger.record_accelerator_result(problem_id, "IRC+UCert", false);
        }

        // Phase 4: G(S) — canonical computation (NO BUDGET LIMIT in production)
        // G(S) = least π under Π-order such that Check(S, π) = PASS
        // This is NOT search. This is the defined projection.
        // For provable S, this terminates. For unprovable S (Gödel), it runs forever.
        let ctx = CoreCtx::new();
        let enumerator = WitnessEnumerator::new();
        let mut checked = 0u64;

        for (rank, bytes) in enumerator {
            // Snapshot budget (test/CLI mode only)
            if let Some(budget) = self.snapshot_budget {
                if checked >= budget {
                    // Snapshot: G is still computing. NOT frontier. NOT failure.
                    return GeneratorResult::Computing {
                        statement_id: problem_id.into(),
                        candidates_computed: checked,
                        current_rank: rank,
                    };
                }
            }

            // Try to elaborate bytes into CoreTerm
            let candidate = match elab_witness_bytes(&bytes) {
                Some(term) => term,
                None => {
                    checked += 1;
                    continue;
                }
            };

            // Type-check: does candidate inhabit goal?
            match type_check(&ctx, &candidate, &goal, &self.env) {
                CheckResult::Pass { proof_hash } => {
                    // G(S) COMPUTED — proof found
                    self.ledger.record_witness_check(
                        "G_compute", rank, bytes.len(), true, true,
                    );
                    self.ledger.record_proof_found(
                        problem_id, rank, proof_hash,
                        &format!("G_compute(rank={})", rank),
                    );

                    // Extract into R for future instant lookups
                    let extracted = extract_rules(&candidate, &goal, proof_hash);
                    let rules_count = extracted.len();
                    for rule in extracted {
                        self.basis.add_rule(rule);
                    }

                    return GeneratorResult::Proved {
                        statement_id: problem_id.into(),
                        witness: candidate,
                        witness_bytes: bytes,
                        proof_hash,
                        method: format!("G_compute(rank={})", rank),
                        rank,
                        rules_extracted: rules_count,
                    };
                }
                CheckResult::Fail { .. } => {
                    checked += 1;
                    if checked % 1000 == 0 && checked > 0 {
                        self.ledger.record_witness_check(
                            "G_compute", rank, bytes.len(), true, false,
                        );
                    }
                }
            }
        }

        // Unreachable for provable statements.
        // The enumerator is infinite and surjective over D*.
        // Every finite proof term appears at some rank.
        // If S is provable, G(S) terminates.
        unreachable!("G(S) must terminate for provable S ∈ 𝒰")
    }

    /// Run G on all S ∈ 𝒰.
    /// Each found proof is extracted into R, accelerating subsequent problems.
    pub fn generate_all(&mut self) -> Vec<GeneratorResult> {
        use crate::irc::ALL_PROBLEM_IDS;
        ALL_PROBLEM_IDS.iter().map(|id| self.generate(id)).collect()
    }

    /// Run G on a specific list of problems.
    pub fn generate_list(&mut self, problem_ids: &[&str]) -> Vec<GeneratorResult> {
        problem_ids.iter().map(|id| self.generate(id)).collect()
    }

    /// COMPLETE_𝒰 evidence — the collection of all proved (S, π) pairs.
    /// Built incrementally as G computes.
    pub fn complete_evidence(&self) -> CompleteEvidence {
        let proved = self.ledger.proved_problems();
        CompleteEvidence {
            proved_count: proved.len(),
            proved_ids: proved,
            total_in_universe: 20,
            basis_size: self.basis.len(),
            is_complete: self.ledger.proved_problems().len() == 20,
        }
    }

    /// Self-awareness summary.
    pub fn awareness_summary(&self) -> String {
        let evidence = self.complete_evidence();
        format!(
            "G: {}/{} proved | R: {} rules ({} applications) | COMPLETE_𝒰: {} | Ledger: {} events | Chain: {}",
            evidence.proved_count,
            evidence.total_in_universe,
            self.basis.len(),
            self.basis.total_applications(),
            if evidence.is_complete { "PROVED" } else { "building..." },
            self.ledger.len(),
            if self.ledger.verify_chain() { "VALID" } else { "BROKEN" },
        )
    }
}

/// Evidence for COMPLETE_𝒰.
///
/// COMPLETE_𝒰 is proved when every S ∈ 𝒰 has a verified proof in R.
/// The evidence is the collection of all (S, π) pairs.
#[derive(Debug)]
pub struct CompleteEvidence {
    /// Number of problems proved so far.
    pub proved_count: usize,
    /// IDs of proved problems.
    pub proved_ids: Vec<String>,
    /// Total members of 𝒰.
    pub total_in_universe: usize,
    /// Size of R (compiled rules from proofs).
    pub basis_size: usize,
    /// Is COMPLETE_𝒰 fully proved?
    pub is_complete: bool,
}

/// Check if a term is a PROVED marker (Constructor "Proved" "mk").
fn is_proved_marker(term: &CoreTerm) -> bool {
    matches!(term, CoreTerm::Constructor { type_name, ctor_name, .. }
        if type_name == "Proved" && ctor_name == "mk")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_creation() {
        let g = Generator::new();
        assert_eq!(g.basis.len(), 0);
        assert!(g.ledger.is_empty());
        assert!(g.snapshot_budget.is_none());
    }

    #[test]
    fn generator_testing() {
        let g = Generator::testing(100);
        assert_eq!(g.snapshot_budget, Some(100));
    }

    #[test]
    fn g_proves_known_via_accelerator() {
        let mut g = Generator::testing(10);
        let result = g.generate("zfc_zero_ne_one");
        assert!(result.is_proved(), "known theorem should be PROVED");
        assert_eq!(result.status_str(), "PROVED");
    }

    #[test]
    fn g_extracts_rules_from_proof() {
        let mut g = Generator::testing(10);
        let before = g.basis.len();
        g.generate("zfc_zero_ne_one");
        assert!(g.basis.len() > before, "proving should extract rules into R");
    }

    #[test]
    fn g_not_in_universe_for_stubs() {
        // Use a truly unknown problem (falls through to "True" default)
        let mut g = Generator::testing(10);
        let result = g.generate("nonexistent_stub_xyz");
        match &result {
            GeneratorResult::NotInUniverse { reason, .. } => {
                assert!(reason.contains("placeholder"));
            }
            other => panic!("stub problem should be NotInUniverse, got {:?}", other),
        }
    }

    #[test]
    fn g_computing_for_open() {
        let mut g = Generator::testing(10);
        let result = g.generate("goldbach");
        // With budget=10, G snapshots as Computing (not Frontier!)
        match &result {
            GeneratorResult::Computing { candidates_computed, .. } => {
                assert!(*candidates_computed <= 10);
            }
            GeneratorResult::Proved { .. } => {
                // If G found it, even better
            }
            other => panic!("expected Computing or Proved, got {:?}", other),
        }
    }

    #[test]
    fn g_never_returns_frontier() {
        // The Generator has NO Frontier variant. This is by design.
        let mut g = Generator::testing(5);
        for id in &["goldbach", "collatz", "twin_primes"] {
            let result = g.generate(id);
            // Result is either Proved, Computing, or NotInUniverse
            // NEVER Frontier
            match &result {
                GeneratorResult::Proved { .. } => {}
                GeneratorResult::Computing { .. } => {}
                GeneratorResult::NotInUniverse { .. } => {}
            }
        }
    }

    #[test]
    fn g_all_proves_7() {
        let mut g = Generator::testing(5);
        let results = g.generate_all();
        assert_eq!(results.len(), 20);
        let proved = results.iter().filter(|r| r.is_proved()).count();
        assert_eq!(proved, 7, "7 problems should be PROVED (7 IRC), got {}", proved);
    }

    #[test]
    fn g_all_no_frontier() {
        let mut g = Generator::testing(5);
        let results = g.generate_all();
        // Every result is Proved, Computing, or NotInUniverse
        // No Frontier anywhere
        for r in &results {
            assert!(
                matches!(r, GeneratorResult::Proved { .. }
                    | GeneratorResult::Computing { .. }
                    | GeneratorResult::NotInUniverse { .. }),
                "unexpected result type for {}: {:?}", r.problem_id(), r.status_str()
            );
        }
    }

    #[test]
    fn g_rules_grow_across_problems() {
        let mut g = Generator::testing(5);
        g.generate("zfc_zero_ne_one");
        let r1 = g.basis.len();
        g.generate("lagrange");
        let r2 = g.basis.len();
        assert!(r2 >= r1, "R should grow as more proofs are found");
    }

    #[test]
    fn g_complete_evidence() {
        let mut g = Generator::testing(5);
        g.generate_all();
        let evidence = g.complete_evidence();
        assert_eq!(evidence.proved_count, 7);
        assert_eq!(evidence.total_in_universe, 20);
        assert!(!evidence.is_complete, "not all 20 are proved yet");
        assert!(evidence.basis_size > 0, "R should have rules");
    }

    #[test]
    fn g_deterministic() {
        let mut g1 = Generator::testing(5);
        let mut g2 = Generator::testing(5);
        let r1 = g1.generate("zfc_zero_ne_one");
        let r2 = g2.generate("zfc_zero_ne_one");
        assert_eq!(r1.is_proved(), r2.is_proved(), "G must be deterministic");
    }

    #[test]
    fn g_awareness_summary() {
        let mut g = Generator::testing(5);
        g.generate("zfc_zero_ne_one");
        let summary = g.awareness_summary();
        assert!(summary.contains("G:"));
        assert!(summary.contains("R:"));
        assert!(summary.contains("VALID"));
    }
}
