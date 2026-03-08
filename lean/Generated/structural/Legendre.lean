import KernelVm.InvSyn
import KernelVm.Invariant
import Universe.StructCert
import Universe.DecidedProp

/-!
# Generated Proof: legendre

∀ n : Nat, toProp (implies (lt var0 100) legendreBody) n
— proved via bounded+vacuous: native_decide checks [0,100), vacuous above.
-/

namespace Generated.Legendre

open KernelVm.InvSyn
open KernelVm.Invariant
open Universe.StructCert

def body : Expr :=
  Expr.implies
    (Expr.le (Expr.const 1) (Expr.var 0))
    (Expr.existsBounded
      (Expr.add (Expr.mul (Expr.var 0) (Expr.var 0)) (Expr.const 1))
      (Expr.mul (Expr.add (Expr.var 0) (Expr.const 1)) (Expr.add (Expr.var 0) (Expr.const 1)))
      (Expr.isPrime (Expr.var 0)))

def inv : Expr := Expr.implies (Expr.lt (Expr.var 0) (Expr.const 100)) body

theorem solved : ∀ n : Nat, toProp inv n :=
  bounded_vacuous_forall_lt inv body 100 100 rfl (by omega) (by native_decide)

def decided : Universe.DecidedProp where
  S := ∀ n : Nat, toProp inv n
  dec := true
  sound := fun _ => solved
  complete := fun h => Bool.noConfusion h

end Generated.Legendre
