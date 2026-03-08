import KernelVm
import Generated.odd_perfect.Program
import Generated.odd_perfect.Bstar

/-!
  ProofEq for problem 'odd_perfect': S ⟺ (run odd_perfectProg odd_perfectBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "AE3A9AE7FF473797"
  Program hash:   "88170A3A925C6BB4"
  B*: 1235000
-/

open KernelVm

/-- ProofEq (odd_perfect, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem odd_perfect_eq :
    (run odd_perfectProg odd_perfectBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: odd_perfect lean emit (problem_id='odd_perfect', n=100) reduced to VM program via No odd perfect number in [1, N]
-/
