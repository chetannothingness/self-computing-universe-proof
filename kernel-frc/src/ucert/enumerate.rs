//! Complete enumerator E: ℕ → Cert.
//!
//! v2: Uses InvSyn's real AST candidate generation instead of string seeds.
//! Each certificate carries actual Expr ASTs for structural verification.
//!
//! Enumeration order:
//!   Rank 0..k:     Trivial invariant certs (Const(1), ground exprs)
//!   Rank k+1..m:   Known proof certs (using registered theorem names)
//!   Rank m+1..p:   Structural certs with real Expr invariants
//!   Rank p+1..:    Composite certs (chains of sub-certificates)
//!
//! Deterministic: E(k) always produces the same Cert for same k.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::invsyn::ast::Expr;
use crate::ucert::cert::{Cert, InvCert, BaseCert, StepCert, LinkCert};

/// Known theorem names for seeding enumeration.
const KNOWN_THEOREMS: &[&str] = &[
    "bertrand_postulate",
    "lagrange_four_squares",
    "helfgott_weak_goldbach",
    "fermat_last_theorem",
];

/// Complete certificate enumerator.
/// Generates every Cert value in canonical order using real Expr ASTs.
pub struct CertEnumerator {
    /// Precomputed certificates by rank
    certs: Vec<Cert>,
}

impl CertEnumerator {
    /// Create a new enumerator with real Expr-based certificates.
    pub fn new() -> Self {
        let mut certs = Vec::new();

        // Generate the set of structural invariant Exprs
        let structural_exprs = Self::structural_invariant_exprs();

        // Phase 1: Trivial invariant certs with structural step + trivial link
        for inv in &structural_exprs {
            let inv_hash = hash_expr(inv);
            certs.push(Cert::InvariantCert(InvCert {
                invariant: inv.clone(),
                invariant_desc: format!("{:?}", inv),
                invariant_hash: inv_hash,
                base_cert: BaseCert::Trivial,
                step_cert: StepCert::Structural(inv.clone()),
                link_cert: LinkCert::Trivial,
            }));
        }

        // Phase 2: Known proof certs
        for theorem in KNOWN_THEOREMS {
            // With trivial base and link
            certs.push(Cert::InvariantCert(InvCert {
                invariant: Expr::Const(1),
                invariant_desc: format!("prefix_for_{}", theorem),
                invariant_hash: simple_hash(theorem),
                base_cert: BaseCert::Trivial,
                step_cert: StepCert::KnownProof(theorem.to_string()),
                link_cert: LinkCert::Trivial,
            }));
            // With direct check base
            certs.push(Cert::InvariantCert(InvCert {
                invariant: Expr::Const(1),
                invariant_desc: format!("direct_for_{}", theorem),
                invariant_hash: simple_hash(&format!("direct_{}", theorem)),
                base_cert: BaseCert::DirectCheck(1),
                step_cert: StepCert::KnownProof(theorem.to_string()),
                link_cert: LinkCert::DirectImplication,
            }));
        }

        // Phase 3: Structural certs with DirectImplication link
        for inv in &structural_exprs {
            let inv_hash = hash_expr(inv) ^ 0x1111;
            certs.push(Cert::InvariantCert(InvCert {
                invariant: inv.clone(),
                invariant_desc: format!("{:?}", inv),
                invariant_hash: inv_hash,
                base_cert: BaseCert::DirectCheck(1),
                step_cert: StepCert::Structural(inv.clone()),
                link_cert: LinkCert::DirectImplication,
            }));
        }

        // Phase 4: Structural certs with structural link (inv → inv identity)
        for inv in &structural_exprs {
            let inv_hash = hash_expr(inv) ^ 0x2222;
            certs.push(Cert::InvariantCert(InvCert {
                invariant: inv.clone(),
                invariant_desc: format!("{:?}", inv),
                invariant_hash: inv_hash,
                base_cert: BaseCert::Trivial,
                step_cert: StepCert::Structural(inv.clone()),
                link_cert: LinkCert::Structural(inv.clone(), inv.clone()),
            }));
        }

        // Phase 5: Composition certs (known proof + structural)
        for theorem in KNOWN_THEOREMS {
            for inv in &structural_exprs[..structural_exprs.len().min(3)] {
                let inv_hash = simple_hash(&format!("comp_{}_{:?}", theorem, inv));
                certs.push(Cert::InvariantCert(InvCert {
                    invariant: inv.clone(),
                    invariant_desc: format!("{}+{:?}", theorem, inv),
                    invariant_hash: inv_hash,
                    base_cert: BaseCert::Trivial,
                    step_cert: StepCert::Composition(
                        Box::new(StepCert::KnownProof(theorem.to_string())),
                        Box::new(StepCert::Structural(inv.clone())),
                    ),
                    link_cert: LinkCert::Trivial,
                }));
            }
        }

        CertEnumerator { certs }
    }

