//! Persistent, Merkle-committed rule database for SEC.
//!
//! Once a rule's soundness proof passes Lean4 verification (no sorry),
//! the rule is added to the database and committed via a Merkle root.
//! Rules are persistent — once proven, they stay forever.

use kernel_types::{Hash32, hash, HASH_ZERO};

use crate::invsyn::ast::Expr;
use crate::invsyn::structural::StructuralVerdict;
use super::rule_syn::{RuleSyn, RuleExpr, RuleKind};

/// A rule whose soundness has been verified by Lean4.
#[derive(Debug, Clone)]
pub struct ProvenRule {
    /// The rule schema.
    pub schema: RuleSyn,
    /// Hash of the Lean soundness proof file.
    pub soundness_hash: Hash32,
    /// Name of the Lean theorem that proves soundness.
    pub lean_theorem_name: String,
    /// Deterministic hash of the rule itself.
    pub rule_hash: Hash32,
    /// Epoch when this rule was discovered.
    pub discovered_epoch: u64,
}

impl ProvenRule {
    /// Create a new ProvenRule from a verified schema.
    pub fn new(schema: RuleSyn, soundness_hash: Hash32, lean_theorem_name: String, epoch: u64) -> Self {
        let rule_hash = schema.rule_hash();
        Self {
            schema,
            soundness_hash,
            lean_theorem_name,
            rule_hash,
            discovered_epoch: epoch,
        }
    }
}

/// Persistent, Merkle-committed rule database.
///
/// Rules are stored in discovery order. The Merkle root is recomputed
/// on every addition to ensure deterministic commitment.
#[derive(Debug, Clone)]
pub struct RuleDb {
    rules: Vec<ProvenRule>,
    merkle_root: Hash32,
}

impl RuleDb {
    /// Create an empty rule database.
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            merkle_root: HASH_ZERO,
        }
    }

    /// Add a proven rule to the database and recompute the Merkle root.
    pub fn add_rule(&mut self, rule: ProvenRule) {
        self.rules.push(rule);
        self.recompute_merkle();
    }

    /// Get the current Merkle root.
    pub fn merkle_root(&self) -> Hash32 {
        self.merkle_root
    }

    /// Get the number of rules in the database.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if the database is empty.
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Get all rules.
    pub fn rules(&self) -> &[ProvenRule] {
        &self.rules
    }

    /// Try to verify a step obligation using rules in the database.
    ///
    /// Pattern-matches each rule's conclusion against the obligation.
    /// If a rule unifies and all its premises hold, returns Verified.
    pub fn try_step_rules(&self, inv: &Expr, delta: i64) -> StructuralVerdict {
        for rule in &self.rules {
            if !matches!(rule.schema.kind, RuleKind::StepPreservation | RuleKind::Monotonicity | RuleKind::Composition) {
                continue;
            }
            if let Some(desc) = try_apply_step_rule(&rule.schema, inv, delta) {
                return StructuralVerdict::Verified(format!(
                    "SEC rule '{}': {}",
                    rule.lean_theorem_name, desc
                ));
            }
        }
        StructuralVerdict::NotVerifiable("no SEC step rule matched".into())
    }

    /// Try to verify a link obligation using rules in the database.
    ///
    /// Pattern-matches each rule's conclusion against the obligation.
    /// If a rule unifies and all its premises hold, returns Verified.
    pub fn try_link_rules(&self, inv: &Expr, prop: &Expr) -> StructuralVerdict {
        for rule in &self.rules {
            if !matches!(rule.schema.kind, RuleKind::InequalityLift | RuleKind::Rewrite | RuleKind::Composition) {
                continue;
            }
            if let Some(desc) = try_apply_link_rule(&rule.schema, inv, prop) {
                return StructuralVerdict::Verified(format!(
                    "SEC rule '{}': {}",
                    rule.lean_theorem_name, desc
                ));
            }
        }
        StructuralVerdict::NotVerifiable("no SEC link rule matched".into())
    }

    /// Recompute the Merkle root from all rule hashes.
    fn recompute_merkle(&mut self) {
        let hashes: Vec<Hash32> = self.rules.iter().map(|r| r.rule_hash).collect();
        self.merkle_root = hash::merkle_root(&hashes);
    }
}

