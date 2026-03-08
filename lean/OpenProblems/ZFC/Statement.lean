/-!
# ZFC 0 ≠ 1 — Trivial Disproof

The simplest possible FRC: 0 ≠ 1 is trivially true in any consistent system.
The VM program checks 0 ≠ 1 and halts with code 1.
-/

namespace OpenProblems.ZFC

/-- 0 ≠ 1 is trivially true. -/
def zeroNeOne : Prop := (0 : Nat) ≠ 1

theorem zeroNeOne_proof : zeroNeOne := Nat.zero_ne_one

end OpenProblems.ZFC
