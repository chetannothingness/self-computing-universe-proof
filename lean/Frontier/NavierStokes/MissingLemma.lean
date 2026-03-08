/-!
# Navier-Stokes Existence and Smoothness — Frontier (INVALID)

STATUS: INVALID — No finite B* derivable.

The problem asks whether smooth, globally defined solutions to the
3D incompressible Navier-Stokes equations always exist. This is
inherently about continuous PDE solutions, which cannot be reduced
to a finite VM computation.

Missing instrument: A discretization scheme that provably captures
all blowup behavior of 3D Navier-Stokes within a finite grid of
size N, with error bounds certifying that smooth solutions on the
grid imply smooth solutions in the continuum.

Schemas tried: CertifiedNumerics, EffectiveCompactness.
-/

namespace Frontier.NavierStokes

def missingLemma : String :=
  "∃ discretization D, ∀ initial_data u₀ : H¹(R³), " ++
  "smooth(D(u₀, N)) for all N → smooth(NS_solution(u₀))"

def blockingReason : String :=
  "Navier-Stokes is a PDE existence problem in continuous space. " ++
  "No known discretization can certify that finite computation " ++
  "implies existence of smooth solutions for all time."

end Frontier.NavierStokes
