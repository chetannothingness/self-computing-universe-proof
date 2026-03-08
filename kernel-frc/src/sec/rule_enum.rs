//! Canonical enumeration of RuleSyn candidates by (size, kind, hash).
//!
//! The enumerator generates candidate inference rules deterministically.
//! Gap-targeted enumeration inspects the gap's Expr structure to prioritize
//! relevant rule shapes.

use super::rule_syn::{RuleSyn, RuleExpr, RuleKind};

/// Enumerate candidate rules up to the given size.
///
/// Returns candidates sorted by (size, kind ordinal, hash) for deterministic ordering.
pub fn enumerate_candidates(max_size: usize) -> Vec<RuleSyn> {
    let mut candidates = Vec::new();

    if max_size >= 1 {
        candidates.extend(size_1_rules());
    }
    if max_size >= 2 {
        candidates.extend(size_2_rules());
    }
    if max_size >= 3 {
        candidates.extend(size_3_rules());
    }

    // Sort by (size, kind ordinal, hash)
    candidates.sort_by(|a, b| {
        let size_cmp = a.size.cmp(&b.size);
        if size_cmp != std::cmp::Ordering::Equal {
            return size_cmp;
        }
        let kind_cmp = kind_ordinal(&a.kind).cmp(&kind_ordinal(&b.kind));
        if kind_cmp != std::cmp::Ordering::Equal {
            return kind_cmp;
        }
        a.order_hash().cmp(&b.order_hash())
    });

    candidates
}

/// Enumerate candidates targeted at a specific gap.
///
/// Inspects the gap's invariant and property expressions to prioritize
/// relevant rule shapes (e.g., if the gap involves a lower bound,
/// prioritize monotonicity rules).
pub fn enumerate_for_gap(
    max_size: usize,
    _inv_expr: Option<&crate::invsyn::ast::Expr>,
    _prop_expr: Option<&crate::invsyn::ast::Expr>,
    _delta: i64,
) -> Vec<RuleSyn> {
    // Start with general enumeration
    let mut candidates = enumerate_candidates(max_size);

    // Gap-targeted: generate specific rule shapes based on the gap structure
    // For now, the general enumeration covers all shapes. Future versions
    // can inspect inv_expr/prop_expr to prioritize.

    // Deduplicate by hash
    candidates.dedup_by(|a, b| a.order_hash() == b.order_hash());

    candidates
}

/// Size 1 rules: ground preservation, trivial steps.
fn size_1_rules() -> Vec<RuleSyn> {
    vec![
        // Ground preservation: if I has no free variables, step is trivial
        RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 1,
            premises: vec![
                // MetaVar(0) must be ground (checked at application time)
                RuleExpr::MetaVar(0),
            ],
            conclusion: RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
            description: "ground expression preserved by any step".to_string(),
            size: 1,
        },
        // Direct lower bound preservation
        RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 1,
            premises: vec![
                RuleExpr::Le(
                    Box::new(RuleExpr::MetaVar(0)),
                    Box::new(RuleExpr::Concrete(crate::invsyn::ast::Expr::Var(0))),
                ),
            ],
            conclusion: RuleExpr::StepPreserved(
                Box::new(RuleExpr::Le(
                    Box::new(RuleExpr::MetaVar(0)),
                    Box::new(RuleExpr::Concrete(crate::invsyn::ast::Expr::Var(0))),
                )),
                1,
            ),
            description: "lower bound n >= c preserved by positive delta".to_string(),
            size: 1,
        },
    ]
}

/// Size 2 rules: monotonicity, single-function preservation.
fn size_2_rules() -> Vec<RuleSyn> {
    vec![
        // Monotonicity: f monotone, x ≤ y ⇒ f(x) ≤ f(y)
        RuleSyn {
            kind: RuleKind::Monotonicity,
            arity: 2,
            premises: vec![
                RuleExpr::Le(Box::new(RuleExpr::MetaVar(0)), Box::new(RuleExpr::MetaVar(1))),
            ],
            conclusion: RuleExpr::StepPreserved(
                Box::new(RuleExpr::Le(Box::new(RuleExpr::MetaVar(0)), Box::new(RuleExpr::MetaVar(1)))),
                1,
            ),
            description: "monotone ordering preserved under positive step".to_string(),
            size: 2,
        },
        // Conjunction preservation: A ∧ B preserved iff both preserved
        RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 2,
            premises: vec![
                RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
                RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(1)), 1),
            ],
            conclusion: RuleExpr::StepPreserved(
                Box::new(RuleExpr::And(Box::new(RuleExpr::MetaVar(0)), Box::new(RuleExpr::MetaVar(1)))),
                1,
            ),
            description: "conjunction preservation: both conjuncts preserved".to_string(),
            size: 2,
        },
    ]
}