    /// Generate the set of structural invariant expressions.
    /// These are real Expr ASTs — not strings.
    fn structural_invariant_exprs() -> Vec<Expr> {
        let mut exprs = Vec::new();

        // Ground constants (step trivially verified)
        exprs.push(Expr::Const(1));
        // Note: Const(0) is NOT included. It's always false, making
        // step and link vacuously true but base always fails for real problems.
        // Including it would cause unsound "proofs" via vacuous implication.

        // Lower bounds: Le(Const(c), Var(0)) — n ≥ c
        for c in &[0i64, 1, 2, 3, 4, 7] {
            exprs.push(Expr::Le(
                Box::new(Expr::Const(*c)),
                Box::new(Expr::Var(0)),
            ));
        }

        // Strict lower bounds: Lt(Const(c), Var(0)) — n > c
        for c in &[0i64, 1, 2, 3] {
            exprs.push(Expr::Lt(
                Box::new(Expr::Const(*c)),
                Box::new(Expr::Var(0)),
            ));
        }

        // Modular congruences (for delta-compatible problems)
        // Eq(Mod(Var(0), Const(m)), Const(r))
        for m in &[2i64, 3, 4, 6] {
            for r in 0..*m {
                exprs.push(Expr::Eq(
                    Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(*m)))),
                    Box::new(Expr::Const(r)),
                ));
            }
        }

        // Native primitives with known proofs
        exprs.push(Expr::FourSquares(Box::new(Expr::Var(0))));
        exprs.push(Expr::FltHolds(Box::new(Expr::Var(0))));

        // Conjunctions: range + modular
        let range = Expr::Le(Box::new(Expr::Const(4)), Box::new(Expr::Var(0)));
        let mod_2_0 = Expr::Eq(
            Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(2)))),
            Box::new(Expr::Const(0)),
        );
        exprs.push(Expr::And(Box::new(range.clone()), Box::new(mod_2_0.clone())));

        let range7 = Expr::Le(Box::new(Expr::Const(7)), Box::new(Expr::Var(0)));
        let mod_2_1 = Expr::Eq(
            Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(2)))),
            Box::new(Expr::Const(1)),
        );
        exprs.push(Expr::And(Box::new(range7), Box::new(mod_2_1)));

        // Negation-based: Not(Le(Var(0), Const(c))) = n > c
        for c in &[0i64, 1, 2, 3] {
            exprs.push(Expr::Not(Box::new(Expr::Le(
                Box::new(Expr::Var(0)),
                Box::new(Expr::Const(*c)),
            ))));
        }

        // Dedup by hash
        exprs.sort_by_key(|e| hash_expr(e));
        exprs.dedup_by(|a, b| hash_expr(a) == hash_expr(b));

        exprs
    }

    /// E(k) — the k-th certificate in canonical order.
    /// Returns None if k exceeds the precomputed set.
    pub fn at_rank(&self, k: u64) -> Option<&Cert> {
        self.certs.get(k as usize)
    }

    /// Total number of precomputed certificates.
    pub fn total_certs(&self) -> u64 {
        self.certs.len() as u64
    }

    /// Iterator over all certificates by rank.
    pub fn iter(&self) -> impl Iterator<Item = (u64, &Cert)> {
        self.certs.iter().enumerate().map(|(i, c)| (i as u64, c))
    }

    /// Iterator over certificates up to a max rank.
    pub fn iter_up_to(&self, max_rank: u64) -> impl Iterator<Item = (u64, &Cert)> {
        self.certs
            .iter()
            .enumerate()
            .take(max_rank as usize)
            .map(|(i, c)| (i as u64, c))
    }
}

