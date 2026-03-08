import KernelVm
import Generated.weak_goldbach.Program
import Generated.weak_goldbach.Bstar

/-!
  ProofEq for problem 'weak_goldbach': S ⟺ (run weak_goldbachProg weak_goldbachBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "529D6D506B9D329F"
  Program hash:   "D9891618A2DC0F4E"
  B*: 1383328
-/

open KernelVm

/-- ProofEq (weak_goldbach, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem weak_goldbach_eq :
    (run weak_goldbachProg weak_goldbachBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: weak_goldbach lean emit (problem_id='weak_goldbach', n=30) reduced to VM program via Weak Goldbach: every odd n > 5 is sum of three primes (Helfgott)
-/
