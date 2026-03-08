/-!
# SEC Rule Syntax — Lean4 Formalization

The Self-Extending Calculus (SEC) adds new inference rules to the kernel.
Each rule is a schema with metavariables, premises, and a conclusion.
A rule is sound iff its soundness theorem type-checks in Lean4.

This file defines the rule syntax and the `Sound` predicate.
Generated `Sound_<hash>.lean` files prove specific rules sound.
-/

import KernelVm.InvSyn

namespace KernelVm.SEC

/-- Rule expression — patterns with metavariables for matching. -/
inductive RuleExpr where
  | metaVar (idx : Nat)
  | concrete (e : InvSyn.Expr)
  | stepPreserved (inner : RuleExpr) (delta : Int)
  | linkImplies (inv prop : RuleExpr)
  | andR (l r : RuleExpr)
  | orR (l r : RuleExpr)
  | leR (l r : RuleExpr)
  | addDelta (inner : RuleExpr) (delta : Int)
  deriving Repr, BEq

/-- Rule kind — what type of inference this rule encodes. -/
inductive RuleKind where
  | stepPreservation
  | monotonicity
  | inequalityLift
  | algebraicIdentity
  | composition
  | rewrite
  deriving Repr, BEq

/-- A synthesized inference rule schema. -/
structure RuleSyn where
  kind : RuleKind
  arity : Nat
  premises : List RuleExpr
  conclusion : RuleExpr
  deriving Repr, BEq

/-- An instantiation maps metavariable indices to InvSyn expressions. -/
def Instantiation := Nat → InvSyn.Expr

/-- Evaluate a RuleExpr under an instantiation to get an InvSyn.Expr.
    Returns `none` for meta-level constructs (stepPreserved, linkImplies). -/
def evalRuleExpr (inst : Instantiation) : RuleExpr → Option InvSyn.Expr
  | .metaVar i => some (inst i)
  | .concrete e => some e
  | .andR l r => do
    let lv ← evalRuleExpr inst l
    let rv ← evalRuleExpr inst r
    return InvSyn.Expr.andE lv rv
  | .orR l r => do
    let lv ← evalRuleExpr inst l
    let rv ← evalRuleExpr inst r
    return InvSyn.Expr.orE lv rv
  | .leR l r => do
    let lv ← evalRuleExpr inst l
    let rv ← evalRuleExpr inst r
    return InvSyn.Expr.le lv rv
  | .addDelta inner delta => do
    let iv ← evalRuleExpr inst inner
    return InvSyn.Expr.add iv (InvSyn.Expr.const delta)
  | .stepPreserved _ _ => none
  | .linkImplies _ _ => none

/-- Step preservation as a proposition:
    For all n, toProp inv n → toProp inv (n + delta). -/
def stepPreserved (inv : InvSyn.Expr) (delta : Nat) : Prop :=
  ∀ (n : Nat), InvSyn.toProp inv n → InvSyn.toProp inv (n + delta)

/-- Link implication as a proposition:
    For all n, toProp inv n → toProp prop n. -/
def linkImplies (inv prop : InvSyn.Expr) : Prop :=
  ∀ (n : Nat), InvSyn.toProp inv n → InvSyn.toProp prop n

/-- Soundness of a rule: for ALL instantiations, if all premises hold,
    then the conclusion holds.

    This is the key predicate. A rule R is sound iff `Sound R` has a proof.
    The SEC engine generates `Sound_<hash>.lean` files that prove this
    for specific rules. Lean4 is the ONLY soundness oracle. -/
def Sound (r : RuleSyn) : Prop :=
  ∀ (inst : Instantiation),
    (∀ (p : RuleExpr), p ∈ r.premises → ∃ e, evalRuleExpr inst p = some e ∧
      ∀ (n : Nat), InvSyn.toProp e n) →
    ∃ e, evalRuleExpr inst r.conclusion = some e ∧
      ∀ (n : Nat), InvSyn.toProp e n

end KernelVm.SEC
