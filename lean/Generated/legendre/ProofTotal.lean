import KernelVm
import Generated.legendre.Program
import Generated.legendre.Bstar

/-!
  ProofTotal for problem 'legendre': run legendreProg legendreBstar terminates.
  B*: 2660000
  Halting argument: Program has 83 instructions. Legendre: prime between n² and (n+1)² for all n ≤ N B*=2660000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (legendre):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem legendre_total :
    ∃ c, (run legendreProg legendreBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem legendre_within_budget :
    (run legendreProg legendreBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
