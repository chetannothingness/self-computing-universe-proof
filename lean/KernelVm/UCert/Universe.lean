/-!
# Universal Certificate Calculus — Universe

The universal statement language. Every computable predicate over Nat
is expressible as a Statement in this calculus.

From FOUNDATION.md: S admissible ⟺ ∃ FRC(S). The UCert calculus
extends this to unbounded proofs: S provable ⟺ ∃ cert, Check(S, cert) = true.

Key property: every computable predicate over Nat is expressible as
a `decideProp` identified by its problem hash. Domain primitives
(IsPrime, CollatzReaches1, etc.) compile to this form.
No ad-hoc constructors needed — everything is a program.
-/

namespace KernelVm.UCert

/-- Objects in the universe — the ONLY admissible syntax for values. -/
inductive Obj where
  | nat : Nat → Obj
  | pair : Obj → Obj → Obj
  | code : List Nat → Obj          -- Goedel-coded finite programs
  deriving Repr, BEq, Hashable

/-- Statement identifier — each problem has a unique numeric ID.
    The hash is computed deterministically from the problem description. -/
abbrev StatementId := UInt64

/-- A statement in the universal certificate calculus.
    Every domain-specific predicate compiles to a Statement via Goedel coding.
    PrimeCount(n), ZetaZero(k), PDEFlow(t), CircuitSize(n) are ALL macros
    over `decideProp` — no separate "domain packages" needed. -/
inductive Statement where
  /-- ∀n from `start` by `delta`, P(n) holds.
      The predicate P is identified by the StatementId.
      Computation of P(n) for each n is delegated to the runtime. -/
  | forallFrom : StatementId → Nat → Nat → Statement
  /-- ∀n, f(n) = true where f is identified by StatementId.
      This is the universal form — every decidable predicate compiles here. -/
  | decideProp : StatementId → Statement
  /-- Conjunction of two statements. -/
  | andS : Statement → Statement → Statement
  /-- Disjunction of two statements. -/
  | orS : Statement → Statement → Statement
  /-- Negation of a statement. -/
  | negS : Statement → Statement
  deriving Repr, BEq, Hashable

/-- Extract the primary StatementId from a Statement. -/
def Statement.primaryId : Statement → Option StatementId
  | Statement.forallFrom id _ _ => some id
  | Statement.decideProp id => some id
  | Statement.andS s _ => s.primaryId
  | Statement.orS s _ => s.primaryId
  | Statement.negS s => s.primaryId

/-- Check if two statements have the same structure. -/
def Statement.structEq : Statement → Statement → Bool
  | Statement.forallFrom id1 s1 d1, Statement.forallFrom id2 s2 d2 =>
    id1 == id2 && s1 == s2 && d1 == d2
  | Statement.decideProp id1, Statement.decideProp id2 => id1 == id2
  | Statement.andS a1 b1, Statement.andS a2 b2 =>
    a1.structEq a2 && b1.structEq b2
  | Statement.orS a1 b1, Statement.orS a2 b2 =>
    a1.structEq a2 && b1.structEq b2
  | Statement.negS s1, Statement.negS s2 => s1.structEq s2
  | _, _ => false

end KernelVm.UCert
