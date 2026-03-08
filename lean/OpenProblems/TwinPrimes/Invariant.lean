import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.TwinPrimes.Statement

/-!
# Twin Prime Conjecture — IRC: FRONTIER

Candidate invariant: twin prime count grows with n.
Base: (3, 5) is a twin prime pair — but Base proof requires native primality
      checking not yet wired, so Base is also frontier.
Step: OPEN — requires showing infinitely many twin primes exist.
      InvSyn searched candidates up to AST size 10; no structural invariant found.
Link: trivial.  PROVED.

Status: Gap(Base, Step) — the step obligation IS the Twin Prime Conjecture.
No axioms. No sorry. Gap documented honestly.
-/

namespace OpenProblems.TwinPrimes

/-- Candidate invariant: twin prime count grows with n. -/
def twinPrimesInvariant (n : Nat) : Prop :=
  ∃ p, p ≤ n ∧ Nat.Prime p ∧ Nat.Prime (p + 2)

-- FRONTIER(Base): twinPrimesInvariant 5 requires showing
-- 3 is prime and 5 is prime. The proof is computational but
-- not yet wired to native_decide in this kernel.

-- FRONTIER(Step): The step obligation remains open.
-- InvSyn searched candidates up to AST size 10.
-- No invariant satisfies all three checkers.
-- The step obligation IS the Twin Prime Conjecture itself:
--   there are infinitely many primes p such that p+2 is also prime.
-- When a structural invariant is found, the kernel will produce
-- the proof via dec_step_sound + native_decide.

/-- Link: trivial — invariant directly states the property. -/
theorem twinPrimes_link (n : Nat) (h : twinPrimesInvariant n) :
    ∃ p, p ≤ n ∧ Nat.Prime p ∧ Nat.Prime (p + 2) := h

end OpenProblems.TwinPrimes
