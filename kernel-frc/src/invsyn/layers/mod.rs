//! Layer trait and implementations for InvSyn checkers.
//!
//! Each layer provides decidable checkers for Base, Step, and Link obligations
//! at a specific level of mathematical complexity.

pub mod lia;
pub mod polynomial;
pub mod algebraic;

use super::ast::Expr;
use super::normalize::ReachabilityProblem;

/// Result of a layer check.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Whether the check passed.
    pub passed: bool,
    /// The layer that performed the check.
    pub layer_name: String,
    /// Description of the check for audit trail.
    pub description: String,
}

/// Layer trait — each layer provides Base/Step/Link checkers for its domain.
pub trait Layer: Send + Sync {
    /// Layer name for reporting.
    fn name(&self) -> &str;

    /// Check that the invariant holds at the initial state(s).
    fn check_base(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult;

    /// Check that the invariant is preserved by the step relation.
    fn check_step(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult;

    /// Check that the invariant implies the target property.
    fn check_link(&self, inv: &Expr, problem: &ReachabilityProblem) -> CheckResult;
}
