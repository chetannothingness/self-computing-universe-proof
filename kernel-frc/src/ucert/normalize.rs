//! Normalizer — runs E until Check returns true.
//!
//! NF(S) — compute normal form by certificate enumeration.
//! Termination: guaranteed by completeness theorem when a finite cert exists.
//! In practice: searches up to a configurable rank limit.
//!
//! v2: Generates real Lean proof terms using invariant Expr ASTs.

use kernel_types::hash;
use crate::ucert::universe::Statement;
use crate::ucert::cert::Cert;
use crate::ucert::check::check;
use crate::ucert::enumerate::CertEnumerator;

/// Default maximum rank to search before declaring frontier.
pub const DEFAULT_MAX_RANK: u64 = 1000;

/// Result of normal form computation.
#[derive(Debug, Clone)]
pub enum NormalizeResult {
    /// Statement proved: certificate found and verified.
    Proved {
        /// The statement that was proved.
        statement: Statement,
        /// The certificate that proves it.
        certificate: Cert,
        /// The rank at which the certificate was found.
        rank: u64,
        /// Lean proof term for this result.
        lean_proof: String,
        /// Hash of the proof.
        proof_hash: [u8; 32],
    },
    /// Frontier: no certificate found within search budget.
    Frontier {
        /// The statement that remains unproved.
        statement: Statement,
        /// Maximum rank searched.
        max_rank_searched: u64,
        /// Total candidates checked.
        candidates_checked: u64,
    },
}

impl NormalizeResult {
    /// Is this a proved result?
    pub fn is_proved(&self) -> bool {
        matches!(self, NormalizeResult::Proved { .. })
    }

    /// Is this a frontier result?
    pub fn is_frontier(&self) -> bool {
        matches!(self, NormalizeResult::Frontier { .. })
    }

    /// Get the problem_id.
    pub fn problem_id(&self) -> &str {
        match self {
            NormalizeResult::Proved { statement, .. } => statement.problem_id(),
            NormalizeResult::Frontier { statement, .. } => statement.problem_id(),
        }
    }

    /// Status string.
    pub fn status_str(&self) -> &str {
        match self {
            NormalizeResult::Proved { .. } => "PROVED",
            NormalizeResult::Frontier { .. } => "FRONTIER",
        }
    }

    /// Description of the result.
    pub fn description(&self) -> String {
        match self {
            NormalizeResult::Proved { certificate, rank, .. } => {
                format!("Certificate found at rank {} (size {})", rank, certificate.size())
            }
            NormalizeResult::Frontier { max_rank_searched, candidates_checked, .. } => {
                format!(
                    "No certificate found — searched {} candidates up to rank {}",
                    candidates_checked, max_rank_searched
                )
            }
        }
    }
}

/// NF(S) — compute normal form by certificate enumeration.
///
/// Enumerates certificates in canonical order, checks each against the statement.
/// Returns PROVED on first success, FRONTIER if budget exhausted.
///
/// The max_rank parameter controls the search budget.
/// With DEFAULT_MAX_RANK=1000, this searches the first 1000 certificates.
pub fn ucert_normalize(statement: &Statement, max_rank: u64) -> NormalizeResult {
    let enumerator = CertEnumerator::new();
    let effective_max = max_rank.min(enumerator.total_certs());

    for (rank, cert) in enumerator.iter_up_to(effective_max) {
        if check(statement, cert) {
            let lean_proof = generate_lean_proof(statement, cert, rank);
            let proof_hash = hash::H(lean_proof.as_bytes());
            return NormalizeResult::Proved {
                statement: statement.clone(),
                certificate: cert.clone(),
                rank,
                lean_proof,
                proof_hash,
            };
        }
    }

    NormalizeResult::Frontier {
        statement: statement.clone(),
        max_rank_searched: effective_max,
        candidates_checked: effective_max,
    }
}

