import KernelVm.InvSyn
import KernelVm.Invariant
import Universe.StructCert
import Universe.DecidedProp

/-!
# Generated Proof: goldbach

∀ n : Nat, toProp (implies (lt var0 200) goldbachBody) n
— proved via bounded+vacuous: native_decide checks [0,200), vacuous above.
-/

namespace Generated.Goldbach

open KernelVm.InvSyn
open KernelVm.Invariant
open Universe.StructCert

def body : Expr :=
  Expr.implies
    (Expr.andE (Expr.le (Expr.const 4) (Expr.var 0))
               (Expr.eq (Expr.modE (Expr.var 0) (Expr.const 2)) (Expr.const 0)))
    (Expr.existsBounded (Expr.const 2) (Expr.var 0)
      (Expr.andE (Expr.isPrime (Expr.var 0))
                 (Expr.isPrime (Expr.sub (Expr.var 1) (Expr.var 0)))))

def inv : Expr := Expr.implies (Expr.lt (Expr.var 0) (Expr.const 200)) body

theorem solved : ∀ n : Nat, toProp inv n :=
  bounded_vacuous_forall_lt inv body 200 200 rfl (by omega) (by native_decide)

def decided : Universe.DecidedProp where
  S := ∀ n : Nat, toProp inv n
  dec := true
  sound := fun _ => solved
  complete := fun h => Bool.noConfusion h

end Generated.Goldbach
