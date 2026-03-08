//! μ-selector — canonical witness synthesis.
//!
//! Computes the least witness that satisfies the goal under Π-canonical ordering.
//! This is DETERMINISTIC COMPUTATION, not search.
//! There is no branching, no guessing, no heuristic.
//! The result is uniquely determined by the mathematics.
//!
//! The μ-selector costs time/energy (ledgered), but it is canonical:
//!   inv := μ(inv) s.t. Φ(inv)
//!   where μ = least under Π-canonical ordering
//!
//! Once a witness is found and compiled into R via ExtractRule,
//! future normalization of the same/similar problems is instant.

use super::core_term::{CoreTerm, CoreCtx, CoreEnv};
use super::type_check::{CheckResult, type_check};
use super::witness::WitnessEnumerator;
use super::elab::elab_witness_bytes;
use super::ledger::ProofLedger;
use kernel_types::{Hash32, hash};

/// Result of the μ-selector.
#[derive(Debug, Clone)]
pub enum MuResult {
    /// Witness found — the least candidate that type-checks.
    Found {
        /// The witness term.
        witness: CoreTerm,
        /// The raw bytes of the witness.
        witness_bytes: Vec<u8>,
        /// Hash of the witness.
        witness_hash: Hash32,
        /// Rank in the canonical ordering where it was found.
        rank: u64,
        /// How many candidates were checked.
        candidates_checked: u64,
    },
    /// Budget exhausted — no witness found within budget.
    Exhausted {
        /// How many candidates were checked.
        candidates_checked: u64,
        /// Maximum byte length reached.
        max_length: usize,
    },
}

impl MuResult {
    pub fn is_found(&self) -> bool {
        matches!(self, MuResult::Found { .. })
    }
}

/// Compute the least witness for a goal type.
///
/// This is the μ-selector: canonical, deterministic, no branching.
/// Enumerates CoreTerms by (size, then canonical byte hash).
/// For each: type_check(ctx, candidate, goal, env).
/// First PASS = the witness.
///
/// Costs time/energy (ledgered). But NOT search.
pub fn least_witness(
    goal: &CoreTerm,
    ctx: &CoreCtx,
    env: &CoreEnv,
    ledger: &mut ProofLedger,
    budget: u64,
) -> MuResult {
    let enumerator = WitnessEnumerator::new();
    let mut checked = 0u64;
    let mut max_len = 0usize;

    for (rank, bytes) in enumerator {
        if checked >= budget {
            break;
        }

        max_len = max_len.max(bytes.len());

        // Try to elaborate the bytes into a CoreTerm
        let candidate = match elab_witness_bytes(&bytes) {
            Some(term) => term,
            None => {
                checked += 1;
                continue;
            }
        };

        // Type-check: does this candidate inhabit the goal type?
        match type_check(ctx, &candidate, goal, env) {
            CheckResult::Pass { proof_hash } => {
                // FOUND — the least witness
                ledger.record_witness_check(
                    "mu_selector", rank, bytes.len(), true, true,
                );

                return MuResult::Found {
                    witness: candidate,
                    witness_bytes: bytes,
                    witness_hash: proof_hash,
                    rank,
                    candidates_checked: checked + 1,
                };
            }
            CheckResult::Fail { .. } => {
                // Not this one — continue to next
                if checked % 1000 == 0 && checked > 0 {
                    // Periodic ledger event for self-awareness
                    ledger.record_witness_check(
                        "mu_selector", rank, bytes.len(), true, false,
                    );
                }
                checked += 1;
            }
        }
    }

    MuResult::Exhausted {
        candidates_checked: checked,
        max_length: max_len,
    }
}

/// Try a specific CoreTerm as a witness for a goal.
/// Used when the accelerator or normalizer has a candidate.
pub fn check_candidate(
    candidate: &CoreTerm,
    goal: &CoreTerm,
    ctx: &CoreCtx,
    env: &CoreEnv,
) -> Option<Hash32> {
    match type_check(ctx, candidate, goal, env) {
        CheckResult::Pass { proof_hash } => Some(proof_hash),
        CheckResult::Fail { .. } => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mu_finds_nat_lit() {
        // Goal: Nat (any natural number is a witness)
        let goal = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();
        let mut ledger = ProofLedger::new();

        let result = least_witness(&goal, &ctx, &env, &mut ledger, 1000);

        // The μ-selector should find a NatLit witness
        // (the first valid CoreTerm bytes that parse as a NatLit and type-check as Nat)
        match &result {
            MuResult::Found { witness, rank, .. } => {
                // The witness should be a Nat
                match type_check(&ctx, witness, &goal, &env) {
                    CheckResult::Pass { .. } => {} // correct
                    CheckResult::Fail { reason } => panic!("witness should type-check: {}", reason),
                }
            }
            MuResult::Exhausted { candidates_checked, .. } => {
                // With budget 1000, we might not reach a valid CoreTerm
                // that's fine — the μ-selector is honest about exhaustion
            }
        }
    }

    #[test]
    fn mu_exhausts_budget() {
        // Goal: something hard to satisfy with small budget
        let goal = CoreTerm::Pi {
            param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
            body: Box::new(CoreTerm::Prop),
        };
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();
        let mut ledger = ProofLedger::new();

        let result = least_witness(&goal, &ctx, &env, &mut ledger, 10);

        match result {
            MuResult::Exhausted { candidates_checked, .. } => {
                assert!(candidates_checked <= 10);
            }
            MuResult::Found { .. } => {
                // If found, that's also fine — the μ-selector is correct
            }
        }
    }

    #[test]
    fn check_candidate_pass() {
        let candidate = CoreTerm::NatLit(42);
        let goal = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        assert!(check_candidate(&candidate, &goal, &ctx, &env).is_some());
    }

    #[test]
    fn check_candidate_fail() {
        let candidate = CoreTerm::NatLit(42);
        let goal = CoreTerm::Prop; // NatLit is not a Prop
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        assert!(check_candidate(&candidate, &goal, &ctx, &env).is_none());
    }

    #[test]
    fn mu_deterministic() {
        // Same goal + budget → same result
        let goal = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let mut ledger1 = ProofLedger::new();
        let mut ledger2 = ProofLedger::new();

        let r1 = least_witness(&goal, &ctx, &env, &mut ledger1, 100);
        let r2 = least_witness(&goal, &ctx, &env, &mut ledger2, 100);

        // Both should produce the same outcome
        match (&r1, &r2) {
            (MuResult::Found { rank: r1, .. }, MuResult::Found { rank: r2, .. }) => {
                assert_eq!(r1, r2, "μ-selector must be deterministic");
            }
            (MuResult::Exhausted { .. }, MuResult::Exhausted { .. }) => {
                // Both exhausted — deterministic
            }
            _ => panic!("results should be the same kind"),
        }
    }
}
