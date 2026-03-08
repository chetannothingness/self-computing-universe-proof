/-!
# Invariant Reduction Certificate Framework

An IRC proves ∀n, P(n) via:
1. Base: I(0)
2. Step: ∀n, I(n) → I(n+1)
3. Link: ∀n, I(n) → P(n)

Then by Nat.rec: ∀n, I(n), hence ∀n, P(n).

This is Route B of the unbounded proof strategy. FRC handles bounded fragments;
IRC handles the full unbounded statement via inductive invariants.
-/

namespace KernelVm.Invariant

/-- An IRC packages an invariant with its three obligations. -/
structure IRC (P : Nat → Prop) where
  /-- The invariant predicate -/
  I : Nat → Prop
  /-- Base case: I holds at 0 -/
  base : I 0
  /-- Inductive step: I is preserved -/
  step : ∀ n, I n → I (n + 1)
  /-- Link: I implies the target property -/
  link : ∀ n, I n → P n

/-- Given an IRC, derive ∀n, I(n) by induction. -/
theorem irc_invariant_holds {P : Nat → Prop} (irc : IRC P) :
    ∀ n, irc.I n := by
  intro n
  induction n with
  | zero => exact irc.base
  | succ k ih => exact irc.step k ih

/-- Given an IRC, derive ∀n, P(n). This is the main theorem. -/
theorem irc_implies_forall {P : Nat → Prop} (irc : IRC P) :
    ∀ n, P n := by
  intro n
  exact irc.link n (irc_invariant_holds irc n)

/-- IRC for reachability — more general than Nat-indexed version.
    Handles state spaces beyond Nat with arbitrary step relations. -/
structure ReachIRC (X : Type) (I0 : X → Prop) (step : X → X → Prop) (P : X → Prop) where
  I    : X → Prop
  base : ∀ x, I0 x → I x
  step : ∀ x x', I x → step x x' → I x'
  link : ∀ x, I x → P x

/-- For Nat-indexed problems with successor step, convert IRC to ReachIRC. -/
def natReachIRC (P : Nat → Prop) (irc : IRC P) :
    ReachIRC Nat (· = 0) (fun n m => m = n + 1) P where
  I := irc.I
  base := by intro x hx; rw [hx]; exact irc.base
  step := by intro x x' hI hstep; rw [hstep]; exact irc.step x hI
  link := irc.link

end KernelVm.Invariant
