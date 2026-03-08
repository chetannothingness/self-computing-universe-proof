import KernelVm.UCert.Universe
import KernelVm.InvSyn

/-!
# Universal Certificate Calculus — Certificate Types

Certificate language: finite, checkable objects that witness proof obligations.
Every certificate is a data structure — no functions, no oracles.
The checker verifies certificates in bounded time.

v2: StepCert.structural and LinkCert.structural carry real Expr ASTs,
not strings. The checker calls the real structural_step_check and
structural_link_check on these Expr values.
-/

namespace KernelVm.UCert

open KernelVm.InvSyn (Expr)

/-- Base case certificate — proves I(init) holds. -/
inductive BaseCert where
  /-- Evaluate I(n) for n = init..init+bound, all must hold. -/
  | directCheck : Nat → BaseCert
  /-- I(init) is trivially true (e.g., vacuous quantifier). -/
  | trivial : BaseCert
  deriving Repr, BEq, Hashable

-- ─── Typed certificate structures for advanced step proofs ───

/-- A single step in an interval arithmetic proof. -/
inductive IntervalStep where
  | eval : (point : Int) → (valueLo : Int) → (valueHi : Int) → IntervalStep
  | subdivide : (mid : Int) → IntervalStep
  | monotoneOn : (lo : Int) → (hi : Int) → IntervalStep
  deriving Repr, BEq, Hashable

/-- Interval arithmetic certificate — proves value stays in [lo, hi]. -/
structure IntervalCert where
  lo : Expr
  hi : Expr
  proofSteps : List IntervalStep
  deriving Repr, BEq, Hashable

/-- Sieve-theoretic bound certificate. -/
structure SieveCert where
  sieveLevel : Nat
  remainderBound : Expr
  mainTerm : Expr
  deriving Repr, BEq, Hashable

/-- Certified sum bound. -/
structure SumCert where
  sumExpr : Expr
  bound : Expr
  errorBound : Expr
  deriving Repr, BEq, Hashable

/-- Algebraic identity certificate (Gröbner basis / SOS decomposition). -/
structure AlgebraicCert where
  identity : Expr
  witnesses : List Expr
  deriving Repr, BEq, Hashable

/-- Justification for a monotone step. -/
inductive MonoJustification where
  | algebraic : AlgebraicCert → MonoJustification
  | monotonicity : MonoJustification
  | cauchySchwarz : MonoJustification
  | amGm : MonoJustification
  deriving Repr, BEq, Hashable

/-- A single step in a monotone inequality chain. -/
structure MonoStep where
  src : Expr
  dst : Expr
  justification : MonoJustification
  deriving Repr, BEq, Hashable

/-- Step certificate — proves ∀n, I(n) → I(n+δ).
    The critical component: this is where mathematical content lives.
    v2: structural carries real Expr (not String). -/
inductive StepCert where
  /-- Reference to a known theorem (Chebyshev, Lagrange, Wiles, etc.).
      The theorem name is verified against a registry of accepted proofs. -/
  | knownProof : String → StepCert
  /-- Structural verification via InvSyn engine.
      v2: carries the REAL invariant Expr (for structural_step_check). -/
  | structural : Expr → StepCert
  /-- Evaluate step up to bound (for base cases only, never proves ∀). -/
  | directEval : Nat → StepCert
  /-- Interval enclosure proof. -/
  | intervalBound : IntervalCert → StepCert
  /-- Sieve-theoretic bound certificate. -/
  | sieveBound : SieveCert → StepCert
  /-- Finite sum bound certificate. -/
  | sumBound : SumCert → StepCert
  /-- Monotone inequality chain. -/
  | monotoneChain : List MonoStep → StepCert
  /-- Algebraic identity certificate. -/
  | algebraicId : AlgebraicCert → StepCert
  /-- Compose two step proofs. -/
  | composition : StepCert → StepCert → StepCert
  deriving Repr, BEq, Hashable

/-- Link certificate — proves ∀n, I(n) → P(n).
    v2: structural carries (invariant, property) Exprs. -/
inductive LinkCert where
  /-- Link is trivially true (invariant directly contains property). -/
  | trivial : LinkCert
  /-- Direct logical implication. -/
  | directImplication : LinkCert
  /-- Structural verification via InvSyn engine.
      v2: carries (invariant, property) real Exprs. -/
  | structural : Expr → Expr → LinkCert
  deriving Repr, BEq, Hashable

/-- Invariant certificate — the IRC bridge.
    Packages an invariant with certificates for Base, Step, and Link.
    v2: includes the real invariant Expr. -/
structure InvCert where
  /-- The actual invariant expression (real Expr AST). -/
  invariant : Expr
  /-- Description of the invariant predicate I(n). -/
  invariantDesc : String
  /-- Invariant hash (deterministic, for deduplication). -/
  invariantHash : UInt64
  /-- Certificate that I(init) holds. -/
  baseCert : BaseCert
  /-- Certificate that ∀n, I(n) → I(n+δ). -/
  stepCert : StepCert
  /-- Certificate that ∀n, I(n) → P(n). -/
  linkCert : LinkCert
  deriving Repr, BEq, Hashable

/-- Universal certificate type — the kernel's proof witness.
    Every certificate is finite, enumerable, and checkable. -/
inductive Cert where
  /-- IRC certificate: invariant + base + step + link. -/
  | invariantCert : InvCert → Cert
  /-- Existential witness: provides a concrete n satisfying ∃. -/
  | witnessCert : Nat → Cert
  /-- Composite: chain of sub-certificates for compound statements. -/
  | compositeCert : List Cert → Cert
  /-- Proof trace: sequence of rewrite steps as certificate. -/
  | proofTrace : List String → Cert
  deriving Repr, BEq, Hashable

/-- Certificate size — total node count for enumeration ordering. -/
partial def Cert.size : Cert → Nat
  | Cert.invariantCert _ => 1
  | Cert.witnessCert _ => 1
  | Cert.compositeCert cs => 1 + cs.foldl (fun acc c => acc + c.size) 0
  | Cert.proofTrace steps => 1 + steps.length

/-- StepCert size for enumeration. -/
def StepCert.size : StepCert → Nat
  | StepCert.knownProof _ => 1
  | StepCert.structural _ => 1
  | StepCert.directEval _ => 1
  | StepCert.intervalBound _ => 1
  | StepCert.sieveBound _ => 1
  | StepCert.sumBound _ => 1
  | StepCert.monotoneChain steps => 1 + steps.length
  | StepCert.algebraicId _ => 1
  | StepCert.composition a b => 1 + a.size + b.size

end KernelVm.UCert
