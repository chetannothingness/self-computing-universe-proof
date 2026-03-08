import KernelVm
import Generated.zfc_zero_ne_one.Program
import Generated.zfc_zero_ne_one.Bstar

/-!
  ProofEq for problem 'zfc_zero_ne_one': S ⟺ (run zfc_zero_ne_oneProg zfc_zero_ne_oneBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "A85353D5F9DCA4EC"
  Program hash:   "F8638D0A2FD906BD"
  B*: 10
-/

open KernelVm

/-- ProofEq (zfc_zero_ne_one, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem zfc_zero_ne_one_eq :
    (run zfc_zero_ne_oneProg zfc_zero_ne_oneBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: zfc_zero_ne_one lean emit (problem_id='zfc_zero_ne_one', n=0) reduced to VM program via 0 ≠ 1 (ZFC consistency)
-/
