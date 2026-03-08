import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.WeakGoldbach.Statement

/-!
# Weak Goldbach Conjecture — IRC: PROVED (KnownProof: Helfgott 2013)

Invariant: I(n) = ∀ odd m ∈ [7,n], m is sum of three primes.
  Base: vacuously true (no odd m in [7,0]).  PROVED.
  Step: Helfgott's circle method bound (2013).
        FRONTIER(Step) — Helfgott's proof is NOT yet in Mathlib.
        The theorem is mathematically proved (published, peer-reviewed),
        but no Lean formalization exists.
  Link: trivial — I(n) contains the property at m = n.  PROVED.

Kernel status: PROVED by IRC (KnownProof), FRONTIER for Lean verification.
The step proof awaits Mathlib formalization of Helfgott's theorem.
-/

namespace OpenProblems.WeakGoldbach

/-- Prefix invariant: weak Goldbach holds for all odd m in [7, n]. -/
def weakGoldbachInvariant (n : Nat) : Prop :=
  ∀ m, 7 ≤ m → m ≤ n → m % 2 = 1 → isSumOfThreePrimes m

/-- Base: vacuously true (no m satisfies 7 ≤ m ∧ m ≤ 0). -/
theorem weakGoldbach_base : weakGoldbachInvariant 0 := by
  intro m h7 h0; omega

-- FRONTIER(Step): Helfgott's theorem (2013) proves this mathematically,
-- but it is NOT in Mathlib yet. We do NOT use sorry — the step is
-- honestly marked as FRONTIER pending Mathlib formalization.
-- When Mathlib adds `helfgott_weak_goldbach`, the step becomes:
--   theorem weakGoldbach_step (n : Nat) (h : weakGoldbachInvariant n) :
--       weakGoldbachInvariant (n + 1) := by
--     intro m h7 hm hodd
--     by_cases heq : m ≤ n
--     · exact h m h7 heq hodd
--     · have : m = n + 1 := by omega
--       subst this; exact helfgott_weak_goldbach (n + 1) (by omega) hodd

/-- Link: trivial — I(n) contains the property at m = n. -/
theorem weakGoldbach_link (n : Nat) (h : weakGoldbachInvariant n) :
    7 ≤ n → n % 2 = 1 → isSumOfThreePrimes n := by
  intro h7 hodd; exact h n h7 (le_refl n) hodd

end OpenProblems.WeakGoldbach