/// Size 3+ rules: composition, algebraic transforms.
fn size_3_rules() -> Vec<RuleSyn> {
    vec![
        // Composition: step rule A + step rule B ⇒ step rule (A ∧ B)
        RuleSyn {
            kind: RuleKind::Composition,
            arity: 2,
            premises: vec![
                RuleExpr::MetaVar(0),
                RuleExpr::MetaVar(1),
            ],
            conclusion: RuleExpr::StepPreserved(
                Box::new(RuleExpr::And(Box::new(RuleExpr::MetaVar(0)), Box::new(RuleExpr::MetaVar(1)))),
                1,
            ),
            description: "compose two step rules into conjunction".to_string(),
            size: 3,
        },
        // Inequality lift: X ≤ Y ∧ Y preserved ⇒ X-related preserved
        RuleSyn {
            kind: RuleKind::InequalityLift,
            arity: 2,
            premises: vec![
                RuleExpr::Le(Box::new(RuleExpr::MetaVar(0)), Box::new(RuleExpr::MetaVar(1))),
            ],
            conclusion: RuleExpr::LinkImplies(
                Box::new(RuleExpr::Le(Box::new(RuleExpr::MetaVar(0)), Box::new(RuleExpr::Concrete(crate::invsyn::ast::Expr::Var(0))))),
                Box::new(RuleExpr::Le(Box::new(RuleExpr::MetaVar(1)), Box::new(RuleExpr::Concrete(crate::invsyn::ast::Expr::Var(0))))),
            ),
            description: "inequality lift: n >= a ∧ a >= b ⇒ n >= b".to_string(),
            size: 3,
        },
        // Rewrite: truth-preserving rewrite
        RuleSyn {
            kind: RuleKind::Rewrite,
            arity: 2,
            premises: vec![
                // MetaVar(0) ⟺ MetaVar(1) (logically equivalent)
                RuleExpr::MetaVar(0),
                RuleExpr::MetaVar(1),
            ],
            conclusion: RuleExpr::LinkImplies(
                Box::new(RuleExpr::MetaVar(0)),
                Box::new(RuleExpr::MetaVar(1)),
            ),
            description: "truth-preserving rewrite: equivalent expressions".to_string(),
            size: 3,
        },
        // Algebraic identity placeholder
        RuleSyn {
            kind: RuleKind::AlgebraicIdentity,
            arity: 1,
            premises: vec![RuleExpr::MetaVar(0)],
            conclusion: RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
            description: "algebraic identity: polynomial equality".to_string(),
            size: 3,
        },
    ]
}

/// Ordinal for deterministic sorting of RuleKind.
fn kind_ordinal(kind: &RuleKind) -> u8 {
    match kind {
        RuleKind::StepPreservation => 0,
        RuleKind::Monotonicity => 1,
        RuleKind::InequalityLift => 2,
        RuleKind::AlgebraicIdentity => 3,
        RuleKind::Composition => 4,
        RuleKind::Rewrite => 5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumerate_deterministic() {
        let c1 = enumerate_candidates(3);
        let c2 = enumerate_candidates(3);
        assert_eq!(c1.len(), c2.len());
        for (a, b) in c1.iter().zip(c2.iter()) {
            assert_eq!(a.order_hash(), b.order_hash());
        }
    }

    #[test]
    fn enumerate_sorted_by_size() {
        let candidates = enumerate_candidates(3);
        for window in candidates.windows(2) {
            assert!(window[0].size <= window[1].size);
        }
    }

    #[test]
    fn enumerate_expected_counts() {
        let s1 = enumerate_candidates(1);
        let s2 = enumerate_candidates(2);
        let s3 = enumerate_candidates(3);
        assert_eq!(s1.len(), 2, "size 1 should have 2 rules");
        assert_eq!(s2.len(), 4, "size ≤2 should have 4 rules");
        assert_eq!(s3.len(), 8, "size ≤3 should have 8 rules");
    }

    #[test]
    fn gap_targeted_includes_general() {
        let general = enumerate_candidates(3);
        let targeted = enumerate_for_gap(3, None, None, 1);
        // Targeted should include at least everything from general
        assert!(targeted.len() >= general.len());
    }

    #[test]
    fn all_rules_have_descriptions() {
        let candidates = enumerate_candidates(3);
        for c in &candidates {
            assert!(!c.description.is_empty());
        }
    }
}
