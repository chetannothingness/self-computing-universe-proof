import KernelVm
import Generated.mersenne.Program
import Generated.mersenne.Bstar

/-!
  ProofTotal for problem 'mersenne': run mersenneProg mersenneBstar terminates.
  B*: 5689680
  Halting argument: Program has 122 instructions. Mersenne prime exists for p in [2, P] B*=5689680 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (mersenne):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem mersenne_total :
    ∃ c, (run mersenneProg mersenneBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem mersenne_within_budget :
    (run mersenneProg mersenneBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
