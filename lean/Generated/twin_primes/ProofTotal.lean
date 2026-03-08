import KernelVm
import Generated.twin_primes.Program
import Generated.twin_primes.Bstar

/-!
  ProofTotal for problem 'twin_primes': run twin_primesProg twin_primesBstar terminates.
  B*: 9160000
  Halting argument: Program has 102 instructions. Twin prime pair exists in [2, N] B*=9160000 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (twin_primes):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem twin_primes_total :
    ∃ c, (run twin_primesProg twin_primesBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem twin_primes_within_budget :
    (run twin_primesProg twin_primesBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
