//! Universal statement type — mirrors lean/KernelVm/UCert/Universe.lean.
//!
//! Every problem compiles to a Statement. The `DecideProp` constructor
//! represents ∀n, f(n) = true for a total decidable function f,
//! identified by the problem's hash.

use serde::{Serialize, Deserialize};
use kernel_types::hash;

/// Statement identifier — each problem has a unique numeric ID.
pub type StatementId = u64;

/// Universal statement type (mirrors Lean).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Statement {
    /// ∀n from `start` by `delta`, P(n) holds.
    /// The predicate P is identified by `problem_id`.
    ForallFrom {
        problem_id: String,
        statement_id: StatementId,
        start: u64,
        delta: u64,
        description: String,
    },
    /// ∀n, f(n) = true where f is identified by StatementId.
    DecideProp {
        problem_id: String,
        statement_id: StatementId,
        description: String,
    },
    /// Conjunction.
    And(Box<Statement>, Box<Statement>),
    /// Disjunction.
    Or(Box<Statement>, Box<Statement>),
    /// Negation.
    Neg(Box<Statement>),
}

impl Statement {
    /// Create a ForallFrom statement.
    pub fn forall_from(problem_id: &str, start: u64, delta: u64, desc: &str) -> Self {
        let sid = hash_problem_id(problem_id);
        Statement::ForallFrom {
            problem_id: problem_id.to_string(),
            statement_id: sid,
            start,
            delta,
            description: desc.to_string(),
        }
    }

    /// Create a DecideProp statement.
    pub fn decide_prop(problem_id: &str, desc: &str) -> Self {
        let sid = hash_problem_id(problem_id);
        Statement::DecideProp {
            problem_id: problem_id.to_string(),
            statement_id: sid,
            description: desc.to_string(),
        }
    }

    /// Get the primary problem_id.
    pub fn problem_id(&self) -> &str {
        match self {
            Statement::ForallFrom { problem_id, .. } => problem_id,
            Statement::DecideProp { problem_id, .. } => problem_id,
            Statement::And(a, _) => a.problem_id(),
            Statement::Or(a, _) => a.problem_id(),
            Statement::Neg(s) => s.problem_id(),
        }
    }

    /// Get the statement ID.
    pub fn statement_id(&self) -> StatementId {
        match self {
            Statement::ForallFrom { statement_id, .. } => *statement_id,
            Statement::DecideProp { statement_id, .. } => *statement_id,
            Statement::And(a, _) => a.statement_id(),
            Statement::Or(a, _) => a.statement_id(),
            Statement::Neg(s) => s.statement_id(),
        }
    }

    /// Get the description.
    pub fn description(&self) -> &str {
        match self {
            Statement::ForallFrom { description, .. } => description,
            Statement::DecideProp { description, .. } => description,
            Statement::And(_, _) => "conjunction",
            Statement::Or(_, _) => "disjunction",
            Statement::Neg(_) => "negation",
        }
    }

    /// Deterministic hash of this statement.
    pub fn statement_hash(&self) -> [u8; 32] {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        hash::H(&bytes)
    }

    /// Convert to Lean representation.
    pub fn to_lean(&self) -> String {
        match self {
            Statement::ForallFrom { statement_id, start, delta, .. } => {
                format!("Statement.forallFrom {} {} {}", statement_id, start, delta)
            }
            Statement::DecideProp { statement_id, .. } => {
                format!("Statement.decideProp {}", statement_id)
            }
            Statement::And(a, b) => {
                format!("Statement.andS ({}) ({})", a.to_lean(), b.to_lean())
            }
            Statement::Or(a, b) => {
                format!("Statement.orS ({}) ({})", a.to_lean(), b.to_lean())
            }
            Statement::Neg(s) => {
                format!("Statement.negS ({})", s.to_lean())
            }
        }
    }
}

/// Compute a deterministic StatementId from a problem_id string.
fn hash_problem_id(problem_id: &str) -> StatementId {
    let h = hash::H(format!("ucert:statement:{}", problem_id).as_bytes());
    u64::from_le_bytes([h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statement_deterministic() {
        let s1 = Statement::forall_from("goldbach", 4, 2, "Goldbach");
        let s2 = Statement::forall_from("goldbach", 4, 2, "Goldbach");
        assert_eq!(s1.statement_hash(), s2.statement_hash());
    }

    #[test]
    fn statement_id_unique() {
        let s1 = Statement::forall_from("goldbach", 4, 2, "G");
        let s2 = Statement::forall_from("collatz", 1, 1, "C");
        assert_ne!(s1.statement_id(), s2.statement_id());
    }

    #[test]
    fn statement_to_lean() {
        let s = Statement::forall_from("goldbach", 4, 2, "G");
        let lean = s.to_lean();
        assert!(lean.starts_with("Statement.forallFrom"));
    }

    #[test]
    fn problem_id_extraction() {
        let s = Statement::forall_from("collatz", 1, 1, "C");
        assert_eq!(s.problem_id(), "collatz");
    }
}
