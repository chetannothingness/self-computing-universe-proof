import KernelVm
import Generated.weak_goldbach.Program
import Generated.weak_goldbach.Bstar

/-!
  ProofTotal for problem 'weak_goldbach': run weak_goldbachProg weak_goldbachBstar terminates.
  B*: 1383328
  Halting argument: Program has 176 instructions. Weak Goldbach: every odd n > 5 is sum of three primes (Helfgott) B*=1383328 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (weak_goldbach):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem weak_goldbach_total :
    ∃ c, (run weak_goldbachProg weak_goldbachBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem weak_goldbach_within_budget :
    (run weak_goldbachProg weak_goldbachBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
