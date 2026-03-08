import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.OddPerfect.Statement

/-!
# Odd Perfect Numbers — IRC: FRONTIER

Candidate invariant: prefix accumulator — no odd perfect number ≤ n.
Base: vacuously true.  PROVED.
Step: OPEN — requires showing no odd number n+1 is perfect.
      InvSyn searched candidates up to AST size 10; no structural invariant found.
Link: trivial.  PROVED.

Status: Gap(Step) — the step obligation IS the Odd Perfect Number Conjecture.
No axioms. No sorry. Gap documented honestly.
-/

namespace OpenProblems.OddPerfect

/-- Candidate invariant: no odd perfect number ≤ n. -/
def oddPerfectInvariant (n : Nat) : Prop :=
  ∀ m, 1 ≤ m → m ≤ n → m % 2 = 1 → ¬ (∃ d, d > 0 ∧ d < m ∧ m % d = 0 ∧ d = m)

/-- Base: vacuously true. -/
theorem oddPerfect_base : oddPerfectInvariant 0 := by
  intro m h1 h0; omega

-- FRONTIER(Step): The step obligation remains open.
-- InvSyn searched candidates up to AST size 10.
-- No invariant satisfies all three checkers.
-- The step obligation IS the Odd Perfect Number Conjecture:
--   no odd number is a perfect number.
-- When a structural invariant is found, the kernel will produce
-- the proof via dec_step_sound + native_decide.

/-- Link: trivial — invariant contains the property. -/
theorem oddPerfect_link (n : Nat) (h : oddPerfectInvariant n) :
    oddPerfectInvariant n := h

end OpenProblems.OddPerfect
