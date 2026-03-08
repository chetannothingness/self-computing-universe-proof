import KernelVm
import Generated.collatz.Program
import Generated.collatz.Bstar

/-!
  ProofTotal for problem 'collatz': run collatzProg collatzBstar terminates.
  B*: 1318000
  Halting argument: Program has 53 instructions. Collatz verified for all n in [1, N] B*=1318000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (collatz):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem collatz_total :
    ∃ c, (run collatzProg collatzBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem collatz_within_budget :
    (run collatzProg collatzBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
