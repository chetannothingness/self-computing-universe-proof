/-!
# Odd Perfect Number — Bounded Fragment

No odd perfect number is known to exist.
Bounded fragment: no odd perfect number in [1, N].
-/

namespace OpenProblems.OddPerfect

def divisorSum (n : Nat) : Nat :=
  (List.range n).foldl (fun acc d => if d > 0 ∧ n % d = 0 then acc + d else acc) 0

def isPerfect (n : Nat) : Prop := n > 0 ∧ divisorSum n = n

/-- Odd perfect bounded: no odd perfect number in [1, N]. -/
def oddPerfectBounded (hi : Nat) : Prop :=
  ∀ n, 1 ≤ n → n ≤ hi → n % 2 = 1 → ¬isPerfect n

/-- Full conjecture (documentation only). -/
def noOddPerfect : Prop :=
  ∀ n, n % 2 = 1 → ¬isPerfect n

end OpenProblems.OddPerfect
