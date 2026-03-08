import KernelVm.Run

/-!
# KernelVm Totality Proof

Proves: `step` and `run` are total functions.

In Lean4, totality is enforced by the type checker:
- `step` is defined by structural case analysis on the instruction, with no recursion.
  Every match arm returns a value. No `partial` annotation. No `sorry`.
- `runLoop` is structurally recursive on `fuel : Nat`, which is structurally decreasing.
  Lean's termination checker accepts this automatically.
- `run` composes `runLoop` with a post-processing step, both total.

Therefore, the type checker has already verified totality. The theorems below
make this explicit for documentation and for the proof bundle.

CRITICAL: No `sorry` in this file. Every theorem is fully proved.
-/

namespace KernelVm

/-- step always produces a result for any program and state.
    This is trivially true because step is a total function
    (no `partial`, no `sorry`, exhaustive pattern matching). -/
theorem step_total (p : Program) (s : VmState) :
    ∃ s' b, step p s = (s', b) :=
  ⟨(step p s).1, (step p s).2, rfl⟩

/-- runLoop always terminates for any fuel value.
    Structurally decreasing on Nat ensures termination. -/
theorem runLoop_total (p : Program) (s : VmState) (n : Nat) :
    ∃ s', runLoop p s n = s' :=
  ⟨runLoop p s n, rfl⟩

/-- run always produces a result for any program and budget.
    Composition of total functions is total. -/
theorem run_total (p : Program) (b : Nat) :
    ∃ outcome state, run p b = (outcome, state) :=
  ⟨(run p b).1, (run p b).2, rfl⟩

/-- The budget monotonically decreases: runLoop with fuel 0 returns immediately. -/
theorem runLoop_zero (p : Program) (s : VmState) :
    runLoop p s 0 = s := rfl

/-- runLoop with fuel (n+1) applies step then recurses with fuel n. -/
theorem runLoop_succ (p : Program) (s : VmState) (n : Nat) :
    runLoop p s (n + 1) =
      let (s', running) := step p s
      if running then runLoop p s' n else s' := rfl

end KernelVm
