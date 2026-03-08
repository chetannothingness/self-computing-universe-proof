//! Prefix invariant detection and rejection.
//!
//! A prefix invariant `I(n) = ∀m≤n, P(m)` is banned as a proof candidate
//! for `∀n, P(n)` because its step obligation `∀n, (∀m≤n, P(m)) → (∀m≤n+1, P(m))`
//! reduces to `P(n+1)`, which IS the conjecture itself. The kernel must find
//! structurally independent invariants.

use crate::invsyn::ast::Expr;
use crate::invsyn::structural::contains_var;

/// Check if an invariant expression is a prefix invariant of the form
/// `ForallBounded(lo, Var(0), body)` where the body references the bound variable.
///
/// A prefix invariant wraps the property in a universal quantifier up to n,
/// making the step obligation equivalent to the original conjecture.
pub fn is_prefix_invariant(inv: &Expr, prop: &Expr) -> bool {
    match inv {
        // Direct prefix: ForallBounded(lo, Var(0), body)
        // where body is essentially P applied to the bound variable.
        Expr::ForallBounded(lo, hi, body) => {
            // hi must reference Var(0) (the outer variable n)
            if !matches!(hi.as_ref(), Expr::Var(0)) {
                return false;
            }
            // lo must be ground relative to outer (constant lower bound)
            if contains_var(lo, 0) {
                return false;
            }
            // The body should "look like" the property applied to the bound variable.
            // Under the binder, the bound variable is Var(0) and the outer n is Var(1).
            // If the body is structurally similar to prop with Var(0) replaced,
            // this is a prefix invariant.
            is_property_under_binder(body, prop)
        }

        // Conjunction where one conjunct is a prefix invariant
        Expr::And(a, b) => {
            is_prefix_invariant(a, prop) || is_prefix_invariant(b, prop)
        }

        _ => false,
    }
}

/// Check if `body` (under a binder) represents `prop` applied to the bound variable.
///
/// Under the ForallBounded binder:
/// - Var(0) = the bound variable m
/// - Var(1) = the outer variable n (shifted)
///
/// We check if body matches prop with Var(0) (the free var in prop) replaced
/// by Var(0) (the bound var). Since prop has Var(0) as its free variable
/// and under the binder that becomes Var(1), we check if body equals
/// prop with indices adjusted.
fn is_property_under_binder(body: &Expr, prop: &Expr) -> bool {
    // Simple structural check: if prop uses Var(0) and body uses Var(0)
    // in the same positions, it's a prefix invariant.
    //
    // The exact check: body should equal prop (both use Var(0) to refer
    // to their respective variables). Under the binder, Var(0) in body
    // refers to the bound variable m, while Var(0) in prop refers to n.
    // If they have the same structure, the body is P(m) which makes
    // the invariant ∀m≤n, P(m) — exactly a prefix invariant.
    body == prop
}

/// Check if an invariant's step obligation is independent of the property.
///
/// Returns `true` if the step can be proved without assuming P(n+1),
/// i.e., the invariant is NOT a prefix invariant disguise.
pub fn step_is_independent(inv: &Expr, prop: &Expr) -> bool {
    !is_prefix_invariant(inv, prop)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_prefix_forall_bounded() {
        // I(n) = ∀m ≤ n, P(m) where P(m) = IsPrime(m)
        let prop = Expr::IsPrime(Box::new(Expr::Var(0)));
        let inv = Expr::ForallBounded(
            Box::new(Expr::Const(0)),
            Box::new(Expr::Var(0)),
            Box::new(Expr::IsPrime(Box::new(Expr::Var(0)))),
        );
        assert!(is_prefix_invariant(&inv, &prop));
    }

    #[test]
    fn detect_prefix_in_conjunction() {
        let prop = Expr::IsPrime(Box::new(Expr::Var(0)));
        let prefix = Expr::ForallBounded(
            Box::new(Expr::Const(0)),
            Box::new(Expr::Var(0)),
            Box::new(Expr::IsPrime(Box::new(Expr::Var(0)))),
        );
        let conj = Expr::And(
            Box::new(Expr::Le(Box::new(Expr::Const(2)), Box::new(Expr::Var(0)))),
            Box::new(prefix),
        );
        assert!(is_prefix_invariant(&conj, &prop));
    }

    #[test]
    fn non_prefix_invariant() {
        // I(n) = n ≥ 4 is NOT a prefix invariant
        let prop = Expr::IsPrime(Box::new(Expr::Var(0)));
        let inv = Expr::Le(Box::new(Expr::Const(4)), Box::new(Expr::Var(0)));
        assert!(!is_prefix_invariant(&inv, &prop));
    }

    #[test]
    fn non_prefix_constant() {
        let prop = Expr::Const(1);
        let inv = Expr::Const(1);
        assert!(!is_prefix_invariant(&inv, &prop));
    }

    #[test]
    fn non_prefix_different_body() {
        // ForallBounded but body is different from prop
        let prop = Expr::IsPrime(Box::new(Expr::Var(0)));
        let inv = Expr::ForallBounded(
            Box::new(Expr::Const(0)),
            Box::new(Expr::Var(0)),
            // Body is Le, not IsPrime — not a prefix of the property
            Box::new(Expr::Le(Box::new(Expr::Const(0)), Box::new(Expr::Var(0)))),
        );
        assert!(!is_prefix_invariant(&inv, &prop));
    }

    #[test]
    fn step_independence_check() {
        let prop = Expr::IsPrime(Box::new(Expr::Var(0)));
        let prefix = Expr::ForallBounded(
            Box::new(Expr::Const(0)),
            Box::new(Expr::Var(0)),
            Box::new(Expr::IsPrime(Box::new(Expr::Var(0)))),
        );
        let range = Expr::Le(Box::new(Expr::Const(4)), Box::new(Expr::Var(0)));

        assert!(!step_is_independent(&prefix, &prop));
        assert!(step_is_independent(&range, &prop));
    }
}
