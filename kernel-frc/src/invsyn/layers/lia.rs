//! Layer A: Linear Integer Arithmetic (LIA / Presburger)
//!
//! Decision procedure for linear integer arithmetic invariants.
//! v2: Real LIA decision for step obligation:
//!   - For Le/Lt lower bounds: I(n) → I(n+δ) when δ > 0 (coefficient comparison)
//!   - For modular congruences: n≡r (mod m) → (n+δ)≡r (mod m) when δ%m=0
//!   - For conjunctions: check both conjuncts
//!   - Falls back to bounded evaluation for non-LIA or complex cases.
//!
//! Base and Link use bounded evaluation as fast filter (NOT proof).

use super::{CheckResult, Layer};
use crate::invsyn::ast::{Expr, Layer as AstLayer};
use crate::invsyn::eval::{eval_bool, mk_env};
use crate::invsyn::normalize::ReachabilityProblem;

/// LIA layer checker.
pub struct LiaLayer {
    /// Maximum bound for bounded checking (fast filter).
    pub step_check_bound: u64,
}

impl LiaLayer {
    pub fn new() -> Self {
        Self {
            step_check_bound: 500,
        }
    }

    /// Check if an expression is in the LIA fragment.
    fn is_lia(expr: &Expr) -> bool {
        expr.layer() <= AstLayer::LIA
    }

    /// Real LIA step decision: does I(n) → I(n+δ) hold for all n?
    ///
    /// For LIA expressions, this can be decided by algebraic reasoning
    /// on the expression structure (coefficient comparison, modular arithmetic).
    fn lia_step_decide(inv: &Expr, delta: i64) -> Option<bool> {
        if delta <= 0 {
            return None;
        }

        match inv {
            // Lower bound: Le(Const(c), Var(0)) — n ≥ c → n+δ ≥ c (δ > 0)
            Expr::Le(l, r) => {
                if matches!(r.as_ref(), Expr::Var(0)) && Self::is_ground(l) {
                    Some(true) // Lower bound preserved by positive step
                } else if matches!(l.as_ref(), Expr::Var(0)) && Self::is_ground(r) {
                    Some(false) // Upper bound NOT preserved by positive step
                } else {
                    None // Complex — can't decide
                }
            }

            // Strict lower bound: Lt(Const(c), Var(0)) — n > c → n+δ > c
            Expr::Lt(l, r) => {
                if matches!(r.as_ref(), Expr::Var(0)) && Self::is_ground(l) {
                    Some(true)
                } else {
                    None
                }
            }

            // Modular congruence: Eq(Mod(Var(0), Const(m)), Const(r))
            // n ≡ r (mod m) → (n+δ) ≡ r (mod m) iff δ ≡ 0 (mod m)
            Expr::Eq(l, r) => {
                if let (Expr::Mod(inner_l, inner_r), Expr::Const(_r_val)) =
                    (l.as_ref(), r.as_ref())
                {
                    if let (Expr::Var(0), Expr::Const(m)) = (inner_l.as_ref(), inner_r.as_ref()) {
                        if *m > 0 {
                            return Some(delta % m == 0);
                        }
                    }
                }
                None
            }

            // Modular non-congruence: Ne(Mod(Var(0), Const(m)), Const(r))
            Expr::Ne(l, r) => {
                if let (Expr::Mod(inner_l, inner_r), Expr::Const(_r_val)) =
                    (l.as_ref(), r.as_ref())
                {
                    if let (Expr::Var(0), Expr::Const(m)) = (inner_l.as_ref(), inner_r.as_ref()) {
                        if *m > 0 {
                            return Some(delta % m == 0);
                        }
                    }
                }
                None
            }

            // Conjunction: And(A, B) — both must hold
            Expr::And(a, b) => {
                let a_result = Self::lia_step_decide(a, delta);
                let b_result = Self::lia_step_decide(b, delta);
                match (a_result, b_result) {
                    (Some(true), Some(true)) => Some(true),
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    _ => None,
                }
            }

            // Disjunction: Or(A, B) — both disjuncts must be step-closed
            Expr::Or(a, b) => {
                let a_result = Self::lia_step_decide(a, delta);
                let b_result = Self::lia_step_decide(b, delta);
                match (a_result, b_result) {
                    (Some(true), Some(true)) => Some(true),
                    (Some(false), _) | (_, Some(false)) => Some(false),
                    _ => None,
                }
            }

            // Ground expression (no variables) — trivially preserved
            _ if Self::is_ground(inv) => Some(true),

            // Negation patterns
            Expr::Not(inner) => {
                match inner.as_ref() {
                    // ¬(n ≤ c) = n > c: strict lower bound
                    Expr::Le(l, r) if matches!(l.as_ref(), Expr::Var(0)) && Self::is_ground(r) => {
                        Some(true)
                    }
                    // ¬(n < c) = n ≥ c: lower bound
                    Expr::Lt(l, r) if matches!(l.as_ref(), Expr::Var(0)) && Self::is_ground(r) => {
                        Some(true)
                    }
                    e if Self::is_ground(e) => Some(true),
                    _ => None,
                }
            }

            _ => None,
        }
    }

