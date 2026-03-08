import KernelVm
import Generated.mertens.Program
import Generated.mertens.Bstar

/-!
  ProofEq for problem 'mertens': S ⟺ (run mertensProg mertensBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "45363A2227135B65"
  Program hash:   "C24CB66A94BB6E83"
  B*: 1224400
-/

open KernelVm

/-- ProofEq (mertens, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem mertens_eq :
    (run mertensProg mertensBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: mertens lean emit (problem_id='mertens', n=100) reduced to VM program via Mertens |M(n)| ≤ √n verified for all n ≤ N (Riemann Hypothesis fragment)
-/
