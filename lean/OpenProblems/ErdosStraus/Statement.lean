/-!
# Erdős–Straus Conjecture — Bounded Fragment

For every integer n ≥ 2, 4/n = 1/x + 1/y + 1/z for some positive integers x, y, z.
Bounded fragment: verified for all n in [2, N].
-/

namespace OpenProblems.ErdosStraus

/-- Erdős–Straus decomposition exists for n. -/
def hasDecomposition (n : Nat) : Prop :=
  ∃ x y z : Nat, x > 0 ∧ y > 0 ∧ z > 0 ∧
    4 * x * y * z = n * (y * z + x * z + x * y)

/-- Erdős–Straus bounded: every n in [2, N] has a decomposition. -/
def erdosStrausBounded (hi : Nat) : Prop :=
  ∀ n, 2 ≤ n → n ≤ hi → hasDecomposition n

/-- Full conjecture (documentation only). -/
def erdosStrausFull : Prop :=
  ∀ n, n ≥ 2 → hasDecomposition n

end OpenProblems.ErdosStraus
