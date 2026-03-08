import KernelVm.Step

/-!
# KernelVm Run Function

Total run function — mirrors `Vm::run` (vm.rs lines 494-510).
Runs `step` for up to `b_star` iterations. If not halted after budget,
sets outcome to BudgetExhausted.

The function uses `Nat.fold` (structurally decreasing on the fuel parameter),
so Lean's termination checker accepts it without `partial` or `sorry`.
-/

namespace KernelVm

/-- Run `step` repeatedly for up to `fuel` steps. Stops early if step returns false.
    Structurally decreasing on `fuel`. -/
def runLoop (program : Program) (state : VmState) (fuel : Nat) : VmState :=
  match fuel with
  | 0 => state
  | n + 1 =>
    let (newState, running) := step program state
    if running then
      runLoop program newState n
    else
      newState

/-- Total run function: Program × Nat → (VmOutcome, VmState).
    Always terminates within b_star steps. No divergence possible.
    Mirrors `Vm::run` (vm.rs lines 494-510). -/
def run (program : Program) (b_star : Nat) : VmOutcome × VmState :=
  let finalState := runLoop program VmState.initial b_star
  let finalState :=
    if !finalState.halted then
      { finalState with halted := true, outcome := some VmOutcome.budgetExhausted }
    else
      finalState
  match finalState.outcome with
  | some outcome => (outcome, finalState)
  | none => (VmOutcome.budgetExhausted, finalState)  -- unreachable: halted implies outcome set

end KernelVm