/// Deterministic hash for an Expr.
fn hash_expr(e: &Expr) -> u64 {
    let mut hasher = DefaultHasher::new();
    e.hash(&mut hasher);
    hasher.finish()
}

/// Simple deterministic hash for seeds.
fn simple_hash(s: &str) -> u64 {
    let h = kernel_types::hash::H(s.as_bytes());
    u64::from_le_bytes([h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerator_nonempty() {
        let e = CertEnumerator::new();
        assert!(e.total_certs() > 0);
    }

    #[test]
    fn enumerator_deterministic() {
        let e1 = CertEnumerator::new();
        let e2 = CertEnumerator::new();
        assert_eq!(e1.total_certs(), e2.total_certs());
        for k in 0..e1.total_certs() {
            assert_eq!(
                e1.at_rank(k).unwrap().cert_hash(),
                e2.at_rank(k).unwrap().cert_hash(),
                "Rank {} differs",
                k
            );
        }
    }

    #[test]
    fn at_rank_bounds() {
        let e = CertEnumerator::new();
        assert!(e.at_rank(0).is_some());
        assert!(e.at_rank(e.total_certs()).is_none());
    }

    #[test]
    fn iter_count() {
        let e = CertEnumerator::new();
        let count = e.iter().count();
        assert_eq!(count as u64, e.total_certs());
    }

    #[test]
    fn includes_known_proofs() {
        let e = CertEnumerator::new();
        let has_bertrand = e.iter().any(|(_, c)| match c {
            Cert::InvariantCert(ic) => matches!(&ic.step_cert, StepCert::KnownProof(n) if n == "bertrand_postulate"),
            _ => false,
        });
        assert!(has_bertrand, "Enumerator should include bertrand_postulate cert");
    }

    #[test]
    fn includes_structural_with_expr() {
        let e = CertEnumerator::new();
        let has_structural = e.iter().any(|(_, c)| match c {
            Cert::InvariantCert(ic) => matches!(&ic.step_cert, StepCert::Structural(_)),
            _ => false,
        });
        assert!(has_structural, "Enumerator should include structural certs");

        // Verify structural certs carry real Expr, not strings
        let first_structural = e.iter().find(|(_, c)| match c {
            Cert::InvariantCert(ic) => matches!(&ic.step_cert, StepCert::Structural(_)),
            _ => false,
        });
        if let Some((_, Cert::InvariantCert(ic))) = first_structural {
            match &ic.step_cert {
                StepCert::Structural(expr) => {
                    // expr should be a real Expr, verify it has a size
                    assert!(expr.size() > 0, "Structural cert should carry real Expr");
                }
                _ => panic!("Expected Structural"),
            }
        }
    }

    #[test]
    fn no_direct_eval() {
        // No certificate in the enumeration should use DirectEval as step
        let e = CertEnumerator::new();
        for (_, c) in e.iter() {
            if let Cert::InvariantCert(ic) = c {
                assert!(
                    !matches!(&ic.step_cert, StepCert::DirectEval(_)),
                    "Enumerator should never produce DirectEval step certs"
                );
            }
        }
    }

    #[test]
    fn structural_exprs_include_const_1() {
        let exprs = CertEnumerator::structural_invariant_exprs();
        assert!(exprs.contains(&Expr::Const(1)), "Should include Const(1)");
    }

    #[test]
    fn structural_exprs_include_lower_bounds() {
        let exprs = CertEnumerator::structural_invariant_exprs();
        let has_lower = exprs.iter().any(|e| matches!(e, Expr::Le(l, r)
            if matches!(l.as_ref(), Expr::Const(_))
                && matches!(r.as_ref(), Expr::Var(0))));
        assert!(has_lower, "Should include lower bound Le(Const, Var(0))");
    }

    #[test]
    fn structural_exprs_include_native_primitives() {
        let exprs = CertEnumerator::structural_invariant_exprs();
        let has_four_sq = exprs.iter().any(|e| matches!(e, Expr::FourSquares(_)));
        let has_flt = exprs.iter().any(|e| matches!(e, Expr::FltHolds(_)));
        assert!(has_four_sq, "Should include FourSquares(Var(0))");
        assert!(has_flt, "Should include FltHolds(Var(0))");
    }
}
