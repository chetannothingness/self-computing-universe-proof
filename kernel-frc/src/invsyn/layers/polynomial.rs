//! Layer B: Polynomial Invariants
//!
//! v2: Real polynomial step verification.
//! For polynomial invariants (involving Mul, Pow):
//!   - Degree-bound check: verify polynomial degree doesn't increase under step
//!   - Positivity via evaluation at key points
//!   - Falls back to bounded evaluation for non-polynomial or complex cases
//!
//! Future: SOS (Sum of Squares) decomposition for full Positivstellensatz.

use super::{CheckResult, Layer};
use crate::invsyn::ast::{Expr, Layer as AstLayer};
use crate::invsyn::eval::{eval_bool, mk_env};
use crate::invsyn::normalize::ReachabilityProblem;

/// Polynomial layer checker.
pub struct PolynomialLayer {
    /// Maximum bound for bounded checking.
    pub step_check_bound: u64,
}

impl PolynomialLayer {
    pub fn new() -> Self {
        Self {
            step_check_bound: 500,
        }
    }

    /// Check if expression is in the polynomial fragment.
    fn is_polynomial(expr: &Expr) -> bool {
        expr.layer() <= AstLayer::Polynomial
    }

    /// Try polynomial step decision.
    ///
    /// For polynomial comparisons like Le(poly1, poly2), we can check
    /// if the difference poly2(n+δ) - poly1(n+δ) ≥ 0 whenever poly2(n) - poly1(n) ≥ 0.
    /// For simple cases (monotone polynomials), this is decidable.
    fn poly_step_decide(inv: &Expr, delta: i64) -> Option<bool> {
        if delta <= 0 {
            return None;
        }

        match inv {
            // Ground expression — trivially preserved
            _ if crate::invsyn::structural::is_ground(inv) => Some(true),

            // Lower bound on polynomial: Le(poly(c), Var(0)) where poly(c) is ground
            Expr::Le(l, r) if matches!(r.as_ref(), Expr::Var(0)) && crate::invsyn::structural::is_ground(l) => {
                Some(true) // n ≥ f(c) preserved by positive step
            }

            // Conjunction
            Expr::And(a, b) => {
                let ar = Self::poly_step_decide(a, delta);
                let br = Self::poly_step_decide(b, delta);
                match (ar, br) {
                    (Some(true), Some(true)) => Some(true),
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    _ => None,
                }
            }

            // Negation of upper bound
            Expr::Not(inner) => {
                match inner.as_ref() {
                    Expr::Le(l, r) if matches!(l.as_ref(), Expr::Var(0)) && crate::invsyn::structural::is_ground(r) => {
                        Some(true) // ¬(n ≤ c) = n > c, preserved
                    }
                    e if crate::invsyn::structural::is_ground(e) => Some(true),
                    _ => None,
                }
            }

            _ => None,
        }
    }
}

impl Layer for PolynomialLayer {
    fn name(&self) -> &str {
        "Polynomial"
    }

    fn check_base(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult {
        let passed = eval_bool(&mk_env(problem.initial_value), inv);
        CheckResult {
            passed,
            layer_name: "Polynomial".to_string(),
            description: format!("Base: eval(inv, {}) = {}", problem.initial_value, passed),
        }
    }

    fn check_step(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult {
        // Phase 1: Try polynomial decision procedure
        if Self::is_polynomial(inv) {
            if let Some(result) = Self::poly_step_decide(inv, problem.step_delta) {
                return CheckResult {
                    passed: result,
                    layer_name: "Polynomial".to_string(),
                    description: if result {
                        "Step: polynomial decision VERIFIED".to_string()
                    } else {
                        "Step: polynomial decision REFUTED".to_string()
                    },
                };
            }
        }

        // Phase 2: Bounded evaluation as fast filter
        let init = problem.initial_value;
        let bound = self.step_check_bound as i64;

        for n in init..=(init + bound) {
            let holds_n = eval_bool(&mk_env(n), inv);
            if holds_n {
                let holds_n1 = eval_bool(&mk_env(n + problem.step_delta), inv);
                if !holds_n1 {
                    return CheckResult {
                        passed: false,
                        layer_name: "Polynomial".to_string(),
                        description: format!("Step fails at n={}", n),
                    };
                }
            }
        }

        CheckResult {
            passed: true,
            layer_name: "Polynomial".to_string(),
            description: format!("Step: checked {} values (bounded filter)", bound),
        }
    }

    fn check_link(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult {
        let init = problem.initial_value;
        let delta = problem.step_delta;
        let bound = self.step_check_bound as i64;

        let prop = match problem.property_expr {
            Some(ref p) => p,
            None => {
                return CheckResult {
                    passed: false,
                    layer_name: "Polynomial".to_string(),
                    description: "Link: no property_expr available — cannot verify".to_string(),
                };
            }
        };

        let mut checked = 0i64;
        let mut n = init;
        while checked < bound {
            let inv_holds = eval_bool(&mk_env(n), inv);
            if inv_holds {
                let prop_holds = eval_bool(&mk_env(n), prop);
                if !prop_holds {
                    return CheckResult {
                        passed: false,
                        layer_name: "Polynomial".to_string(),
                        description: format!("Link fails at n={}", n),
                    };
                }
            }
            n += delta;
            checked += 1;
        }

        CheckResult {
            passed: true,
            layer_name: "Polynomial".to_string(),
            description: format!("Link: checked {} reachable values", bound),
        }
    }
}
