import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.Bertrand.Statement
import Mathlib.NumberTheory.Bertrand

/-!
# Bertrand's Postulate — IRC: PROVED (Chebyshev 1852)

Invariant: I(n) = ∀m ∈ [1,n], ∃ prime p, m < p ≤ 2m.
  Base: vacuously true.  PROVED.
  Step: via Mathlib `Nat.exists_prime_lt_and_le_two_mul`.  PROVED.
  Link: trivial.  PROVED.

Kernel status: PROVED (3/3 obligations discharged). UNBOUNDED.
-/

namespace OpenProblems.Bertrand

/-- Prefix invariant: Bertrand holds for all m up to n. -/
def bertrandInvariant (n : Nat) : Prop :=
  ∀ m, 1 ≤ m → m ≤ n → ∃ p, Nat.Prime p ∧ m < p ∧ p ≤ 2 * m

/-- Base: vacuously true (no m satisfies 1 ≤ m ∧ m ≤ 0). -/
theorem bertrand_base : bertrandInvariant 0 := by
  intro m h1 h0; omega

/-- Step: Bertrand's postulate from Mathlib. -/
theorem bertrand_step (n : Nat) (h : bertrandInvariant n) : bertrandInvariant (n + 1) := by
  intro m h1 hm
  by_cases heq : m ≤ n
  · exact h m h1 heq
  · have : m = n + 1 := by omega
    subst this
    exact Nat.exists_prime_lt_and_le_two_mul (n + 1) (by omega)

/-- Link: trivial — I(n) contains the property at m = n. -/
theorem bertrand_link (n : Nat) (h : bertrandInvariant n) :
    1 ≤ n → ∃ p, Nat.Prime p ∧ n < p ∧ p ≤ 2 * n := by
  intro h1; exact h n h1 (le_refl n)

/-- Full IRC. UNBOUNDED. -/
noncomputable def bertrandIRC : KernelVm.Invariant.IRC (fun n => 1 ≤ n → ∃ p, Nat.Prime p ∧ n < p ∧ p ≤ 2 * n) :=
  { I := bertrandInvariant
    base := bertrand_base
    step := bertrand_step
    link := bertrand_link }

/-- UNBOUNDED: ∀ n ≥ 1, ∃ prime p, n < p ≤ 2n. -/
theorem bertrand_full : ∀ n, 1 ≤ n → ∃ p, Nat.Prime p ∧ n < p ∧ p ≤ 2 * n :=
  KernelVm.Invariant.irc_implies_forall bertrandIRC

end OpenProblems.Bertrand
