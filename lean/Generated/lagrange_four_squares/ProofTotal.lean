import KernelVm
import Generated.lagrange_four_squares.Program
import Generated.lagrange_four_squares.Bstar

/-!
  ProofTotal for problem 'lagrange_four_squares': run lagrange_four_squaresProg lagrange_four_squaresBstar terminates.
  B*: 2257120
  Halting argument: Program has 97 instructions. Lagrange: every n is sum of four squares B*=2257120 derived from parameter bounds and loop structure.
-/

open KernelVm

/-- ProofTotal (lagrange_four_squares):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem lagrange_four_squares_total :
    ∃ c, (run lagrange_four_squaresProg lagrange_four_squaresBstar).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem lagrange_four_squares_within_budget :
    (run lagrange_four_squaresProg lagrange_four_squaresBstar).1 ≠ VmOutcome.budgetExhausted := by
  native_decide
