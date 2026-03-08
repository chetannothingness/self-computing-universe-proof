import KernelVm
import Generated.bertrand.Program
import Generated.bertrand.Bstar

/-!
  ProofTotal for problem 'bertrand': run bertrandProg bertrandBstar terminates.
  B*: 4000000
  Halting argument: Program has 75 instructions. Bertrand: prime between n and 2n for all n ≤ N (Chebyshev) B*=4000000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (bertrand):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem bertrand_total :
    ∃ c, (run bertrandProg bertrandBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem bertrand_within_budget :
    (run bertrandProg bertrandBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
