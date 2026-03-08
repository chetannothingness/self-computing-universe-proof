import KernelVm
import Generated.lagrange_four_squares.Program
import Generated.lagrange_four_squares.Bstar

/-!
  ProofEq for problem 'lagrange_four_squares': S ⟺ (run lagrange_four_squaresProg lagrange_four_squaresBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "7142365307ABBDCF"
  Program hash:   "56E3A59DA9F8FE76"
  B*: 2257120
-/

open KernelVm

/-- ProofEq (lagrange_four_squares, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem lagrange_four_squares_eq :
    (run lagrange_four_squaresProg lagrange_four_squaresBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: lagrange_four_squares lean emit (problem_id='lagrange_four_squares', n=30) reduced to VM program via Lagrange: every n is sum of four squares
-/
