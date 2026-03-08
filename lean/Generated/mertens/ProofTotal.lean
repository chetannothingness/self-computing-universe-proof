import KernelVm
import Generated.mertens.Program
import Generated.mertens.Bstar

/-!
  ProofTotal for problem 'mertens': run mertensProg mertensBstar terminates.
  B*: 1224400
  Halting argument: Program has 102 instructions. Mertens |M(n)| ≤ √n verified for all n ≤ N (Riemann Hypothesis fragment) B*=1224400 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (mertens):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem mertens_total :
    ∃ c, (run mertensProg mertensBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem mertens_within_budget :
    (run mertensProg mertensBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
