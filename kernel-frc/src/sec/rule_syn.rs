//! Rule syntax AST for the Self-Extending Calculus (SEC).
//!
//! A rule is a schema with metavariables, premises, and a conclusion.
//! When all premises are satisfied under a metavariable instantiation,
//! the conclusion holds. Rules are objects with proofs — never heuristics.

use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use serde::{Serialize, Deserialize};
use kernel_types::{Hash32, hash};

use crate::invsyn::ast::Expr;

/// The kind of inference rule being synthesized.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleKind {
    /// I(n) → I(n+δ) for a specific structural pattern
    StepPreservation,
    /// f monotone: x ≤ y ⇒ f(x) ≤ f(y)
    Monotonicity,
    /// X ≤ Y ∧ Y preserved ⇒ X preserved
    InequalityLift,
    /// Polynomial identity proof
    AlgebraicIdentity,
    /// rule A + rule B ⇒ rule C
    Composition,
    /// Truth-preserving rewrite
    Rewrite,
}

/// A rule expression — patterns with metavariables for matching.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RuleExpr {
    /// Pattern hole — matches any Expr during unification.
    MetaVar(usize),
    /// Specific InvSyn Expr (ground pattern).
    Concrete(Expr),
    /// Step preservation: I(n) → I(n+δ).
    StepPreserved(Box<RuleExpr>, i64),
    /// Link implication: I(n) → P(n).
    LinkImplies(Box<RuleExpr>, Box<RuleExpr>),
    /// Conjunction of two rule expressions.
    And(Box<RuleExpr>, Box<RuleExpr>),
    /// Disjunction of two rule expressions.
    Or(Box<RuleExpr>, Box<RuleExpr>),
    /// Comparison: lhs ≤ rhs.
    Le(Box<RuleExpr>, Box<RuleExpr>),
    /// Shift: expr + delta.
    AddDelta(Box<RuleExpr>, i64),
}

/// A synthesized inference rule schema.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleSyn {
    /// What kind of rule this is.
    pub kind: RuleKind,
    /// Number of metavariables (pattern holes).
    pub arity: usize,
    /// Premises that must hold for the rule to fire.
    pub premises: Vec<RuleExpr>,
    /// The conclusion that follows when all premises hold.
    pub conclusion: RuleExpr,
    /// Human-readable description of the rule.
    pub description: String,
    /// AST size metric for enumeration ordering.
    pub size: usize,
}

impl RuleSyn {
    /// Compute a deterministic hash of this rule.
    pub fn rule_hash(&self) -> Hash32 {
        let bytes = kernel_types::serpi::canonical_cbor_bytes(self);
        hash::H(&bytes)
    }

    /// Compute a deterministic u64 hash for ordering.
    pub fn order_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.kind.hash(&mut hasher);
        self.arity.hash(&mut hasher);
        self.size.hash(&mut hasher);
        for p in &self.premises {
            p.hash(&mut hasher);
        }
        self.conclusion.hash(&mut hasher);
        hasher.finish()
    }
}

impl RuleExpr {
    /// Compute the AST size of this rule expression.
    pub fn size(&self) -> usize {
        match self {
            RuleExpr::MetaVar(_) => 1,
            RuleExpr::Concrete(e) => e.size(),
            RuleExpr::StepPreserved(inner, _) => 1 + inner.size(),
            RuleExpr::LinkImplies(a, b) => 1 + a.size() + b.size(),
            RuleExpr::And(a, b) => 1 + a.size() + b.size(),
            RuleExpr::Or(a, b) => 1 + a.size() + b.size(),
            RuleExpr::Le(a, b) => 1 + a.size() + b.size(),
            RuleExpr::AddDelta(inner, _) => 1 + inner.size(),
        }
    }

    /// Convert to a Lean4 expression string for soundness proofs.
    pub fn to_lean(&self) -> String {
        match self {
            RuleExpr::MetaVar(i) => format!("inst {}", i),
            RuleExpr::Concrete(e) => e.to_lean(),
            RuleExpr::StepPreserved(inner, delta) => {
                format!("stepPreserved ({}) {}", inner.to_lean(), delta)
            }
            RuleExpr::LinkImplies(a, b) => {
                format!("linkImplies ({}) ({})", a.to_lean(), b.to_lean())
            }
            RuleExpr::And(a, b) => format!("andR ({}) ({})", a.to_lean(), b.to_lean()),
            RuleExpr::Or(a, b) => format!("orR ({}) ({})", a.to_lean(), b.to_lean()),
            RuleExpr::Le(a, b) => format!("leR ({}) ({})", a.to_lean(), b.to_lean()),
            RuleExpr::AddDelta(inner, delta) => {
                format!("addDelta ({}) {}", inner.to_lean(), delta)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_hash_deterministic() {
        let rule = RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 1,
            premises: vec![RuleExpr::MetaVar(0)],
            conclusion: RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
            description: "test rule".to_string(),
            size: 2,
        };
        let h1 = rule.rule_hash();
        let h2 = rule.rule_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn rule_hash_differs_by_kind() {
        let r1 = RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 1,
            premises: vec![],
            conclusion: RuleExpr::MetaVar(0),
            description: "a".to_string(),
            size: 1,
        };
        let r2 = RuleSyn {
            kind: RuleKind::Monotonicity,
            arity: 1,
            premises: vec![],
            conclusion: RuleExpr::MetaVar(0),
            description: "a".to_string(),
            size: 1,
        };
        assert_ne!(r1.rule_hash(), r2.rule_hash());
    }

    #[test]
    fn rule_expr_size() {
        assert_eq!(RuleExpr::MetaVar(0).size(), 1);
        let compound = RuleExpr::And(
            Box::new(RuleExpr::MetaVar(0)),
            Box::new(RuleExpr::MetaVar(1)),
        );
        assert_eq!(compound.size(), 3);
    }

    #[test]
    fn order_hash_deterministic() {
        let rule = RuleSyn {
            kind: RuleKind::AlgebraicIdentity,
            arity: 2,
            premises: vec![RuleExpr::MetaVar(0), RuleExpr::MetaVar(1)],
            conclusion: RuleExpr::And(
                Box::new(RuleExpr::MetaVar(0)),
                Box::new(RuleExpr::MetaVar(1)),
            ),
            description: "test".to_string(),
            size: 3,
        };
        assert_eq!(rule.order_hash(), rule.order_hash());
    }

    #[test]
    fn concrete_rule_expr() {
        let e = RuleExpr::Concrete(Expr::Const(42));
        assert_eq!(e.size(), 1);
        assert!(e.to_lean().contains("42"));
    }
}
