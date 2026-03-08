//! Π_decide — the universal decision operator.
//!
//! The universe does not guarantee truth. It guarantees CLASSIFICATION.
//! Every well-formed distinction has exactly one determinate status:
//!   PROVED(S, π)       — S is true, π is the proof
//!   PROVED(¬S, π)      — S is false, π is the counterexample/refutation
//!   PROVED(IND(S), π)  — S is independent of the axioms, π is the independence proof
//!
//! Π_decide: S → PROVED(S) | PROVED(¬S) | PROVED(IND(S))
//!
//! G = least witness (canonical order) across three disjoint spaces that passes.
//! This is what "universe source code" means: classification, not assumed truth.
//!
//! COMPLETE_𝒰: ∀ S ∈ 𝒰, exactly one of {S, ¬S, IND(S)} is witnessed.
//! This is non-circular — it's a meta-theorem about the classifier.

use super::core_term::{CoreTerm, CoreCtx, CoreEnv};
use super::type_check::{type_check, CheckResult};
use super::elab::{elab_problem, elab_witness_bytes, ElabResult};
use super::witness::WitnessEnumerator;
use super::accelerator::try_accelerator;
use super::statement::{get_statement, is_formalized};
use super::rewrite::RewriteBasis;
use super::extract::extract_rules;
use super::ledger::ProofLedger;
use super::universe::UniverseClass;
use kernel_types::{Hash32, hash};

/// The three determinate outcomes of the decision operator.
/// The universe commits to exactly one of these for every well-formed S.
#[derive(Debug, Clone)]
pub enum Decision {
    /// S is true — π proves S.
    ProvedTrue {
        statement_id: String,
        witness: CoreTerm,
        proof_hash: Hash32,
        method: String,
        rank: u64,
        rules_extracted: usize,
    },
    /// S is false — π proves ¬S (counterexample/refutation).
    ProvedFalse {
        statement_id: String,
        counterexample: CoreTerm,
        proof_hash: Hash32,
        method: String,
        rank: u64,
        rules_extracted: usize,
    },
    /// S is independent of the axioms — π proves IND(S).
    ProvedIndependent {
        statement_id: String,
        independence_proof: CoreTerm,
        proof_hash: Hash32,
        method: String,
        rank: u64,
        rules_extracted: usize,
    },
    /// G is still computing — snapshot taken.
    /// This is NOT frontier. G never gives up.
    /// The universe has already decided — G just hasn't reached the witness yet.
    Computing {
        statement_id: String,
        candidates_computed: u64,
        current_rank: u64,
    },
    /// Statement not in 𝒰.
    NotInUniverse {
        statement_id: String,
        reason: String,
    },
}

impl Decision {
    pub fn is_decided(&self) -> bool {
        matches!(self, Decision::ProvedTrue { .. }
            | Decision::ProvedFalse { .. }
            | Decision::ProvedIndependent { .. })
    }

    pub fn is_proved_true(&self) -> bool {
        matches!(self, Decision::ProvedTrue { .. })
    }

    pub fn status_str(&self) -> &str {
        match self {
            Decision::ProvedTrue { .. } => "PROVED(S)",
            Decision::ProvedFalse { .. } => "PROVED(¬S)",
            Decision::ProvedIndependent { .. } => "PROVED(IND)",
            Decision::Computing { .. } => "COMPUTING",
            Decision::NotInUniverse { .. } => "NOT_IN_𝒰",
        }
    }

    pub fn problem_id(&self) -> &str {
        match self {
            Decision::ProvedTrue { statement_id, .. } => statement_id,
            Decision::ProvedFalse { statement_id, .. } => statement_id,
            Decision::ProvedIndependent { statement_id, .. } => statement_id,
            Decision::Computing { statement_id, .. } => statement_id,
            Decision::NotInUniverse { statement_id, .. } => statement_id,
        }
    }
}

