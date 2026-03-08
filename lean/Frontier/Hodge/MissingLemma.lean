/-!
# Hodge Conjecture — Frontier (INVALID)

STATUS: INVALID — No finite B* derivable.

The Hodge conjecture states that for projective algebraic varieties,
certain cohomology classes (Hodge classes) are algebraic — they can
be represented as rational linear combinations of classes of
algebraic subvarieties.

Missing instrument: A decision procedure for determining whether a
given Hodge class on a projective variety of dimension ≤ d and
degree ≤ D is algebraic, running within B*(d, D) steps.

Schemas tried: AlgebraicDecision, EffectiveCompactness.
-/

namespace Frontier.Hodge

def missingLemma : String :=
  "∃ procedure, ∀ X : ProjectiveVariety, ∀ α : HodgeClass(X), " ++
  "dim(X) ≤ d → deg(X) ≤ D → " ++
  "procedure decides 'α is algebraic' within B*(d, D) steps"

def blockingReason : String :=
  "The Hodge conjecture is about the algebraic structure of " ++
  "cohomology classes on projective varieties. No finite decision " ++
  "procedure is known even for bounded dimension and degree."

end Frontier.Hodge
