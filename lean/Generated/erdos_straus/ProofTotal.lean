import KernelVm
import Generated.erdos_straus.Program
import Generated.erdos_straus.Bstar

/-!
  ProofTotal for problem 'erdos_straus': run erdos_strausProg erdos_strausBstar terminates.
  B*: 1810000
  Halting argument: Program has 90 instructions. Erdős–Straus: 4/n = 1/x + 1/y + 1/z for all n in [2, N] B*=1810000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (erdos_straus):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem erdos_straus_total :
    ∃ c, (run erdos_strausProg erdos_strausBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem erdos_straus_within_budget :
    (run erdos_strausProg erdos_strausBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