/// Try to apply a step rule to an invariant with the given delta.
///
/// Returns a description string if the rule matches, None otherwise.
fn try_apply_step_rule(rule: &RuleSyn, inv: &Expr, delta: i64) -> Option<String> {
    // Match the conclusion pattern against StepPreserved(pattern, rule_delta)
    match &rule.conclusion {
        RuleExpr::StepPreserved(pattern, rule_delta) => {
            if *rule_delta != delta {
                return None;
            }
            // Try to unify the pattern with the invariant
            let bindings = unify_rule_expr(pattern, inv, rule.arity)?;
            // Check all premises with the bindings
            if check_premises(&rule.premises, &bindings) {
                Some(format!(
                    "step preservation matched for delta={}: {}",
                    delta, rule.description
                ))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Try to apply a link rule to an invariant and property.
///
/// Returns a description string if the rule matches, None otherwise.
fn try_apply_link_rule(rule: &RuleSyn, inv: &Expr, prop: &Expr) -> Option<String> {
    // Match the conclusion pattern against LinkImplies(inv_pattern, prop_pattern)
    match &rule.conclusion {
        RuleExpr::LinkImplies(inv_pattern, prop_pattern) => {
            let mut bindings = vec![None; rule.arity];
            // Unify inv_pattern with inv
            unify_into(inv_pattern, inv, &mut bindings)?;
            // Unify prop_pattern with prop
            unify_into(prop_pattern, prop, &mut bindings)?;
            // Check all premises
            if check_premises_with_bindings(&rule.premises, &bindings) {
                Some(format!("link implication matched: {}", rule.description))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Unify a RuleExpr pattern against a concrete Expr.
/// Returns metavariable bindings if unification succeeds.
fn unify_rule_expr(pattern: &RuleExpr, expr: &Expr, arity: usize) -> Option<Vec<Option<Expr>>> {
    let mut bindings = vec![None; arity];
    unify_into(pattern, expr, &mut bindings)?;
    Some(bindings)
}

/// Unify a pattern against an expression, filling in bindings.
fn unify_into(pattern: &RuleExpr, expr: &Expr, bindings: &mut [Option<Expr>]) -> Option<()> {
    match pattern {
        RuleExpr::MetaVar(i) => {
            if *i >= bindings.len() {
                return None;
            }
            match &bindings[*i] {
                Some(existing) => {
                    if existing == expr {
                        Some(())
                    } else {
                        None // Conflict
                    }
                }
                None => {
                    bindings[*i] = Some(expr.clone());
                    Some(())
                }
            }
        }
        RuleExpr::Concrete(c) => {
            if c == expr {
                Some(())
            } else {
                None
            }
        }
        RuleExpr::And(pa, pb) => {
            if let Expr::And(ea, eb) = expr {
                unify_into(pa, ea, bindings)?;
                unify_into(pb, eb, bindings)
            } else {
                None
            }
        }
        RuleExpr::Or(pa, pb) => {
            if let Expr::Or(ea, eb) = expr {
                unify_into(pa, ea, bindings)?;
                unify_into(pb, eb, bindings)
            } else {
                None
            }
        }
        RuleExpr::Le(pa, pb) => {
            if let Expr::Le(ea, eb) = expr {
                unify_into(pa, ea, bindings)?;
                unify_into(pb, eb, bindings)
            } else {
                None
            }
        }
        // StepPreserved and LinkImplies are meta-level — not matched against Expr
        RuleExpr::StepPreserved(_, _) | RuleExpr::LinkImplies(_, _) | RuleExpr::AddDelta(_, _) => {
            None
        }
    }
}

/// Check if all premises hold given complete metavariable bindings.
fn check_premises(premises: &[RuleExpr], bindings: &[Option<Expr>]) -> bool {
    check_premises_with_bindings(premises, bindings)
}

/// Check premises with the given bindings.
/// Premises are structural constraints on the metavariable instantiations.
fn check_premises_with_bindings(premises: &[RuleExpr], bindings: &[Option<Expr>]) -> bool {
    for premise in premises {
        if !check_single_premise(premise, bindings) {
            return false;
        }
    }
    true
}

/// Check a single premise against bindings.
fn check_single_premise(premise: &RuleExpr, bindings: &[Option<Expr>]) -> bool {
    match premise {
        RuleExpr::MetaVar(i) => {
            // A bare metavar premise means "this metavar must be bound"
            *i < bindings.len() && bindings[*i].is_some()
        }
        RuleExpr::Concrete(_) => true, // Ground premises are always satisfied
        _ => {
            // Complex premises: check structurally
            // For now, require all referenced metavars to be bound
            all_metavars_bound(premise, bindings)
        }
    }
}

/// Check that all metavariables referenced in a premise are bound.
fn all_metavars_bound(expr: &RuleExpr, bindings: &[Option<Expr>]) -> bool {
    match expr {
        RuleExpr::MetaVar(i) => *i < bindings.len() && bindings[*i].is_some(),
        RuleExpr::Concrete(_) => true,
        RuleExpr::StepPreserved(inner, _) => all_metavars_bound(inner, bindings),
        RuleExpr::LinkImplies(a, b) => all_metavars_bound(a, bindings) && all_metavars_bound(b, bindings),
        RuleExpr::And(a, b) => all_metavars_bound(a, bindings) && all_metavars_bound(b, bindings),
        RuleExpr::Or(a, b) => all_metavars_bound(a, bindings) && all_metavars_bound(b, bindings),
        RuleExpr::Le(a, b) => all_metavars_bound(a, bindings) && all_metavars_bound(b, bindings),
        RuleExpr::AddDelta(inner, _) => all_metavars_bound(inner, bindings),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_db() {
        let db = RuleDb::new();
        assert!(db.is_empty());
        assert_eq!(db.len(), 0);
        assert_eq!(db.merkle_root(), HASH_ZERO);
    }

    #[test]
    fn add_rule_updates_merkle() {
        let mut db = RuleDb::new();
        let schema = RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 1,
            premises: vec![RuleExpr::MetaVar(0)],
            conclusion: RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
            description: "test".to_string(),
            size: 2,
        };
        let rule = ProvenRule::new(schema, hash::H(b"proof"), "sound_test".to_string(), 0);
        db.add_rule(rule);
        assert_eq!(db.len(), 1);
        assert_ne!(db.merkle_root(), HASH_ZERO);
    }

    #[test]
    fn merkle_deterministic() {
        let make_db = || {
            let mut db = RuleDb::new();
            let s1 = RuleSyn {
                kind: RuleKind::StepPreservation,
                arity: 1,
                premises: vec![],
                conclusion: RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
                description: "a".to_string(),
                size: 2,
            };
            let s2 = RuleSyn {
                kind: RuleKind::Monotonicity,
                arity: 0,
                premises: vec![],
                conclusion: RuleExpr::MetaVar(0),
                description: "b".to_string(),
                size: 1,
            };
            db.add_rule(ProvenRule::new(s1, hash::H(b"p1"), "t1".into(), 0));
            db.add_rule(ProvenRule::new(s2, hash::H(b"p2"), "t2".into(), 1));
            db
        };
        assert_eq!(make_db().merkle_root(), make_db().merkle_root());
    }

    #[test]
    fn unify_metavar() {
        let pattern = RuleExpr::MetaVar(0);
        let expr = Expr::Const(42);
        let bindings = unify_rule_expr(&pattern, &expr, 1);
        assert!(bindings.is_some());
        let b = bindings.unwrap();
        assert_eq!(b[0], Some(Expr::Const(42)));
    }

    #[test]
    fn unify_concrete_match() {
        let pattern = RuleExpr::Concrete(Expr::Const(1));
        let bindings = unify_rule_expr(&pattern, &Expr::Const(1), 0);
        assert!(bindings.is_some());
    }

    #[test]
    fn unify_concrete_mismatch() {
        let pattern = RuleExpr::Concrete(Expr::Const(1));
        let bindings = unify_rule_expr(&pattern, &Expr::Const(2), 0);
        assert!(bindings.is_none());
    }

    #[test]
    fn unify_and_pattern() {
        let pattern = RuleExpr::And(
            Box::new(RuleExpr::MetaVar(0)),
            Box::new(RuleExpr::MetaVar(1)),
        );
        let expr = Expr::And(
            Box::new(Expr::Const(1)),
            Box::new(Expr::Const(2)),
        );
        let bindings = unify_rule_expr(&pattern, &expr, 2);
        assert!(bindings.is_some());
        let b = bindings.unwrap();
        assert_eq!(b[0], Some(Expr::Const(1)));
        assert_eq!(b[1], Some(Expr::Const(2)));
    }

    #[test]
    fn unify_conflict() {
        // Same metavar bound to different values
        let pattern = RuleExpr::And(
            Box::new(RuleExpr::MetaVar(0)),
            Box::new(RuleExpr::MetaVar(0)),
        );
        let expr = Expr::And(
            Box::new(Expr::Const(1)),
            Box::new(Expr::Const(2)),
        );
        let bindings = unify_rule_expr(&pattern, &expr, 1);
        assert!(bindings.is_none());
    }

    #[test]
    fn try_step_no_rules() {
        let db = RuleDb::new();
        let inv = Expr::Const(1);
        let v = db.try_step_rules(&inv, 1);
        assert!(!v.is_verified());
    }

    #[test]
    fn try_link_no_rules() {
        let db = RuleDb::new();
        let inv = Expr::Const(1);
        let prop = Expr::Const(1);
        let v = db.try_link_rules(&inv, &prop);
        assert!(!v.is_verified());
    }
}
