import KernelVm.Invariant
import KernelVm.InvSyn
import OpenProblems.Lagrange.Statement
import Mathlib.NumberTheory.SumFourSquares

/-!
# Lagrange's Four-Square Theorem — IRC: PROVED (Lagrange 1770)

Invariant: I(n) = ∀m ≤ n, ∃ a b c d, a²+b²+c²+d² = m.
  Base: 0 = 0²+0²+0²+0².  PROVED.
  Step: via Mathlib `Nat.sum_four_squares`.  PROVED.
  Link: trivial.  PROVED.

Kernel status: PROVED (3/3 obligations discharged). UNBOUNDED.
-/

namespace OpenProblems.Lagrange

/-- Prefix invariant: four-square property holds for all m up to n. -/
def lagrangeInvariant (n : Nat) : Prop :=
  ∀ m, m ≤ n → isSumOfFourSquares m

/-- Base: 0 = 0² + 0² + 0² + 0². -/
theorem lagrange_base : lagrangeInvariant 0 := by
  intro m hm
  have : m = 0 := by omega
  subst this
  exact ⟨0, 0, 0, 0, rfl⟩

/-- Bridge: Mathlib's a^2 representation to our a*a representation. -/
private theorem pow_two_eq_mul (a : Nat) : a ^ 2 = a * a := by ring

/-- Step: Lagrange's four-square theorem from Mathlib. -/
theorem lagrange_step (n : Nat) (h : lagrangeInvariant n) : lagrangeInvariant (n + 1) := by
  intro m hm
  by_cases heq : m ≤ n
  · exact h m heq
  · have : m = n + 1 := by omega
    subst this
    obtain ⟨a, b, c, d, hsum⟩ := Nat.sum_four_squares (n + 1)
    exact ⟨a, b, c, d, by linarith [pow_two_eq_mul a, pow_two_eq_mul b, pow_two_eq_mul c, pow_two_eq_mul d]⟩

/-- Link: trivial — I(n) contains the property at m = n. -/
theorem lagrange_link (n : Nat) (h : lagrangeInvariant n) :
    isSumOfFourSquares n := by
  exact h n (Nat.le.refl)

/-- Full IRC. UNBOUNDED. -/
noncomputable def lagrangeIRC : KernelVm.Invariant.IRC isSumOfFourSquares :=
  { I := lagrangeInvariant
    base := lagrange_base
    step := lagrange_step
    link := lagrange_link }

/-- UNBOUNDED: ∀ n, n = a² + b² + c² + d². -/
theorem lagrange_full : ∀ n, isSumOfFourSquares n :=
  KernelVm.Invariant.irc_implies_forall lagrangeIRC

end OpenProblems.Lagrange
