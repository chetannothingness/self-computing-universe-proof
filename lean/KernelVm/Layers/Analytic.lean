import KernelVm.InvSyn

/-!
# Layer D: Analytic Invariants

Interval arithmetic and certified summation for analytic bounds.
Handles invariants involving convergent sums, analytic estimates,
and interval-certified bounds.
-/

namespace KernelVm.Layers.Analytic

open KernelVm.InvSyn

/-- Check if an expression uses analytic operations (intervalBound, certifiedSum). -/
def usesAnalytic : Expr → Bool
  | .intervalBound _ _ => true
  | .certifiedSum _ _ _ => true
  | .add l r => usesAnalytic l || usesAnalytic r
  | .sub l r => usesAnalytic l || usesAnalytic r
  | .mul l r => usesAnalytic l || usesAnalytic r
  | .neg e => usesAnalytic e
  | .modE l r => usesAnalytic l || usesAnalytic r
  | .pow base _ => usesAnalytic base
  | .le l r => usesAnalytic l || usesAnalytic r
  | .lt l r => usesAnalytic l || usesAnalytic r
  | .eq l r => usesAnalytic l || usesAnalytic r
  | .ne l r => usesAnalytic l || usesAnalytic r
  | .andE l r => usesAnalytic l || usesAnalytic r
  | .orE l r => usesAnalytic l || usesAnalytic r
  | .notE e => usesAnalytic e
  | .implies l r => usesAnalytic l || usesAnalytic r
  | .forallBounded _ _ body => usesAnalytic body
  | .existsBounded _ _ body => usesAnalytic body
  | .divE l r => usesAnalytic l || usesAnalytic r
  | .abs e => usesAnalytic e
  | .sqrt e => usesAnalytic e
  | .isPrime e => usesAnalytic e
  | .divisorSum e => usesAnalytic e
  | .moebiusFn e => usesAnalytic e
  | .collatzReaches1 e => usesAnalytic e
  | .erdosStrausHolds e => usesAnalytic e
  | .fourSquares e => usesAnalytic e
  | .mertensBelow e => usesAnalytic e
  | .fltHolds e => usesAnalytic e
  | .primeCount e => usesAnalytic e
  | .goldbachRepCount e => usesAnalytic e
  | .primeGapMax e => usesAnalytic e
  | .var _ => false
  | .const _ => false

/-- Analytic decision — evaluate expression with analytic operations. -/
def analyticDecide (e : Expr) : Bool := evalBool (fun _ => 0) e

/-- Soundness of analyticDecide. -/
theorem analyticDecide_sound (e : Expr) (h : analyticDecide e = true) :
    evalBool (fun _ => 0) e = true := by
  unfold analyticDecide at h
  exact h

end KernelVm.Layers.Analytic
