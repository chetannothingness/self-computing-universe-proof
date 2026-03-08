import KernelVm
import Generated.zfc_zero_ne_one.Program
import Generated.zfc_zero_ne_one.Bstar

/-!
  ProofTotal for problem 'zfc_zero_ne_one': run zfc_zero_ne_oneProg zfc_zero_ne_oneBstar terminates.
  B*: 10
  Halting argument: Program has 6 instructions. 0 ≠ 1 (ZFC consistency) B*=10 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (zfc_zero_ne_one):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem zfc_zero_ne_one_total :
    ∃ c, (run zfc_zero_ne_oneProg zfc_zero_ne_oneBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem zfc_zero_ne_one_within_budget :
    (run zfc_zero_ne_oneProg zfc_zero_ne_oneBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
