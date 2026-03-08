/-!
# Collatz Conjecture — Bounded Fragment

Every positive integer eventually reaches 1 under the 3n+1 map.
Bounded fragment: verified for all n in [1, N] within M iterations.
-/

namespace OpenProblems.Collatz

def collatzStep (n : Nat) : Nat :=
  if n % 2 = 0 then n / 2 else 3 * n + 1

/-- Iterate collatzStep k times starting from n. -/
def collatzIter (k : Nat) (n : Nat) : Nat :=
  match k with
  | 0 => n
  | k' + 1 => collatzIter k' (collatzStep n)

def reachesOne (n : Nat) (maxIter : Nat) : Prop :=
  ∃ k, k ≤ maxIter ∧ collatzIter k n = 1

/-- Collatz bounded: every n in [1, N] reaches 1 within maxIter steps. -/
def collatzBounded (hi maxIter : Nat) : Prop :=
  ∀ n, 1 ≤ n → n ≤ hi → reachesOne n maxIter

/-- Full conjecture (documentation only). -/
def collatzFull : Prop :=
  ∀ n, n ≥ 1 → ∃ k, collatzIter k n = 1

end OpenProblems.Collatz
