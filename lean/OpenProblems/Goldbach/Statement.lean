import Mathlib.Data.Nat.Prime.Basic

/-!
# Goldbach Conjecture — Bounded Fragment

Every even integer n ≥ 4 is the sum of two primes.
Bounded fragment: verified for all even n in [4, N].
-/

namespace OpenProblems.Goldbach

def isSumOfTwoPrimes (n : Nat) : Prop :=
  ∃ p q, Nat.Prime p ∧ Nat.Prime q ∧ p + q = n

/-- Goldbach bounded: every even n in [lo, hi] is the sum of two primes. -/
def goldbachBounded (lo hi : Nat) : Prop :=
  ∀ n, lo ≤ n → n ≤ hi → n % 2 = 0 → isSumOfTwoPrimes n

/-- Full conjecture (documentation only — not computationally verifiable). -/
def goldbachFull : Prop :=
  ∀ n, n ≥ 4 → n % 2 = 0 → isSumOfTwoPrimes n

end OpenProblems.Goldbach
