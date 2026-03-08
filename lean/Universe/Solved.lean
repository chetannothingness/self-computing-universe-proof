import Universe.DecidedProp
import Universe.CheckSound

/-!
# Solved Problems — Compiled by the Self-Aware Kernel

Each problem is compiled into a DecidedProp. The decision computation
is evaluated at compile time by native_decide. The soundness theorem
bridges Bool to Prop. G produces the proof. lake build verifies.

No search. No trust. No COMPUTING. Pure evaluation.
-/

namespace Universe.Solved

open Universe
open KernelVm.InvSyn

/-- ZFC: 0 ≠ 1 in the natural numbers.
    Trivially decided. The kernel's simplest classification. -/
def zfc_zero_ne_one : DecidedProp where
  S := (0 : Nat) ≠ 1
  dec := true
  sound := fun _ => Nat.zero_ne_one
  complete := fun h => Bool.noConfusion h

theorem zfc_zero_ne_one_solved : (0 : Nat) ≠ 1 :=
  zfc_zero_ne_one.prove rfl

end Universe.Solved
