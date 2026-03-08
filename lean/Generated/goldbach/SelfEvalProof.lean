import KernelVm.InvSyn
import Universe.SelfEval

namespace Generated.Goldbach.SelfEvalProof

open KernelVm.InvSyn
open Universe.SelfEval

/-- The invariant expression — same as the kernel's computation target. -/
def inv : Expr := Expr.implies (Expr.andE (Expr.le (Expr.const 4) (Expr.var 0)) (Expr.eq (Expr.modE (Expr.var 0) (Expr.const 2)) (Expr.const 0))) (Expr.existsBounded (Expr.const 2) (Expr.var 0) (Expr.andE (Expr.isPrime (Expr.var 0)) (Expr.isPrime (Expr.sub (Expr.var 1) (Expr.var 0)))))

/-- Bounded proof: ∀ n ≤ 100, toProp inv n.
    The eval IS the proof. native_decide IS the replay. One machine. -/
theorem bounded : ∀ n, n ≤ 100 → toProp inv n :=
  replayAll_sound inv 100 (by native_decide)

end Generated.Goldbach.SelfEvalProof
