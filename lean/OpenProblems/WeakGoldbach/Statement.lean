import Mathlib.Data.Nat.Prime.Basic

/-!
# Weak Goldbach Conjecture — Bounded Fragment

Every odd integer greater than 5 is the sum of three primes.
Proven by Helfgott (2013). Bounded fragment verifies for odd n in [7, N].
-/

namespace OpenProblems.WeakGoldbach

def isSumOfThreePrimes (n : Nat) : Prop :=
  ∃ p q r, Nat.Prime p ∧ Nat.Prime q ∧ Nat.Prime r ∧ p + q + r = n

/-- Weak Goldbach bounded: every odd n in [7, N] is sum of three primes. -/
def weakGoldbachBounded (hi : Nat) : Prop :=
  ∀ n, 7 ≤ n → n ≤ hi → n % 2 = 1 → isSumOfThreePrimes n

/-- Full theorem (proven by Helfgott 2013, documentation only). -/
def weakGoldbachFull : Prop :=
  ∀ n, n > 5 → n % 2 = 1 → isSumOfThreePrimes n

end OpenProblems.WeakGoldbach
