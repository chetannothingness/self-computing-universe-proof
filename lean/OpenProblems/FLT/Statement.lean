/-!
# Fermat's Last Theorem — Bounded Fragment

No three positive integers a, b, c satisfy a^n + b^n = c^n for n > 2.
Bounded fragment: verified for n in [3, E] and a,b,c in [1, B].
(FLT was proven by Wiles in 1995; this is the finite computational check.)
-/

namespace OpenProblems.FLT

/-- FLT bounded: no a^n + b^n = c^n for n in [3, maxExp], a,b,c in [1, maxBase]. -/
def fltBounded (maxExp maxBase : Nat) : Prop :=
  ∀ n a b c,
    3 ≤ n → n ≤ maxExp →
    1 ≤ a → a ≤ maxBase →
    1 ≤ b → b ≤ maxBase →
    1 ≤ c → c ≤ maxBase →
    a ^ n + b ^ n ≠ c ^ n

/-- Full theorem (proven by Wiles 1995, documentation only). -/
def fltFull : Prop :=
  ∀ n, n > 2 → ∀ a b c : Nat, a > 0 → b > 0 → c > 0 → a ^ n + b ^ n ≠ c ^ n

end OpenProblems.FLT
