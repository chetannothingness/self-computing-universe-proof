import KernelVm
import Generated.mersenne.Program
import Generated.mersenne.Bstar

/-!
  ProofEq for problem 'mersenne': S ⟺ (run mersenneProg mersenneBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "FC2812B1CDA7036C"
  Program hash:   "D7C5AF32882F9702"
  B*: 5689680
-/

open KernelVm

/-- ProofEq (mersenne, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem mersenne_eq :
    (run mersenneProg mersenneBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: mersenne lean emit (problem_id='mersenne', n=31) reduced to VM program via Mersenne prime exists for p in [2, P]
-/
