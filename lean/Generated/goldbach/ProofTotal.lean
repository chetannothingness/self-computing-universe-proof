import KernelVm
import Generated.goldbach.Program
import Generated.goldbach.Bstar

/-!
  ProofTotal for problem 'goldbach': run goldbachProg goldbachBstar terminates.
  B*: 3360000
  Halting argument: Program has 118 instructions. Goldbach verified for all even n in [4, N] B*=3360000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (goldbach):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem goldbach_total :
    ∃ c, (run goldbachProg goldbachBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem goldbach_within_budget :
    (run goldbachProg goldbachBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
