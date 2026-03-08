import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.Goldbach.Statement

/-!
# Goldbach's Conjecture — IRC: FRONTIER

Candidate invariant: prefix accumulator.
Base: vacuously true.  PROVED.
Step: OPEN — requires showing each new even number is sum of two primes.
      InvSyn searched candidates up to AST size 10; no structural invariant found.
Link: trivial.  PROVED.

Status: Gap(Step) — the step obligation IS Goldbach's Conjecture.
No axioms. No sorry. Gap documented honestly.
-/

namespace OpenProblems.Goldbach

/-- Candidate invariant: prefix accumulator. -/
def goldbachInvariant (n : Nat) : Prop :=
  ∀ m, 4 ≤ m → m ≤ n → m % 2 = 0 → isSumOfTwoPrimes m

/-- Base: vacuously true (no even m in [4, 0]). -/
theorem goldbach_base : goldbachInvariant 0 := by
  intro m h4 h0; omega

-- FRONTIER(Step): The step obligation remains open.
-- InvSyn searched candidates up to AST size 10.
-- No invariant satisfies all three checkers.
-- The step obligation IS Goldbach's Conjecture itself:
--   every even integer ≥ 4 is the sum of two primes.
-- When a structural invariant is found, the kernel will produce
-- the proof via dec_step_sound + native_decide.

/-- Link: trivial — invariant directly contains the property. -/
theorem goldbach_link (n : Nat) (h : goldbachInvariant n) :
    4 ≤ n → n % 2 = 0 → isSumOfTwoPrimes n := by
  intro h4 heven; exact h n h4 (le_refl n) heven

end OpenProblems.Goldbach
