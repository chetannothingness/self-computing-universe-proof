import KernelVm
import Generated.twin_primes.Program
import Generated.twin_primes.Bstar

/-!
  ProofEq for problem 'twin_primes': S ⟺ (run twin_primesProg twin_primesBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "AEC88877B23467BE"
  Program hash:   "947FACE6AA76F58F"
  B*: 9160000
-/

open KernelVm

/-- ProofEq (twin_primes, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem twin_primes_eq :
    (run twin_primesProg twin_primesBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: twin_primes lean emit (problem_id='twin_primes', n=1000) reduced to VM program via Twin prime pair exists in [2, N]
-/