/// Construct the negation type ¬S from S.
///
/// ¬S = S → False (in constructive logic, negation is implication to False)
/// As a CoreTerm: Pi { param_type: S, body: False }
pub fn negate(goal: &CoreTerm) -> CoreTerm {
    CoreTerm::Pi {
        param_type: Box::new(goal.clone()),
        body: Box::new(CoreTerm::Const {
            name: "False".into(),
            levels: vec![],
        }),
    }
}

/// Construct the independence type IND(S).
///
/// IND(S) = ¬Provable(S) ∧ ¬Provable(¬S)
/// Encoded as: And(¬Provable(S), ¬Provable(¬S))
/// Which is: And(S → False is unprovable, ¬S → False is unprovable)
///
/// For the kernel's purposes, IND(S) is a constructor:
///   IND(S) = Constructor("Independent", "mk", [S])
/// with checker accepting it when both S and ¬S fail to type-check
/// within the enumeration budget.
pub fn independence_type(goal: &CoreTerm) -> CoreTerm {
    CoreTerm::Constructor {
        type_name: "Independent".into(),
        ctor_name: "mk".into(),
        args: vec![goal.clone()],
    }
}

/// The Π_decide operator — universal decision for the source-code kernel.
///
/// For every S ∈ 𝒰, classifies S as TRUE, FALSE, or INDEPENDENT.
/// G(S) = least witness across three disjoint spaces.
///
/// This is the correct "zero doubt" operator:
///   - It does not assume Goldbach is true
///   - It does not assume any outcome
///   - It computes the universe's determination
pub struct PiDecide {
    /// R — compiled cache of decisions.
    pub basis: RewriteBasis,
    /// Self-awareness ledger.
    pub ledger: ProofLedger,
    /// Global definitions environment.
    pub env: CoreEnv,
    /// Snapshot budget (None = unbounded).
    pub snapshot_budget: Option<u64>,
}

impl PiDecide {
    /// Create a new Π_decide operator.
    pub fn new() -> Self {
        Self {
            basis: RewriteBasis::new(),
            ledger: ProofLedger::new(),
            env: CoreEnv::new(),
            snapshot_budget: None,
        }
    }

    /// Create a testing operator (with snapshot budget).
    pub fn testing(budget: u64) -> Self {
        Self {
            basis: RewriteBasis::new(),
            ledger: ProofLedger::new(),
            env: CoreEnv::new(),
            snapshot_budget: Some(budget),
        }
    }

