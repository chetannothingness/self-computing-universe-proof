import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.ZFC.Statement

/-!
# ZFC 0 ≠ 1 — IRC: PROVED

Trivial invariant: I(n) = True for all n.
All three obligations are trivially discharged.
-/

namespace OpenProblems.ZFC

/-- Trivial invariant: always True. -/
def zfcInvariant (_ : Nat) : Prop := True

theorem zfc_base : zfcInvariant 0 := trivial

theorem zfc_step (n : Nat) (_ : zfcInvariant n) : zfcInvariant (n + 1) := trivial

theorem zfc_link (n : Nat) (_ : zfcInvariant n) : zeroNeOne := Nat.zero_ne_one

noncomputable def zfcIRC : KernelVm.Invariant.IRC (fun _ => zeroNeOne) :=
  { I := zfcInvariant
    base := zfc_base
    step := zfc_step
    link := zfc_link }

theorem zfc_full : ∀ (_ : Nat), zeroNeOne :=
  KernelVm.Invariant.irc_implies_forall zfcIRC

end OpenProblems.ZFC
