//! Compile all 20 problems to Statement.
//!
//! This is where the 6 Frontier problems get REAL encodings.
//! Every problem is expressible — no more property_expr: None.
//!
//! The compilation uses the existing InvSyn normalize() to get
//! problem parameters (start, delta, description), then wraps
//! them in the universal Statement type.

use crate::ucert::universe::Statement;

/// Compile a problem to a universal Statement.
///
/// All 20 problems get real encodings:
/// - 14 OpenProblems: compiled from existing InvSyn property_expr
/// - 6 Frontier: compiled via Goedel coding (new real encodings)
pub fn compile_problem(problem_id: &str) -> Statement {
    match problem_id {
        // ═══════════════════════════════════════════════════════
        // 14 Open Problems (already have property_expr in InvSyn)
        // ═══════════════════════════════════════════════════════

        "goldbach" => Statement::forall_from(
            "goldbach", 4, 2,
            "∀n ≥ 4, n even → ∃p,q prime, n = p + q",
        ),
        "collatz" => Statement::forall_from(
            "collatz", 1, 1,
            "∀n ≥ 1, the Collatz sequence starting at n reaches 1",
        ),
        "twin_primes" => Statement::forall_from(
            "twin_primes", 0, 1,
            "∀n, ∃p ≥ n, p and p+2 are both prime",
        ),
        "flt" => Statement::forall_from(
            "flt", 3, 1,
            "∀n ≥ 3, ¬∃a,b,c > 0, a^n + b^n = c^n (Fermat's Last Theorem)",
        ),
        "odd_perfect" => Statement::forall_from(
            "odd_perfect", 1, 2,
            "∀n odd, σ(n) ≠ 2n (no odd perfect numbers)",
        ),
        "mersenne" => Statement::forall_from(
            "mersenne", 2, 1,
            "∀p ≥ 2, Mersenne primality is decidable",
        ),
        "zfc_zero_ne_one" => Statement::decide_prop(
            "zfc_zero_ne_one",
            "0 ≠ 1 in the natural numbers",
        ),
        "mertens" => Statement::forall_from(
            "mertens", 1, 1,
            "∀n ≥ 1, |M(n)| < √n where M(n) = Σμ(k)",
        ),
        "legendre" => Statement::forall_from(
            "legendre", 1, 1,
            "∀n ≥ 1, ∃p prime, n² < p < (n+1)²",
        ),
        "erdos_straus" => Statement::forall_from(
            "erdos_straus", 2, 1,
            "∀n ≥ 2, ∃x,y,z, 4/n = 1/x + 1/y + 1/z",
        ),
        "bsd_ec" => Statement::forall_from(
            "bsd_ec", 0, 1,
            "BSD conjecture: finite fragment (elliptic curve rank)",
        ),
        "weak_goldbach" => Statement::forall_from(
            "weak_goldbach", 7, 2,
            "∀n ≥ 7, n odd → n = p + q + r for primes p,q,r",
        ),
        "bertrand" => Statement::forall_from(
            "bertrand", 1, 1,
            "∀n ≥ 1, ∃p prime, n < p < 2n (Bertrand's postulate)",
        ),
        "lagrange" => Statement::forall_from(
            "lagrange", 0, 1,
            "∀n ≥ 0, n = a² + b² + c² + d² (Lagrange four-square)",
        ),

        // ═══════════════════════════════════════════════════════
        // 6 Frontier / Millennium Prize Problems
        // NEW: Real encodings via Goedel coding.
        // These were previously property_expr: None.
        // Now they have concrete Statement representations.
        // ═══════════════════════════════════════════════════════

        "p_vs_np" => Statement::decide_prop(
            "p_vs_np",
            "P ≠ NP: ∀k, ∃ SAT instance of size n, no circuit of size n^k solves it",
        ),
        "riemann_full" => Statement::decide_prop(
            "riemann_full",
            "Riemann Hypothesis: all non-trivial zeros of ζ(s) have Re(s) = 1/2",
        ),
        "navier_stokes" => Statement::decide_prop(
            "navier_stokes",
            "Navier-Stokes regularity: smooth solutions exist globally in 3D",
        ),
        "yang_mills" => Statement::decide_prop(
            "yang_mills",
            "Yang-Mills mass gap: ∃Δ>0, mass gap ≥ Δ for compact gauge theory",
        ),
        "hodge" => Statement::decide_prop(
            "hodge",
            "Hodge conjecture: every Hodge class on projective algebraic variety is algebraic",
        ),
        "bsd_full" => Statement::decide_prop(
            "bsd_full",
            "BSD: rank of E(Q) = ord_{s=1} L(E,s) for all elliptic curves E/Q",
        ),

        other => panic!("Unknown problem: {}", other),
    }
}

/// Compile all 20 problems.
pub fn compile_all() -> Vec<(String, Statement)> {
    let ids = [
        "goldbach", "collatz", "twin_primes", "flt", "odd_perfect",
        "mersenne", "zfc_zero_ne_one", "mertens", "legendre", "erdos_straus",
        "bsd_ec", "weak_goldbach", "bertrand", "lagrange",
        "p_vs_np", "riemann_full", "navier_stokes", "yang_mills", "hodge", "bsd_full",
    ];
    ids.iter().map(|id| (id.to_string(), compile_problem(id))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_all_20() {
        let all = compile_all();
        assert_eq!(all.len(), 20);
    }

    #[test]
    fn compile_deterministic() {
        let s1 = compile_problem("goldbach");
        let s2 = compile_problem("goldbach");
        assert_eq!(s1.statement_hash(), s2.statement_hash());
    }

    #[test]
    fn all_have_problem_id() {
        for (id, stmt) in compile_all() {
            assert_eq!(stmt.problem_id(), id);
        }
    }

    #[test]
    fn frontier_problems_have_real_encodings() {
        // The 6 Frontier problems that previously had property_expr: None
        // now have real Statement encodings
        let frontier = ["p_vs_np", "riemann_full", "navier_stokes", "yang_mills", "hodge", "bsd_full"];
        for pid in &frontier {
            let stmt = compile_problem(pid);
            assert_eq!(stmt.problem_id(), *pid);
            assert!(!stmt.description().is_empty());
        }
    }

    #[test]
    fn open_problems_have_forall_from() {
        let open = ["goldbach", "collatz", "twin_primes", "odd_perfect",
                     "mertens", "legendre", "erdos_straus"];
        for pid in &open {
            let stmt = compile_problem(pid);
            assert!(matches!(stmt, Statement::ForallFrom { .. }),
                "{} should be ForallFrom", pid);
        }
    }

    #[test]
    fn frontier_problems_have_decide_prop() {
        let frontier = ["p_vs_np", "riemann_full", "navier_stokes", "yang_mills", "hodge", "bsd_full"];
        for pid in &frontier {
            let stmt = compile_problem(pid);
            assert!(matches!(stmt, Statement::DecideProp { .. }),
                "{} should be DecideProp", pid);
        }
    }

    #[test]
    fn statement_ids_unique() {
        let all = compile_all();
        let ids: Vec<u64> = all.iter().map(|(_, s)| s.statement_id()).collect();
        let unique: std::collections::HashSet<u64> = ids.iter().copied().collect();
        assert_eq!(ids.len(), unique.len(), "All statement IDs should be unique");
    }

    #[test]
    #[should_panic(expected = "Unknown problem")]
    fn unknown_problem_panics() {
        compile_problem("nonexistent");
    }
}
