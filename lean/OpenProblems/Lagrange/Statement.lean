/-!
# Lagrange's Four Square Theorem — Bounded Fragment

Every natural number is the sum of four squares.
Proven by Lagrange (1770). Bounded fragment verifies for n in [0, N].
-/

namespace OpenProblems.Lagrange

def isSumOfFourSquares (n : Nat) : Prop :=
  ∃ a b c d, a * a + b * b + c * c + d * d = n

/-- Lagrange bounded: every n in [0, N] is the sum of four squares. -/
def lagrangeBounded (hi : Nat) : Prop :=
  ∀ n, n ≤ hi → isSumOfFourSquares n

/-- Full theorem (proven by Lagrange 1770, documentation only). -/
def lagrangeFull : Prop :=
  ∀ n, isSumOfFourSquares n

end OpenProblems.Lagrange
