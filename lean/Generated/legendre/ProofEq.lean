import KernelVm
import Generated.legendre.Program
import Generated.legendre.Bstar

/-!
  ProofEq for problem 'legendre': S ⟺ (run legendreProg legendreBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "E1AE62E25196D855"
  Program hash:   "F448E86613FAF991"
  B*: 2660000
-/

open KernelVm

/-- ProofEq (legendre, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem legendre_eq :
    (run legendreProg legendreBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: legendre lean emit (problem_id='legendre', n=50) reduced to VM program via Legendre: prime between n² and (n+1)² for all n ≤ N
-/
