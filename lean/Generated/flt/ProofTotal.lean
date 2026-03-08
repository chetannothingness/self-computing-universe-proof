import KernelVm
import Generated.flt.Program
import Generated.flt.Bstar

/-!
  ProofTotal for problem 'flt': run fltProg fltBstar terminates.
  B*: 1000000
  Halting argument: Program has 117 instructions. FLT verified for small cases B*=1000000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (flt):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem flt_total :
    ∃ c, (run fltProg fltBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem flt_within_budget :
    (run fltProg fltBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
