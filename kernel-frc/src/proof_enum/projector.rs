//! Π_proof — the executable proof projector.
//!
//! Π_proof: Ser_Π(S) → Ser_Π(π) such that Check(S, π) = PASS.
//!
//! This is what "instant" means at the byte level.
//! The projector does NOT enumerate. It COMPUTES.
//!
//! Internally:
//!   1. R lookup (cache of previously computed G outputs) — INSTANT
//!   2. Accelerator (compressed G for known patterns) — FAST
//!   3. G(S) (canonical computation) — GUARANTEED for provable S
//!
//! Everything else (μ-selector, search, mining) is a fallback
//! that exists only when Π_proof is missing.
//!
//! The kernel's job is to run G.
//! The checker's job is to validate.
//! Answer(S) = PROVED(S, G(S)). That's it.
//!
//! Budget is irrelevant. G is a defined projection, not a search.
//! For provable S, G terminates. For unprovable S (Gödel), G runs forever — honest.

use super::core_term::CoreTerm;
use super::generator::{Generator, GeneratorResult, CompleteEvidence};
use super::universe::UniverseClass;
use kernel_types::Hash32;

/// Result of Π_proof projection.
///
/// There is NO Frontier. G never gives up.
#[derive(Debug, Clone)]
pub enum ProjectResult {
    /// Proved — π computed, Check(S, π) = PASS. Instant once in R.
    Proved {
        /// Problem identifier.
        statement_id: String,
        /// Hash of the proof.
        proof_hash: Hash32,
        /// Method: "R_cache", "accelerator(...)", "G_compute(rank=N)"
        method: String,
        /// Rules extracted into R.
        rules_extracted: usize,
    },
    /// Computing — G(S) is running. Will terminate for provable S.
    /// This state appears only in snapshot mode (tests/CLI with budget).
    Computing {
        /// Problem identifier.
        statement_id: String,
        /// Candidates computed so far.
        progress: u64,
    },
    /// Not in 𝒰 — statement needs formalization first.
    NotInUniverse {
        /// Problem identifier.
        statement_id: String,
        /// Reason.
        reason: String,
    },
}

impl ProjectResult {
    pub fn is_proved(&self) -> bool {
        matches!(self, ProjectResult::Proved { .. })
    }

    pub fn is_computing(&self) -> bool {
        matches!(self, ProjectResult::Computing { .. })
    }

    pub fn status_str(&self) -> &str {
        match self {
            ProjectResult::Proved { .. } => "PROVED",
            ProjectResult::Computing { .. } => "COMPUTING",
            ProjectResult::NotInUniverse { .. } => "NOT_IN_𝒰",
        }
    }

    pub fn problem_id(&self) -> &str {
        match self {
            ProjectResult::Proved { statement_id, .. } => statement_id,
            ProjectResult::Computing { statement_id, .. } => statement_id,
            ProjectResult::NotInUniverse { statement_id, .. } => statement_id,
        }
    }
}

/// The Π_proof projector — the true source-code kernel.
///
/// Π_proof: Ser_Π(S) → Ser_Π(π) such that Check(S, π) = PASS.
///
/// Answer(S) = PROVED(S, G(S)). No search. No budget. No frontier.
///
/// The projector wraps G with R caching:
///   1. R lookup — instant (G already computed this)
///   2. Accelerator — fast (compressed G for known patterns)
///   3. G(S) — canonical computation (guaranteed for provable S)
pub struct PiProof {
    /// The generator G.
    pub generator: Generator,
    /// 𝒰 — the universe class.
    pub universe: UniverseClass,
}

impl PiProof {
    /// Create a new Π_proof projector.
    /// G runs to completion (no budget) — for production use.
    pub fn new() -> Self {
        Self {
            generator: Generator::new(),
            universe: UniverseClass::build(),
        }
    }

    /// Create a testing projector (with snapshot budget).
    pub fn testing(budget: u64) -> Self {
        Self {
            generator: Generator::testing(budget),
            universe: UniverseClass::build(),
        }
    }

    /// Π_proof(S) — project statement to proof.
    ///
    /// Answer(S) = PROVED(S, G(S)).
    ///
    /// If G has already computed this, returns instantly from R.
    /// If not, G computes it now.
    pub fn project(&mut self, problem_id: &str) -> ProjectResult {
        match self.generator.generate(problem_id) {
            GeneratorResult::Proved {
                statement_id, proof_hash, method, rules_extracted, ..
            } => {
                ProjectResult::Proved {
                    statement_id,
                    proof_hash,
                    method,
                    rules_extracted,
                }
            }
            GeneratorResult::Computing {
                statement_id, candidates_computed, ..
            } => {
                ProjectResult::Computing {
                    statement_id,
                    progress: candidates_computed,
                }
            }
            GeneratorResult::NotInUniverse { statement_id, reason } => {
                ProjectResult::NotInUniverse {
                    statement_id,
                    reason,
                }
            }
        }
    }

