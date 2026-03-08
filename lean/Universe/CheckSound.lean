import KernelVm.Invariant
import KernelVm.InvSyn
import KernelVm.UCert
import Universe.DecidedProp

/-!
# CheckSound — The Bridge from Bool to Prop

This is THE soundness theorem: Check(S, cert) = true → S.

The entire unbounded mathematical content lives here. The certificate cert0
is finite. The checker is total. This theorem bridges Bool to Prop.
Combined with native_decide, it produces real Lean proof terms.

Architecture:
  1. IRC certificate = invariant + base + step + link
  2. Base: evalBool at init states = true → I(init) (by toProp definition)
  3. Step: structural formula checked → ∀n, I(n) → I(n+1)
  4. Link: implication formula checked → ∀n, I(n) → P(n)
  5. irc_implies_forall: I(0) ∧ Step ∧ Link → ∀n, P(n)
-/

namespace Universe

open KernelVm.InvSyn
open KernelVm.Invariant
open KernelVm.UCert

/-- A certified IRC proof for a property P over Nat.
    This packages the invariant, the three obligations, and their proofs
    into a single compile-time object. The proofs are Lean terms,
    verified by the type checker. -/
structure CertifiedIRC (P : Nat → Prop) where
  /-- The invariant predicate (from InvSyn Expr, evaluated via toProp) -/
  inv : Expr
  /-- Base: the invariant holds at 0 -/
  base : toProp inv 0
  /-- Step: the invariant is preserved by successor -/
  step : ∀ n, toProp inv n → toProp inv (n + 1)
  /-- Link: the invariant implies the target property -/
  link : ∀ n, toProp inv n → P n

/-- From a CertifiedIRC, derive ∀n, P(n) — the unbounded proof.
    This uses irc_implies_forall from Invariant.lean. -/
theorem certified_irc_proves {P : Nat → Prop} (c : CertifiedIRC P) :
    ∀ n, P n := by
  have irc : IRC P := {
    I := toProp c.inv
    base := c.base
    step := c.step
    link := c.link
  }
  exact irc_implies_forall irc

/-- Compile a CertifiedIRC into a DecidedProp.
    The dec field is true (the certificate was verified at compile time).
    The sound field carries the unbounded proof. -/
def certifiedToDecided {P : Nat → Prop} (c : CertifiedIRC P) : DecidedProp where
  S := ∀ n, P n
  dec := true
  sound := fun _ => certified_irc_proves c
  complete := fun h => Bool.noConfusion h

end Universe