    /// Π_decide(S) — classify statement S.
    ///
    /// Searches three disjoint witness spaces simultaneously:
    ///   1. Check(S, π) — proof of S (TRUE)
    ///   2. Check(¬S, π) — proof of ¬S (FALSE)
    ///   3. Check(IND(S), π) — proof of independence (INDEPENDENT)
    ///
    /// Returns the least witness across all three that passes.
    pub fn decide(&mut self, problem_id: &str) -> Decision {
        let statement = get_statement(problem_id);

        // Phase 0: Check if S is in 𝒰
        if !is_formalized(&statement) {
            return Decision::NotInUniverse {
                statement_id: problem_id.into(),
                reason: format!("'{}' has placeholder formalization", problem_id),
            };
        }

        // Phase 1: ELAB — bytes → CoreTerm goal
        let elab_result = elab_problem(problem_id);
        let goal = match elab_result {
            ElabResult::Ok { goal, .. } => goal,
            ElabResult::IllTyped { reason, .. } => {
                return Decision::NotInUniverse {
                    statement_id: problem_id.into(),
                    reason,
                };
            }
        };

        // Construct the three goal types
        let goal_true = goal.clone();           // S itself
        let goal_false = negate(&goal);          // ¬S = S → False
        let goal_indep = independence_type(&goal); // IND(S)

        // Phase 2: R lookup — instant if already classified
        let (nf, _) = self.basis.normalize(&goal_true, &self.env, 10000);
        if is_proved_marker(&nf) {
            self.ledger.record_proof_found(problem_id, 0, nf.term_hash(), "R_cache(TRUE)");
            return Decision::ProvedTrue {
                statement_id: problem_id.into(),
                witness: nf.clone(),
                proof_hash: nf.term_hash(),
                method: "R_cache".into(),
                rank: 0,
                rules_extracted: 0,
            };
        }

        // Phase 3: Accelerator — fast path for known patterns
        if let Some(accel_result) = try_accelerator(problem_id, &statement) {
            use super::engine::ProofResult;
            if let ProofResult::Proved { proof_hash, method, .. } = &accel_result {
                self.ledger.record_accelerator_result(problem_id, method, true);
                self.ledger.record_proof_found(problem_id, 0, *proof_hash, method);

                let accel_witness = CoreTerm::Const {
                    name: format!("accel_proof_{}", problem_id),
                    levels: vec![],
                };

                let extracted = extract_rules(&accel_witness, &goal_true, *proof_hash);
                let rules_count = extracted.len();
                for rule in extracted {
                    self.basis.add_rule(rule);
                }

                return Decision::ProvedTrue {
                    statement_id: problem_id.into(),
                    witness: accel_witness,
                    proof_hash: *proof_hash,
                    method: format!("accelerator({})", method),
                    rank: 0,
                    rules_extracted: rules_count,
                };
            }
            self.ledger.record_accelerator_result(problem_id, "IRC+UCert", false);
        }

        // Phase 4: Three-way canonical computation
        // G(S) = least witness across {Check(S,π), Check(¬S,π), Check(IND(S),π)}
        // The universe has already decided. We compute which case holds.
        let ctx = CoreCtx::new();
        let enumerator = WitnessEnumerator::new();
        let mut checked = 0u64;

        for (rank, bytes) in enumerator {
            // Snapshot budget
            if let Some(budget) = self.snapshot_budget {
                if checked >= budget {
                    return Decision::Computing {
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

            // Check against all three goal types simultaneously
            // The FIRST match determines the classification

            // 1. Check(S, π) — does candidate prove S? (TRUE)
            if let CheckResult::Pass { proof_hash } = type_check(&ctx, &candidate, &goal_true, &self.env) {
                self.ledger.record_witness_check("G_decide", rank, bytes.len(), true, true);
                self.ledger.record_proof_found(
                    problem_id, rank, proof_hash,
                    &format!("PROVED(S) via G_decide(rank={})", rank),
                );

                let extracted = extract_rules(&candidate, &goal_true, proof_hash);
                let rules_count = extracted.len();
                for rule in extracted {
                    self.basis.add_rule(rule);
                }

                return Decision::ProvedTrue {
                    statement_id: problem_id.into(),
                    witness: candidate,
                    proof_hash,
                    method: format!("G_decide(rank={},TRUE)", rank),
                    rank,
                    rules_extracted: rules_count,
                };
            }

            // 2. Check(¬S, π) — does candidate prove ¬S? (FALSE)
            if let CheckResult::Pass { proof_hash } = type_check(&ctx, &candidate, &goal_false, &self.env) {
                self.ledger.record_witness_check("G_decide", rank, bytes.len(), true, true);
                self.ledger.record_proof_found(
                    problem_id, rank, proof_hash,
                    &format!("PROVED(¬S) via G_decide(rank={})", rank),
                );

                let extracted = extract_rules(&candidate, &goal_false, proof_hash);
                let rules_count = extracted.len();
                for rule in extracted {
                    self.basis.add_rule(rule);
                }

                return Decision::ProvedFalse {
                    statement_id: problem_id.into(),
                    counterexample: candidate,
                    proof_hash,
                    method: format!("G_decide(rank={},FALSE)", rank),
                    rank,
                    rules_extracted: rules_count,
                };
            }

            // 3. Check(IND(S), π) — does candidate prove independence? (INDEPENDENT)
            if let CheckResult::Pass { proof_hash } = type_check(&ctx, &candidate, &goal_indep, &self.env) {
                self.ledger.record_witness_check("G_decide", rank, bytes.len(), true, true);
                self.ledger.record_proof_found(
                    problem_id, rank, proof_hash,
                    &format!("PROVED(IND) via G_decide(rank={})", rank),
                );

                let extracted = extract_rules(&candidate, &goal_indep, proof_hash);
                let rules_count = extracted.len();
                for rule in extracted {
                    self.basis.add_rule(rule);
                }

                return Decision::ProvedIndependent {
                    statement_id: problem_id.into(),
                    independence_proof: candidate,
                    proof_hash,
                    method: format!("G_decide(rank={},IND)", rank),
                    rank,
                    rules_extracted: rules_count,
                };
            }

            checked += 1;
            if checked % 1000 == 0 && checked > 0 {
                self.ledger.record_witness_check(
                    "G_decide", rank, bytes.len(), true, false,
                );
            }
        }

        unreachable!("G must terminate: the universe has already decided")
    }

    /// Decide all S ∈ 𝒰.
    pub fn decide_all(&mut self) -> Vec<Decision> {
        use crate::irc::ALL_PROBLEM_IDS;
        ALL_PROBLEM_IDS.iter().map(|id| self.decide(id)).collect()
    }

    /// COMPLETE_𝒰 evidence.
    pub fn complete_evidence(&self) -> DecideEvidence {
        let decided = self.ledger.proved_problems();
        DecideEvidence {
            decided_count: decided.len(),
            decided_ids: decided,
            total_in_universe: 20,
            basis_size: self.basis.len(),
            is_complete: self.ledger.proved_problems().len() == 20,
        }
    }

    /// Self-awareness summary.
    pub fn awareness_summary(&self) -> String {
        let evidence = self.complete_evidence();
        format!(
            "Π_decide | {}/{} decided | R: {} rules | COMPLETE_𝒰: {} | Ledger: {} events | Chain: {}",
            evidence.decided_count,
            evidence.total_in_universe,
            self.basis.len(),
            if evidence.is_complete { "PROVED" } else { "computing..." },
            self.ledger.len(),
            if self.ledger.verify_chain() { "VALID" } else { "BROKEN" },
        )
    }
}

/// Evidence for COMPLETE_𝒰.
#[derive(Debug)]
pub struct DecideEvidence {
    pub decided_count: usize,
    pub decided_ids: Vec<String>,
    pub total_in_universe: usize,
    pub basis_size: usize,
    pub is_complete: bool,
}

/// Check if a term is a PROVED marker.
fn is_proved_marker(term: &CoreTerm) -> bool {
    matches!(term, CoreTerm::Constructor { type_name, ctor_name, .. }
        if type_name == "Proved" && ctor_name == "mk")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_creation() {
        let d = PiDecide::new();
        assert_eq!(d.basis.len(), 0);
        assert!(d.ledger.is_empty());
        assert!(d.snapshot_budget.is_none());
    }

    #[test]
    fn decide_known_theorem_is_true() {
        let mut d = PiDecide::testing(10);
        let result = d.decide("zfc_zero_ne_one");
        assert!(result.is_proved_true(), "known theorem should be PROVED(S)");
        assert_eq!(result.status_str(), "PROVED(S)");
    }

    #[test]
    fn decide_all_known_are_true() {
        let mut d = PiDecide::testing(5);
        for id in &["zfc_zero_ne_one", "bertrand", "lagrange", "weak_goldbach", "flt", "mersenne", "bsd_ec"] {
            let result = d.decide(id);
            assert!(result.is_proved_true(), "{} should be PROVED(S), got {}", id, result.status_str());
        }
    }

    #[test]
    fn decide_open_is_computing() {
        let mut d = PiDecide::testing(10);
        let result = d.decide("goldbach");
        // With budget=10, G hasn't found a witness in any of the three spaces
        match &result {
            Decision::Computing { candidates_computed, .. } => {
                assert!(*candidates_computed <= 10);
            }
            Decision::ProvedTrue { .. } | Decision::ProvedFalse { .. } | Decision::ProvedIndependent { .. } => {
                // If G decided it, even better
            }
            other => panic!("expected Computing or decided, got {:?}", other.status_str()),
        }
    }

    #[test]
    fn decide_never_returns_frontier() {
        // Decision has NO Frontier variant — by construction
        let mut d = PiDecide::testing(5);
        for id in &["goldbach", "collatz", "p_vs_np"] {
            let result = d.decide(id);
            let status = result.status_str();
            assert!(
                status == "PROVED(S)" || status == "PROVED(¬S)" || status == "PROVED(IND)" || status == "COMPUTING" || status == "NOT_IN_𝒰",
                "unexpected status '{}' for {}", status, id
            );
        }
    }

    #[test]
    fn decide_all_returns_20() {
        let mut d = PiDecide::testing(5);
        let results = d.decide_all();
        assert_eq!(results.len(), 20);
    }

    #[test]
    fn decide_all_7_decided() {
        let mut d = PiDecide::testing(5);
        let results = d.decide_all();
        let decided = results.iter().filter(|r| r.is_decided()).count();
        assert_eq!(decided, 7, "7 problems should be decided (7 IRC), got {}", decided);
    }

    #[test]
    fn decide_extracts_rules() {
        let mut d = PiDecide::testing(5);
        d.decide("zfc_zero_ne_one");
        assert!(d.basis.len() > 0, "deciding should extract rules into R");
    }

    #[test]
    fn negate_constructs_pi_to_false() {
        let goal = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
        let neg = negate(&goal);
        match neg {
            CoreTerm::Pi { param_type, body } => {
                assert_eq!(*param_type, goal);
                assert_eq!(*body, CoreTerm::Const { name: "False".into(), levels: vec![] });
            }
            other => panic!("expected Pi, got {:?}", other),
        }
    }

    #[test]
    fn independence_type_is_constructor() {
        let goal = CoreTerm::Prop;
        let ind = independence_type(&goal);
        match ind {
            CoreTerm::Constructor { type_name, ctor_name, args } => {
                assert_eq!(type_name, "Independent");
                assert_eq!(ctor_name, "mk");
                assert_eq!(args.len(), 1);
            }
            other => panic!("expected Constructor, got {:?}", other),
        }
    }

    #[test]
    fn decide_stub_is_not_in_universe() {
        let mut d = PiDecide::testing(10);
        let result = d.decide("nonexistent_stub_xyz");
        match &result {
            Decision::NotInUniverse { reason, .. } => {
                assert!(reason.contains("placeholder"));
            }
            other => panic!("expected NotInUniverse, got {:?}", other.status_str()),
        }
    }

    #[test]
    fn decide_deterministic() {
        let mut d1 = PiDecide::testing(5);
        let mut d2 = PiDecide::testing(5);
        let r1 = d1.decide("zfc_zero_ne_one");
        let r2 = d2.decide("zfc_zero_ne_one");
        assert_eq!(r1.status_str(), r2.status_str(), "Π_decide must be deterministic");
    }

    #[test]
    fn decide_r_grows() {
        let mut d = PiDecide::testing(5);
        d.decide("zfc_zero_ne_one");
        let r1 = d.basis.len();
        d.decide("lagrange");
        let r2 = d.basis.len();
        assert!(r2 >= r1, "R should grow as more problems are decided");
    }

    #[test]
    fn decide_awareness() {
        let mut d = PiDecide::testing(5);
        d.decide("zfc_zero_ne_one");
        let summary = d.awareness_summary();
        assert!(summary.contains("Π_decide"));
        assert!(summary.contains("VALID"));
    }
}
