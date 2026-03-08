import KernelVm
import Generated.erdos_straus.Program
import Generated.erdos_straus.Bstar

/-!
  ProofEq for problem 'erdos_straus': S ⟺ (run erdos_strausProg erdos_strausBstar = Halted 1)
  Schema: FiniteSearch
  Statement hash: "B19F0DCCA0E4F810"
  Program hash:   "EF85EF1DBF6B72FF"
  B*: 1810000
-/

open KernelVm

/-- ProofEq (erdos_straus, FiniteSearch):
    The VM program performs exhaustive search over the finite domain.
    Finding a witness causes halt with code 1.
    The reduction: ∃x ∈ [lo, hi]. P(x) ⟺ "program returns 1 within B* steps". -/
theorem erdos_straus_eq :
    (run erdos_strausProg erdos_strausBstar).1 = VmOutcome.halted 1 := by native_decide

/-! ## Reduction Chain
  Step 1: erdos_straus lean emit (problem_id='erdos_straus', n=30) reduced to VM program via Erdős–Straus: 4/n = 1/x + 1/y + 1/z for all n in [2, N]
-/