    /// Check if expression is ground (no free variables).
    fn is_ground(expr: &Expr) -> bool {
        crate::invsyn::structural::is_ground(expr)
    }
}

impl Layer for LiaLayer {
    fn name(&self) -> &str {
        "LIA"
    }

    fn check_base(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult {
        // Check the invariant at the initial state
        let passed = eval_bool(&mk_env(problem.initial_value), inv);
        CheckResult {
            passed,
            layer_name: "LIA".to_string(),
            description: format!(
                "Base: eval(inv, {}) = {}",
                problem.initial_value, passed
            ),
        }
    }

    fn check_step(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult {
        // Phase 1: Try real LIA decision procedure
        if Self::is_lia(inv) {
            if let Some(result) = Self::lia_step_decide(inv, problem.step_delta) {
                return CheckResult {
                    passed: result,
                    layer_name: "LIA".to_string(),
                    description: if result {
                        "Step: LIA decision procedure VERIFIED".to_string()
                    } else {
                        "Step: LIA decision procedure REFUTED".to_string()
                    },
                };
            }
        }

        // Phase 2: Fall back to bounded evaluation as fast filter
        let init = problem.initial_value;
        let bound = self.step_check_bound as i64;

        for n in init..=(init + bound) {
            let holds_n = eval_bool(&mk_env(n), inv);
            if holds_n {
                let holds_n1 = eval_bool(&mk_env(n + problem.step_delta), inv);
                if !holds_n1 {
                    return CheckResult {
                        passed: false,
                        layer_name: "LIA".to_string(),
                        description: format!(
                            "Step fails: I({}) = true but I({}) = false",
                            n, n + problem.step_delta
                        ),
                    };
                }
            }
        }

        CheckResult {
            passed: true,
            layer_name: "LIA".to_string(),
            description: format!(
                "Step: checked {} values from {} with delta {} (bounded filter, NOT proof)",
                bound, init, problem.step_delta
            ),
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
                    layer_name: "LIA".to_string(),
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
                        layer_name: "LIA".to_string(),
                        description: format!(
                            "Link fails: I({}) = true but P({}) = false",
                            n, n
                        ),
                    };
                }
            }
            n += delta;
            checked += 1;
        }

        CheckResult {
            passed: true,
            layer_name: "LIA".to_string(),
            description: format!(
                "Link: checked {} reachable values from {} with delta {}",
                bound, init, delta
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lia_fragment_check() {
        assert!(LiaLayer::is_lia(&Expr::Var(0)));
        assert!(LiaLayer::is_lia(&Expr::Add(
            Box::new(Expr::Var(0)),
            Box::new(Expr::Const(1))
        )));
        // mul of two variables is not LIA
        assert!(!LiaLayer::is_lia(&Expr::Mul(
            Box::new(Expr::Var(0)),
            Box::new(Expr::Var(1))
        )));
        // mul by const is LIA
        assert!(LiaLayer::is_lia(&Expr::Mul(
            Box::new(Expr::Const(2)),
            Box::new(Expr::Var(0))
        )));
    }

    #[test]
    fn lia_step_lower_bound() {
        // Le(Const(4), Var(0)) with delta=2: n ≥ 4 → n+2 ≥ 4
        let inv = Expr::Le(Box::new(Expr::Const(4)), Box::new(Expr::Var(0)));
        assert_eq!(LiaLayer::lia_step_decide(&inv, 2), Some(true));
    }

    #[test]
    fn lia_step_upper_bound_fails() {
        // Le(Var(0), Const(100)) with delta=1: n ≤ 100 → n+1 ≤ 100 is FALSE
        let inv = Expr::Le(Box::new(Expr::Var(0)), Box::new(Expr::Const(100)));
        assert_eq!(LiaLayer::lia_step_decide(&inv, 1), Some(false));
    }

    #[test]
    fn lia_step_modular() {
        // n%2=0 with delta=2: preserved (2%2=0)
        let inv = Expr::Eq(
            Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(2)))),
            Box::new(Expr::Const(0)),
        );
        assert_eq!(LiaLayer::lia_step_decide(&inv, 2), Some(true));
        // n%2=0 with delta=1: NOT preserved (1%2≠0)
        assert_eq!(LiaLayer::lia_step_decide(&inv, 1), Some(false));
    }

    #[test]
    fn lia_step_conjunction() {
        // And(n≥4, n%2=0) with delta=2: both preserved
        let inv = Expr::And(
            Box::new(Expr::Le(Box::new(Expr::Const(4)), Box::new(Expr::Var(0)))),
            Box::new(Expr::Eq(
                Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(2)))),
                Box::new(Expr::Const(0)),
            )),
        );
        assert_eq!(LiaLayer::lia_step_decide(&inv, 2), Some(true));
    }

    #[test]
    fn lia_step_ground() {
        assert_eq!(LiaLayer::lia_step_decide(&Expr::Const(1), 1), Some(true));
    }
}