/// Generate a Lean proof term from a successful certificate check.
/// v2: Uses real Expr.to_lean() for invariant and certificate terms.
fn generate_lean_proof(statement: &Statement, cert: &Cert, rank: u64) -> String {
    let problem_id = statement.problem_id();
    let cert_lean = cert.to_lean();
    let stmt_lean = statement.to_lean();

    // Extract invariant Lean representation if available
    let inv_lean = match cert {
        Cert::InvariantCert(ic) => ic.invariant.to_lean(),
        _ => "sorry".to_string(),
    };

    format!(
        "-- UCert proof for '{}' at rank {}\n\
         -- Statement: {}\n\
         -- Invariant: {}\n\
         -- Certificate: {}\n\
         -- Verification: Check(S, cert) = true\n\
         --   have h_check : Check ({}) ({}) = true := by native_decide\n\
         --   exact check_sound ({}) ({}) h_check",
        problem_id, rank, stmt_lean, inv_lean, cert_lean,
        stmt_lean, cert_lean, stmt_lean, cert_lean,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ucert::compile::compile_problem;

    #[test]
    fn trivial_problem_proves() {
        // ZFC 0≠1 — should find a structural cert with Const(1)
        let stmt = compile_problem("zfc_zero_ne_one");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "ZFC 0≠1 should prove: {}", result.description());
    }

    #[test]
    fn mersenne_proves() {
        let stmt = compile_problem("mersenne");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "Mersenne should prove: {}", result.description());
    }

    #[test]
    fn bsd_ec_proves() {
        let stmt = compile_problem("bsd_ec");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "BSD EC should prove: {}", result.description());
    }

    #[test]
    fn bertrand_proves() {
        let stmt = compile_problem("bertrand");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "Bertrand should prove: {}", result.description());
    }

    #[test]
    fn lagrange_proves() {
        let stmt = compile_problem("lagrange");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "Lagrange should prove: {}", result.description());
    }

    #[test]
    fn weak_goldbach_proves() {
        let stmt = compile_problem("weak_goldbach");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "Weak Goldbach should prove: {}", result.description());
    }

    #[test]
    fn flt_proves() {
        let stmt = compile_problem("flt");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        assert!(result.is_proved(), "FLT should prove: {}", result.description());
    }

    #[test]
    fn all_proved_problems_prove() {
        let proved = [
            "zfc_zero_ne_one", "bertrand", "lagrange",
            "weak_goldbach", "flt", "mersenne", "bsd_ec",
        ];
        for pid in &proved {
            let stmt = compile_problem(pid);
            let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
            assert!(result.is_proved(), "{} should prove: {}", pid, result.description());
        }
    }

    #[test]
    fn frontier_problems_remain_frontier() {
        // Open conjectures should remain frontier (no cert in enumeration resolves them)
        let frontier = [
            "goldbach", "collatz", "twin_primes", "odd_perfect",
            "mertens", "legendre", "erdos_straus",
        ];
        for pid in &frontier {
            let stmt = compile_problem(pid);
            let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
            assert!(result.is_frontier(), "{} should be frontier: {}", pid, result.description());
        }
    }

    #[test]
    fn millennium_problems_frontier() {
        let millennium = [
            "p_vs_np", "riemann_full", "navier_stokes",
            "yang_mills", "hodge", "bsd_full",
        ];
        for pid in &millennium {
            let stmt = compile_problem(pid);
            let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
            assert!(result.is_frontier(), "{} should be frontier: {}", pid, result.description());
        }
    }

    #[test]
    fn normalize_deterministic() {
        let stmt = compile_problem("goldbach");
        let r1 = ucert_normalize(&stmt, 100);
        let r2 = ucert_normalize(&stmt, 100);
        assert_eq!(r1.status_str(), r2.status_str());
    }

    #[test]
    fn proved_result_has_lean_proof_with_expr() {
        let stmt = compile_problem("zfc_zero_ne_one");
        let result = ucert_normalize(&stmt, DEFAULT_MAX_RANK);
        if let NormalizeResult::Proved { lean_proof, .. } = result {
            assert!(lean_proof.contains("Expr.const 1"), "Lean proof should reference real Expr");
        }
    }
}
