import KernelVm
import Generated.flt.Program
import Generated.flt.Bstar

/-!
  ProofEq for problem 'flt': S ⟺ (run fltProg fltBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "E218AE88B2B0CB58"
  Program hash:   "E0CCB461D638DE7A"
  B*: 1000000
-/

open KernelVm

/-- ProofEq (flt, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem flt_eq :
    (run fltProg fltBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: flt lean emit (problem_id='flt', n=2, aux=5) reduced to VM program via FLT verified for small cases
-/
