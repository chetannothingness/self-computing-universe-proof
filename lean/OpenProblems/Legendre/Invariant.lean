import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.Legendre.Statement

/-!
# Legendre's Conjecture — IRC: FRONTIER

Candidate invariant: prefix accumulator.
Base: vacuously true (no m satisfies 1 ≤ m ∧ m ≤ 0).  PROVED.
Step: OPEN — requires showing a prime exists between (n+1)² and (n+2)².
      InvSyn searched candidates up to AST size 10; no structural invariant found.
Link: trivial.  PROVED.

Status: Gap(Step) — the step obligation IS Legendre's Conjecture.
No axioms. No sorry. Gap documented honestly.
-/

namespace OpenProblems.Legendre

/-- Candidate invariant: Legendre property holds for all m ≤ n. -/
def legendreInvariant (n : Nat) : Prop :=
  ∀ m, 1 ≤ m → m ≤ n → ∃ p, Nat.Prime p ∧ m * m < p ∧ p ≤ (m + 1) * (m + 1)

/-- Base: vacuously true (no m satisfies 1 ≤ m ∧ m ≤ 0). -/
theorem legendre_base : legendreInvariant 0 := by
  intro m h1 h0; omega

-- FRONTIER(Step): The step obligation remains open.
-- InvSyn searched candidates up to AST size 10.
-- No invariant satisfies all three checkers.
-- The step obligation IS Legendre's Conjecture itself:
--   for every n ≥ 1, there exists a prime between n² and (n+1)².
-- When a structural invariant is found, the kernel will produce
-- the proof via dec_step_sound + native_decide.

/-- Link: trivial — invariant contains the property at m = n. -/
theorem legendre_link (n : Nat) (h : legendreInvariant n) :
    1 ≤ n → ∃ p, Nat.Prime p ∧ n * n < p ∧ p ≤ (n + 1) * (n + 1) := by
  intro h1; exact h n h1 (le_refl n)

end OpenProblems.Legendre
