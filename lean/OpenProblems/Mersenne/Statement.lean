import Mathlib.Data.Nat.Prime.Basic

/-!
# Mersenne Prime — Bounded Fragment

A Mersenne prime is a prime of the form 2^p - 1 where p is prime.
Bounded fragment: there exists a Mersenne prime for some prime p in [2, P].
-/

namespace OpenProblems.Mersenne

def isMersennePrime (p : Nat) : Prop :=
  Nat.Prime p ∧ Nat.Prime (2 ^ p - 1)

/-- Mersenne bounded: there exists a Mersenne prime with exponent p ≤ P. -/
def mersenneBounded (maxP : Nat) : Prop :=
  ∃ p, 2 ≤ p ∧ p ≤ maxP ∧ isMersennePrime p

end OpenProblems.Mersenne
