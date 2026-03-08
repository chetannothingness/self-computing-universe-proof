import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.ErdosStraus.Statement

/-!
# Erdos-Straus Conjecture — IRC: FRONTIER

Candidate invariant: prefix accumulator.
Base: vacuously true.  PROVED.
Step: OPEN — requires showing 4/(n+1) = 1/x + 1/y + 1/z for some x,y,z.
      InvSyn searched candidates up to AST size 10; no structural invariant found.
Link: trivial.  PROVED.

Status: Gap(Step) — the step obligation IS the Erdos-Straus Conjecture.
No axioms. No sorry. Gap documented honestly.
-/

namespace OpenProblems.ErdosStraus

/-- Candidate invariant: Erdos-Straus holds for all m in [2, n]. -/
def erdosStrausInvariant (n : Nat) : Prop :=
  ∀ m, 2 ≤ m → m ≤ n → ∃ x y z : Nat, x > 0 ∧ y > 0 ∧ z > 0 ∧
    4 * x * y * z = m * (y * z + x * z + x * y)

/-- Base: vacuously true (no m satisfies 2 ≤ m ∧ m ≤ 0). -/
theorem erdosStraus_base : erdosStrausInvariant 0 := by
  intro m h2 h0; omega

-- FRONTIER(Step): The step obligation remains open.
-- InvSyn searched candidates up to AST size 10.
-- No invariant satisfies all three checkers.
-- The step obligation IS the Erdos-Straus Conjecture itself:
--   for every integer n >= 2, 4/n = 1/x + 1/y + 1/z for some positive x,y,z.
-- When a structural invariant is found, the kernel will produce
-- the proof via dec_step_sound + native_decide.

/-- Link: trivial — invariant contains the property at m = n. -/
theorem erdosStraus_link (n : Nat) (h : erdosStrausInvariant n) :
    2 ≤ n → ∃ x y z : Nat, x > 0 ∧ y > 0 ∧ z > 0 ∧
    4 * x * y * z = n * (y * z + x * z + x * y) := by
  intro h2; exact h n h2 (Nat.le.refl)

end OpenProblems.ErdosStraus
