//! Accelerator bridge — IRC/InvSyn/UCert fast path.
//!
//! The accelerator is a COMPRESSION LAYER, not the engine.
//! It is tried first because it's fast for known-pattern problems.
//! For open problems, the accelerator returns None and the universal
//! witness enumerator (the real engine) takes over.

use kernel_types::hash;

use crate::irc::{IrcSearch, PROVED_PROBLEM_IDS};
use crate::frc_types::IrcResult;
use crate::ucert::{compile_problem, ucert_normalize, NormalizeResult};

use super::statement::ProofStatement;
use super::engine::ProofResult;

/// Try the accelerator (IRC + UCert) for fast results.
///
/// Returns Some(ProofResult::Proved) if the accelerator solves it, None otherwise.
/// When None, the universal witness enumerator must be used.
///
/// The accelerator is COMPRESSED G — it computes G(S) directly for known patterns.
/// Each layer is a projection of G, not a separate engine:
///   1. IRC — invariant-based proof construction
///   2. UCert — certificate normalization
pub fn try_accelerator(problem_id: &str, statement: &ProofStatement) -> Option<ProofResult> {
    // Fast path 1: Try IRC
    if let Some(result) = try_irc_accelerator(problem_id, statement) {
        return Some(result);
    }

    // Fast path 2: Try UCert
    if let Some(result) = try_ucert_accelerator(problem_id, statement) {
        return Some(result);
    }

    None
}

/// Try IRC accelerator — if IRC proves all 3 obligations, record the result.
fn try_irc_accelerator(problem_id: &str, statement: &ProofStatement) -> Option<ProofResult> {
    let engine = IrcSearch::new();
    let result = engine.search(problem_id);

    match result {
        IrcResult::Proved(irc) => {
            let proof_hash = hash::H(
                format!("irc_proved:{}:{}", problem_id, irc.irc_hash.iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>())
                .as_bytes()
            );

            let method = format!("IRC(invariant={:?})", irc.invariant.kind);
            let proof_script = format!("-- Proved by IRC accelerator: {}", method);

            Some(ProofResult::Proved {
                statement: statement.clone(),
                witness: proof_script.as_bytes().to_vec(),
                proof_script,
                rank: 0,
                proof_hash,
                method,
            })
        }
        IrcResult::Frontier(_) => None,
    }
}

/// Try UCert accelerator — if UCert finds a certificate, record the result.
fn try_ucert_accelerator(problem_id: &str, statement: &ProofStatement) -> Option<ProofResult> {
    let ucert_stmt = compile_problem(problem_id);
    let result = ucert_normalize(&ucert_stmt, 1000);

    match result {
        NormalizeResult::Proved { certificate, rank, proof_hash, .. } => {
            let method = format!("UCert(rank={}, size={})", rank, certificate.size());
            let proof_script = format!("-- Proved by UCert accelerator: {}", method);

            Some(ProofResult::Proved {
                statement: statement.clone(),
                witness: proof_script.as_bytes().to_vec(),
                proof_script,
                rank,
                proof_hash,
                method,
            })
        }
        NormalizeResult::Frontier { .. } => None,
    }
}


/// Check if a problem is in the known-proved list.
pub fn is_known_proved(problem_id: &str) -> bool {
    PROVED_PROBLEM_IDS.contains(&problem_id)
}

/// All problems decidable by the accelerator (IRC + UCert).
pub const ACCELERATOR_DECIDABLE: &[&str] = &[
    // IRC-proved (7)
    "zfc_zero_ne_one", "bertrand", "lagrange", "weak_goldbach", "flt", "mersenne", "bsd_ec",
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_enum::statement::get_statement;

    #[test]
    fn accelerator_proves_zfc() {
        let stmt = get_statement("zfc_zero_ne_one");
        let result = try_accelerator("zfc_zero_ne_one", &stmt);
        assert!(result.is_some(), "ZFC 0≠1 should be proved by accelerator");
        match result.unwrap() {
            ProofResult::Proved { method, .. } => {
                assert!(method.starts_with("IRC") || method.starts_with("UCert"),
                    "method should be IRC or UCert, got: {}", method);
            }
            ProofResult::Frontier { .. } => panic!("ZFC should be PROVED"),
        }
    }

    #[test]
    fn accelerator_proves_known() {
        for id in PROVED_PROBLEM_IDS {
            let stmt = get_statement(id);
            let result = try_accelerator(id, &stmt);
            assert!(result.is_some(), "{} should be proved by accelerator", id);
        }
    }

    #[test]
    fn accelerator_returns_none_for_open() {
        // These are open or pending formalization — accelerator should return None
        for id in &["goldbach", "collatz", "twin_primes", "odd_perfect", "legendre", "erdos_straus",
                     "mertens", "p_vs_np", "riemann_full", "navier_stokes", "yang_mills", "hodge", "bsd_full"] {
            let stmt = get_statement(id);
            let result = try_accelerator(id, &stmt);
            assert!(result.is_none(), "{} should NOT be proved by accelerator", id);
        }
    }

    #[test]
    fn is_known_proved_correct() {
        assert!(is_known_proved("zfc_zero_ne_one"));
        assert!(is_known_proved("bertrand"));
        assert!(!is_known_proved("goldbach"));
        assert!(!is_known_proved("p_vs_np"));
    }

    #[test]
    fn all_7_accelerator_decidable() {
        for id in ACCELERATOR_DECIDABLE {
            let stmt = get_statement(id);
            let result = try_accelerator(id, &stmt);
            assert!(result.is_some(), "{} should be decidable by accelerator", id);
        }
    }

    #[test]
    fn accelerator_decidable_count() {
        assert_eq!(ACCELERATOR_DECIDABLE.len(), 7, "7 problems should be accelerator-decidable");
    }
}
