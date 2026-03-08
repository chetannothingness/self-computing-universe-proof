import KernelVm.Run

/-!
# KernelVm Determinism Proof

Proves: `step` and `run` are deterministic — same input → same output.

Since `step` and `run` are pure functions with no side effects,
determinism follows immediately from functional extensionality:
applying the same function to the same arguments always yields the same result.
In Lean4, this is trivially `rfl`.
-/

namespace KernelVm

/-- step is deterministic: same program + same state → same result. -/
theorem step_deterministic (p : Program) (s : VmState) :
    step p s = step p s := rfl

/-- runLoop is deterministic: same program + same state + same fuel → same result. -/
theorem runLoop_deterministic (p : Program) (s : VmState) (n : Nat) :
    runLoop p s n = runLoop p s n := rfl

/-- run is deterministic: same program + same b_star → same outcome and state. -/
theorem run_deterministic (p : Program) (b : Nat) :
    run p b = run p b := rfl

/-- Corollary: if two runs produce different outcomes, the inputs must differ. -/
theorem run_outcome_eq (p1 p2 : Program) (b1 b2 : Nat)
    (hp : p1 = p2) (hb : b1 = b2) :
    run p1 b1 = run p2 b2 := by
  subst hp; subst hb; rfl

end KernelVm
