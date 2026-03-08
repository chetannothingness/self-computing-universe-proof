import KernelVm
import Generated.bsd_ec_count.Program
import Generated.bsd_ec_count.Bstar

/-!
  ProofTotal for problem 'bsd_ec_count': run bsd_ec_countProg bsd_ec_countBstar terminates.
  B*: 1008400
  Halting argument: Program has 84 instructions. BSD: elliptic curve point count over F_p with Hasse bound B*=1008400 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (bsd_ec_count):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem bsd_ec_count_total :
    ∃ c, (run bsd_ec_countProg bsd_ec_countBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem bsd_ec_count_within_budget :
    (run bsd_ec_countProg bsd_ec_countBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
