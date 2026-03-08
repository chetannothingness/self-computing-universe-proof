import KernelVm.InvSyn
import KernelVm.Invariant
import Universe.StructCert
import Universe.DecidedProp

/-!
# Generated Proof: lt_one

∀ n : Nat, -1 < ↑n — proved via structural certificate pipeline.
-/

namespace Generated.LtOne

open KernelVm.InvSyn
open KernelVm.Invariant
open Universe.StructCert

def inv : Expr := Expr.lt (Expr.const (-1)) (Expr.var 0)

theorem base : toProp inv 0 := by native_decide
theorem stepOk : CheckStep inv (.ltBound (-1)) = true := by native_decide
theorem linkOk : CheckLink inv inv .identity = true := by native_decide

theorem solved : ∀ n : Nat, toProp inv n :=
  structural_proves_forall (toProp inv) inv inv (.ltBound (-1)) .identity
    base stepOk linkOk (fun _ h => h)

def decided : Universe.DecidedProp where
  S := ∀ n : Nat, toProp inv n
  dec := true
  sound := fun _ => solved
  complete := fun h => Bool.noConfusion h

end Generated.LtOne
