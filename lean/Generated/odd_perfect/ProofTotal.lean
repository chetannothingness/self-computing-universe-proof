import KernelVm
import Generated.odd_perfect.Program
import Generated.odd_perfect.Bstar

/-!
  ProofTotal for problem 'odd_perfect': run odd_perfectProg odd_perfectBstar terminates.
  B*: 1235000
  Halting argument: Program has 47 instructions. No odd perfect number in [1, N] B*=1235000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (odd_perfect):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem odd_perfect_total :
    ∃ c, (run odd_perfectProg odd_perfectBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem odd_perfect_within_budget :
    (run odd_perfectProg odd_perfectBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
