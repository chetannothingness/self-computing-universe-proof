import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.Collatz.Statement

/-!
# Collatz Conjecture — IRC: FRONTIER

Candidate invariant: prefix accumulator.
Base: vacuously true.  PROVED.
Step: OPEN — requires showing n+1 eventually reaches 1 under 3n+1 map.
      InvSyn searched candidates up to AST size 10; no structural invariant found.
Link: trivial.  PROVED.

Status: Gap(Step) — the step obligation IS the Collatz Conjecture.
No axioms. No sorry. Gap documented honestly.
-/

namespace OpenProblems.Collatz

/-- Candidate invariant: all m ≤ n eventually reach 1. -/
def collatzInvariant (n : Nat) : Prop :=
  ∀ m, 1 ≤ m → m ≤ n → ∃ k, collatzIter k m = 1

/-- Base: vacuously true (no m satisfies 1 ≤ m ∧ m ≤ 0). -/
theorem collatz_base : collatzInvariant 0 := by
  intro m h1 h0; omega

-- FRONTIER(Step): The step obligation remains open.
-- InvSyn searched candidates up to AST size 10.
-- No invariant satisfies all three checkers.
-- The step obligation IS the Collatz Conjecture itself:
--   every positive integer eventually reaches 1 under the 3n+1 map.
-- When a structural invariant is found, the kernel will produce
-- the proof via dec_step_sound + native_decide.

/-- Link: trivial — invariant directly contains the property. -/
theorem collatz_link (n : Nat) (h : collatzInvariant n) :
    1 ≤ n → ∃ k, collatzIter k n = 1 := by
  intro h1; exact h n h1 (Nat.le.refl)

end OpenProblems.Collatz
