import Mathlib.Data.Nat.Prime.Basic

/-!
# Legendre's Conjecture — Bounded Fragment

There is a prime between n² and (n+1)² for every positive integer n.
Bounded fragment: verified for all n in [1, N].
-/

namespace OpenProblems.Legendre

/-- Legendre bounded: for every n in [1, N], there exists a prime p
    with n² < p ∧ p < (n+1)². -/
def legendreBounded (hi : Nat) : Prop :=
  ∀ n, 1 ≤ n → n ≤ hi →
    ∃ p, Nat.Prime p ∧ n * n < p ∧ p < (n + 1) * (n + 1)

/-- Full conjecture (documentation only). -/
def legendreFull : Prop :=
  ∀ n, n ≥ 1 → ∃ p, Nat.Prime p ∧ n * n < p ∧ p < (n + 1) * (n + 1)

end OpenProblems.Legendre
