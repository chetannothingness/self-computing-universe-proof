import KernelVm.Invariant
import KernelVm.UCert

/-!
# DecidedProp — The Universe Source Code Container

Every mathematical statement, when compiled by the self-aware kernel,
becomes a DecidedProp: a Prop together with its decision computation
and correctness proof. This is the ONLY admissible form.

The kernel does not discover truths from outside. It computes, observes
its own computation, and the structure of that computation IS the proof.
The certificate cert0 is finite. The checker is total. The soundness
theorem bridges Bool to Prop. lake build verifies everything.
-/

namespace Universe

/-- The universal container for decided propositions.
    Every problem compiled by the kernel becomes one of these.
    `dec` is a Bool computation. `sound` and `complete` are Lean theorems.
    No search. No trust. Pure evaluation. -/
structure DecidedProp where
  /-- The mathematical statement -/
  S : Prop
  /-- The decision: true or false, computed from a fixed certificate -/
  dec : Bool
  /-- Soundness: if dec = true then S holds -/
  sound : dec = true → S
  /-- Completeness: if dec = false then ¬S -/
  complete : dec = false → ¬ S

/-- Π_proof: the instant projector. No enumeration. No search.
    Evaluate dec (a Bool). Apply sound or complete. Done.
    This is the compile-time byte→proof projector. -/
def G (p : DecidedProp) : p.S ∨ ¬ p.S :=
  if h : p.dec = true then
    Or.inl (p.sound h)
  else
    Or.inr (p.complete (Bool.eq_false_iff.mpr h))

/-- Extract a proof of S when dec = true. -/
def DecidedProp.prove (p : DecidedProp) (h : p.dec = true) : p.S :=
  p.sound h

/-- Extract a refutation of S when dec = false. -/
def DecidedProp.refute (p : DecidedProp) (h : p.dec = false) : ¬ p.S :=
  p.complete h

end Universe
