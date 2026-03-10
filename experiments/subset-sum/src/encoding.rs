//! Build a kernel Expr tree for Subset Sum from arbitrary weights.
//!
//! For weights = [w0, w1, ..., w_{k-1}] and outer variable T (target),
//! produces k nested ExistsBounded(0, 1, ...) where each bound variable
//! b_i ∈ {0, 1} represents "include item i or not."
//!
//! The innermost body checks: b0*w0 + b1*w1 + ... + b_{k-1}*w_{k-1} = T.
//!
//! Variable binding (inside the innermost body):
//!   Var(0)   = b_{k-1}  (innermost quantifier's bound variable)
//!   Var(1)   = b_{k-2}
//!   ...
//!   Var(k-1) = b_0      (outermost quantifier's bound variable)
//!   Var(k)   = T         (from the environment via mk_env)

use kernel_frc::invsyn::Expr;

/// Build an Expr that checks: "∃ subset of `weights` summing to T"
/// where T is provided as env[0] when evaluating (accessed as Var(k)).
pub fn build_subset_sum_expr(weights: &[i64]) -> Expr {
    let k = weights.len();
    // Build the innermost equality check first, then wrap with ExistsBounded.
    let body = build_sum_eq_target(weights);
    wrap_with_exists(body, k)
}

/// Build: b0*w0 + b1*w1 + ... + b_{k-1}*w_{k-1} = Var(k)
///
/// Inside k nested ExistsBounded:
///   Var(i) = b_{k-1-i}  (bound variable from the (k-i)-th quantifier)
///   Var(k) = T           (outer environment)
///
/// So the weight w_j pairs with Var(k-1-j).
fn build_sum_eq_target(weights: &[i64]) -> Expr {
    let k = weights.len();
    // Build the sum: Mul(Var(k-1-0), w0) + Mul(Var(k-1-1), w1) + ...
    let mut terms: Vec<Expr> = Vec::with_capacity(k);
    for j in 0..k {
        let var_idx = k - 1 - j;
        terms.push(Expr::Mul(
            Box::new(Expr::Var(var_idx)),
            Box::new(Expr::Const(weights[j])),
        ));
    }
    // Chain additions: terms[0] + terms[1] + ... + terms[k-1]
    let sum = terms
        .into_iter()
        .reduce(|acc, t| Expr::Add(Box::new(acc), Box::new(t)))
        .unwrap_or(Expr::Const(0));

    // Eq(sum, Var(k))  where Var(k) = target T from environment
    Expr::Eq(Box::new(sum), Box::new(Expr::Var(k)))
}

/// Wrap `body` with k nested ExistsBounded(0, 1, ...).
/// Outermost quantifier binds b_0, innermost binds b_{k-1}.
fn wrap_with_exists(body: Expr, k: usize) -> Expr {
    let mut expr = body;
    for _ in 0..k {
        expr = Expr::ExistsBounded(
            Box::new(Expr::Const(0)),
            Box::new(Expr::Const(1)),
            Box::new(expr),
        );
    }
    expr
}

/// Pretty-print the subset sum instance.
pub fn describe_instance(weights: &[i64]) -> String {
    let max_sum: i64 = weights.iter().sum();
    format!(
        "Subset Sum: {} items, weights = {:?}, max_sum = {}, search space = 2^{} = {}",
        weights.len(),
        weights,
        max_sum,
        weights.len(),
        1u64 << weights.len(),
    )
}

/// Compute max achievable sum.
pub fn max_sum(weights: &[i64]) -> i64 {
    weights.iter().sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_frc::invsyn::eval::{eval, mk_env};

    #[test]
    fn trivial_single_weight() {
        // weights = [5], target = 5 → should be reachable (b=1)
        let expr = build_subset_sum_expr(&[5]);
        let result = eval(&mk_env(5), &expr);
        assert_eq!(result, 1, "target 5 should be reachable with weight [5]");

        // target = 3 → not reachable
        let result = eval(&mk_env(3), &expr);
        assert_eq!(result, 0, "target 3 should NOT be reachable with weight [5]");

        // target = 0 → reachable (empty subset)
        let result = eval(&mk_env(0), &expr);
        assert_eq!(result, 1, "target 0 should be reachable (empty subset)");
    }

    #[test]
    fn two_weights() {
        let expr = build_subset_sum_expr(&[3, 7]);
        let check = |t: i64| eval(&mk_env(t), &expr);

        assert_eq!(check(0), 1);   // empty
        assert_eq!(check(3), 1);   // {3}
        assert_eq!(check(7), 1);   // {7}
        assert_eq!(check(10), 1);  // {3, 7}
        assert_eq!(check(5), 0);   // no subset sums to 5
        assert_eq!(check(4), 0);
    }

    #[test]
    fn five_weights() {
        let weights = vec![2, 3, 5, 7, 11];
        let expr = build_subset_sum_expr(&weights);
        let check = |t: i64| eval(&mk_env(t), &expr);

        // 0 always reachable
        assert_eq!(check(0), 1);
        // individual weights
        assert_eq!(check(2), 1);
        assert_eq!(check(11), 1);
        // full sum
        assert_eq!(check(28), 1);
        // 1 is not reachable
        assert_eq!(check(1), 0);
        // 4 is not reachable (no subset of {2,3,5,7,11} sums to 4)
        assert_eq!(check(4), 0);
    }
}
