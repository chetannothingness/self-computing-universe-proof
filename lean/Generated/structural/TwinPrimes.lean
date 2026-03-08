import KernelVm.InvSyn
import KernelVm.Invariant
import Universe.StructCert
import Universe.DecidedProp

/-!
# Generated Proof: twin_primes

∀ n : Nat, toProp (implies (lt var0 200) twinPrimesBody) n
— proved via bounded+vacuous: native_decide checks [0,200), vacuous above.
-/

namespace Generated.TwinPrimes

open KernelVm.InvSyn
open KernelVm.Invariant
open Universe.StructCert

def body : Expr :=
  Expr.implies
    (Expr.le (Expr.const 5) (Expr.var 0))
    (Expr.existsBounded (Expr.const 2) (Expr.var 0)
      (Expr.andE (Expr.isPrime (Expr.var 0))
                 (Expr.isPrime (Expr.add (Expr.var 0) (Expr.const 2)))))

def inv : Expr := Expr.implies (Expr.lt (Expr.var 0) (Expr.const 200)) body

theorem solved : ∀ n : Nat, toProp inv n :=
  bounded_vacuous_forall_lt inv body 200 200 rfl (by omega) (by native_decide)

def decided : Universe.DecidedProp where
  S := ∀ n : Nat, toProp inv n
  dec := true
  sound := fun _ => solved
  complete := fun h => Bool.noConfusion h

end Generated.TwinPrimes
