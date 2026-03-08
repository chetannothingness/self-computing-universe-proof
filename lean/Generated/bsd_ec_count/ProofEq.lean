import KernelVm
import Generated.bsd_ec_count.Program
import Generated.bsd_ec_count.Bstar

/-!
  ProofEq for problem 'bsd_ec_count': S ⟺ (run bsd_ec_countProg bsd_ec_countBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "4E17D5CA87991A58"
  Program hash:   "926734E6BC7E47F7"
  B*: 1008400
-/

open KernelVm

/-- ProofEq (bsd_ec_count, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem bsd_ec_count_eq :
    (run bsd_ec_countProg bsd_ec_countBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: bsd_ec_count lean emit (problem_id='bsd_ec_count', n=10, aux=0) reduced to VM program via BSD: elliptic curve point count over F_p with Hasse bound
-/
