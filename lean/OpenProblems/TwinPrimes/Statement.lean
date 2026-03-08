import Mathlib.Data.Nat.Prime.Basic

/-!
# Twin Prime Conjecture — Bounded Fragment

There are infinitely many primes p such that p+2 is also prime.
Bounded fragment: there exists a twin prime pair (p, p+2) with p in [2, N].
-/

namespace OpenProblems.TwinPrimes

def isTwinPrimePair (p : Nat) : Prop :=
  Nat.Prime p ∧ Nat.Prime (p + 2)

/-- Twin primes bounded: there exists a twin prime pair in [2, N]. -/
def twinPrimesBounded (hi : Nat) : Prop :=
  ∃ p, 2 ≤ p ∧ p ≤ hi ∧ isTwinPrimePair p

/-- Full conjecture (documentation only). -/
def twinPrimesFull : Prop :=
  ∀ N, ∃ p, p > N ∧ isTwinPrimePair p

end OpenProblems.TwinPrimes
