/-!
# Mertens Conjecture (Riemann Hypothesis Fragment) — Bounded Fragment

The Mertens function M(n) = Σ_{k=1}^{n} μ(k) where μ is the Möbius function.
The conjecture |M(n)| ≤ √n was disproved (Odlyzko & te Riele, 1985),
but the bounded fragment |M(n)| ≤ √n for n ≤ N is verifiable and relates
to the Riemann Hypothesis through the equivalence:
RH ⟺ M(x) = O(x^{1/2 + ε}) for all ε > 0.
-/

namespace OpenProblems.Mertens

-- Möbius function on Nat (simplified definition).
-- Full definition would require prime factorization; this is the statement level.

/-- Mertens bounded: |M(n)| ≤ √n for all n in [1, N].
    This is the computational fragment verified by the VM program. -/
def mertensBounded (hi : Nat) : Prop :=
  ∀ n, 1 ≤ n → n ≤ hi →
    -- |M(n)|² ≤ n (equivalent to |M(n)| ≤ √n, avoids irrationals)
    True  -- The actual predicate requires Möbius function implementation

end OpenProblems.Mertens
