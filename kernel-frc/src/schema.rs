// Schema trait — each schema is a reduction strategy that tries to
// build an FRC for a given statement.
//
// A schema is a function that attempts to:
//   1. Build a candidate VM program C
//   2. Derive a candidate bound B*(params)
//   3. Construct ProofEq (S ⟺ run(C,B*)=1) and ProofTotal (halting)
//
// Each schema must be proof-producing, not just code-producing.

use kernel_types::Hash32;
use crate::frc_types::{Frc, Gap, SchemaId};

/// The result of a schema attempting to reduce a statement.
#[derive(Debug, Clone)]
pub enum SchemaResult {
    /// Schema successfully produced an FRC
    Success(Frc),
    /// Schema cannot reduce this statement — returns the gap (missing subgoal)
    Failure(Gap),
    /// Schema is not applicable to this statement type
    NotApplicable,
}

/// A reduction schema — tries to build an FRC for a statement.
pub trait Schema: Send + Sync {
    /// Schema identifier
    fn id(&self) -> SchemaId;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Cost of attempting this schema (for enumeration ordering)
    fn cost(&self) -> u64;

    /// Attempt to reduce the statement to a finite computation.
    ///
    /// Takes:
    ///   - statement_hash: H(S)
    ///   - statement: the statement as a structured description
    ///   - context: any supporting context
    ///
    /// Returns SchemaResult: Success(FRC), Failure(Gap), or NotApplicable.
    fn attempt_reduction(
        &self,
        statement_hash: Hash32,
        statement: &StatementDesc,
        context: &ReductionContext,
    ) -> SchemaResult;
}

/// Structured description of a statement for schema consumption.
#[derive(Debug, Clone)]
pub struct StatementDesc {
    /// Statement kind (universal, existential, equivalence, etc.)
    pub kind: StatementKind,
    /// The statement text
    pub text: String,
    /// Variables and their domains
    pub variables: Vec<VariableDesc>,
    /// The predicate/property to check
    pub predicate: String,
    /// Additional parameters
    pub params: Vec<(String, i64)>,
}

/// What kind of statement this is (determines which schemas apply).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StatementKind {
    /// ∀x ∈ D, P(x) — universal over finite domain
    UniversalFinite,
    /// ∃x ∈ D, P(x) — existential over finite domain
    ExistentialFinite,
    /// ∀x, P(x) — universal over potentially infinite domain
    UniversalInfinite,
    /// ∃x, P(x) — existential over potentially infinite domain
    ExistentialInfinite,
    /// P ↔ Q — equivalence
    Equivalence,
    /// Algebraic identity or equation
    Algebraic,
    /// Analytic inequality or bound
    Analytic,
    /// Boolean satisfiability
    BoolSat,
    /// Arithmetic: find x such that f(x) = target
    ArithFind,
}

/// A variable in the statement.
#[derive(Debug, Clone)]
pub struct VariableDesc {
    pub name: String,
    pub domain_lo: Option<i64>,
    pub domain_hi: Option<i64>,
    pub is_finite: bool,
}

/// Context for reduction — available lemmas, bounds, etc.
#[derive(Debug, Clone)]
pub struct ReductionContext {
    /// Previously proven lemmas (by goal hash)
    pub available_lemmas: Vec<Hash32>,
    /// Maximum VM steps allowed
    pub max_vm_steps: u64,
    /// Maximum memory slots
    pub max_memory_slots: usize,
}

impl ReductionContext {
    pub fn default_context() -> Self {
        Self {
            available_lemmas: Vec::new(),
            max_vm_steps: 1_000_000,
            max_memory_slots: 1024,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_kind_equality() {
        assert_eq!(StatementKind::BoolSat, StatementKind::BoolSat);
        assert_ne!(StatementKind::BoolSat, StatementKind::ArithFind);
    }

    #[test]
    fn reduction_context_default() {
        let ctx = ReductionContext::default_context();
        assert!(ctx.available_lemmas.is_empty());
        assert_eq!(ctx.max_vm_steps, 1_000_000);
    }
}
