/-!
# P vs NP — Frontier (INVALID)

STATUS: INVALID — No finite B* derivable.

The P vs NP problem asks whether every problem whose solution can be
quickly verified (NP) can also be quickly solved (P). No FRC exists because:

Missing instrument: A proof that all poly-time Turing machines of size ≤ n
are enumerable below an explicit bound B*(n), with decidable runtime verification.

Schemas tried: BoundedCounterexample, FiniteSearch, EffectiveCompactness,
ProofMining, AlgebraicDecision.

This is the honest boundary of FRC admissibility: the problem requires
reasoning about all possible algorithms, which cannot be bounded finitely.
-/

namespace Frontier.PvsNP

/-- The missing lemma: decidable enumeration of poly-time TMs. -/
def missingLemma : String :=
  "∀ n : Nat, ∃ B : Nat, ∀ M : TuringMachine, " ++
  "size(M) ≤ n → runtime(M) ∈ O(poly(input_size)) → " ++
  "M is enumerable below index B"

/-- Why this blocks FRC: without this bound, no finite program C
    can decide P ≠ NP (or P = NP) within B* steps. -/
def blockingReason : String :=
  "FRC requires a finite computation C that decides the statement within B* steps. " ++
  "P vs NP requires reasoning over the space of all possible algorithms, " ++
  "which has no known finite enumeration bound."

end Frontier.PvsNP