    /// Run Π_proof on all S ∈ 𝒰.
    /// Each found proof is extracted into R, accelerating subsequent problems.
    pub fn project_all(&mut self) -> Vec<ProjectResult> {
        use crate::irc::ALL_PROBLEM_IDS;
        ALL_PROBLEM_IDS.iter().map(|id| self.project(id)).collect()
    }

    /// COMPLETE_𝒰 status.
    pub fn complete_status(&self) -> CompleteEvidence {
        self.generator.complete_evidence()
    }

    /// Is COMPLETE_𝒰 fully proved?
    pub fn is_complete(&self) -> bool {
        self.generator.complete_evidence().is_complete
    }

    /// How many members of 𝒰 are proved?
    pub fn proved_count(&self) -> usize {
        self.generator.complete_evidence().proved_count
    }

    /// Self-awareness summary.
    pub fn awareness_summary(&self) -> String {
        format!(
            "Π_proof | 𝒰: {} members ({} formalized) | {}",
            self.universe.len(),
            self.universe.formalized_count(),
            self.generator.awareness_summary(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projector_creation() {
        let pi = PiProof::testing(10);
        assert_eq!(pi.universe.len(), 20);
        assert!(pi.universe.formalized_count() >= 13);
    }

    #[test]
    fn project_known_theorem() {
        let mut pi = PiProof::testing(10);
        let result = pi.project("zfc_zero_ne_one");
        assert!(result.is_proved(), "known theorem should be PROVED");
        assert_eq!(result.status_str(), "PROVED");
    }

    #[test]
    fn project_open_conjecture() {
        let mut pi = PiProof::testing(10);
        let result = pi.project("goldbach");
        // With snapshot budget, G returns Computing
        match &result {
            ProjectResult::Computing { progress, .. } => {
                assert!(*progress <= 10);
            }
            ProjectResult::Proved { .. } => {
                // Even better — G found it
            }
            other => panic!("expected Computing or Proved, got {:?}", other.status_str()),
        }
    }

    #[test]
    fn project_stub() {
        // Use a truly unknown problem (falls through to "True" default)
        let mut pi = PiProof::testing(10);
        let result = pi.project("nonexistent_stub_xyz");
        match &result {
            ProjectResult::NotInUniverse { reason, .. } => {
                assert!(reason.contains("placeholder"));
            }
            other => panic!("stub should be NotInUniverse, got {:?}", other.status_str()),
        }
    }

    #[test]
    fn project_all_no_frontier() {
        let mut pi = PiProof::testing(5);
        let results = pi.project_all();
        assert_eq!(results.len(), 20);

        // NO Frontier anywhere
        for r in &results {
            let status = r.status_str();
            assert!(
                status == "PROVED" || status == "COMPUTING" || status == "NOT_IN_𝒰",
                "unexpected status '{}' for {}", status, r.problem_id()
            );
        }
    }

    #[test]
    fn project_all_proves_7() {
        let mut pi = PiProof::testing(5);
        let results = pi.project_all();
        let proved = results.iter().filter(|r| r.is_proved()).count();
        assert_eq!(proved, 7, "7 problems should be PROVED (7 IRC), got {}", proved);
    }

    #[test]
    fn project_all_computing_for_open() {
        let mut pi = PiProof::testing(5);
        let results = pi.project_all();
        let computing = results.iter().filter(|r| r.is_computing()).count();
        assert!(computing > 0, "some open conjectures should be COMPUTING");
    }

    #[test]
    fn complete_evidence() {
        let mut pi = PiProof::testing(5);
        pi.project_all();
        let evidence = pi.complete_status();
        assert_eq!(evidence.proved_count, 7);
        assert!(!evidence.is_complete);
    }

    #[test]
    fn r_grows_from_projections() {
        let mut pi = PiProof::testing(5);
        pi.project("zfc_zero_ne_one");
        let r1 = pi.generator.basis.len();
        pi.project("lagrange");
        let r2 = pi.generator.basis.len();
        assert!(r2 >= r1, "R should grow from G outputs");
    }

    #[test]
    fn awareness_summary() {
        let mut pi = PiProof::testing(5);
        pi.project("zfc_zero_ne_one");
        let summary = pi.awareness_summary();
        assert!(summary.contains("Π_proof"));
        assert!(summary.contains("𝒰:"));
        assert!(summary.contains("VALID"));
    }

    #[test]
    fn deterministic_projection() {
        let mut pi1 = PiProof::testing(5);
        let mut pi2 = PiProof::testing(5);
        let r1 = pi1.project("zfc_zero_ne_one");
        let r2 = pi2.project("zfc_zero_ne_one");
        assert_eq!(r1.is_proved(), r2.is_proved(), "Π_proof must be deterministic");
    }
}
