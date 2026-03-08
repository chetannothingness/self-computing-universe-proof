import Mathlib.Data.Nat.Prime.Basic

/-!
# Bertrand's Postulate — Bounded Fragment

For every n ≥ 1, there exists a prime p such that n < p ≤ 2n.
Proven by Chebyshev (1852). Bounded fragment verifies for n in [1, N].
-/

namespace OpenProblems.Bertrand

/-- Bertrand bounded: for every n in [1, N], there exists a prime in (n, 2n]. -/
def bertrandBounded (hi : Nat) : Prop :=
  ∀ n, 1 ≤ n → n ≤ hi →
    ∃ p, Nat.Prime p ∧ n < p ∧ p ≤ 2 * n

/-- Full theorem (proven by Chebyshev 1852, documentation only). -/
def bertrandFull : Prop :=
  ∀ n, n ≥ 1 → ∃ p, Nat.Prime p ∧ n < p ∧ p ≤ 2 * n

end OpenProblems.Bertrand
