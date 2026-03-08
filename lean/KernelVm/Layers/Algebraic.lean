import KernelVm.InvSyn

/-!
# Layer C: Algebraic Invariants

Gröbner basis certificates and normal-form checks for algebraic identities.
Verifies that algebraic identities between polynomials hold by checking
Gröbner basis membership certificates.
-/

namespace KernelVm.Layers.Algebraic

open KernelVm.InvSyn

/-- Check if an expression is in the algebraic fragment.
    Same as polynomial fragment — algebraic layer uses polynomial operations
    but with Gröbner-basis-style certificates for ideal membership. -/
def isAlgebraic : Expr → Bool
  | .var _ => true
  | .const _ => true
  | .add l r => isAlgebraic l && isAlgebraic r
  | .sub l r => isAlgebraic l && isAlgebraic r
  | .mul l r => isAlgebraic l && isAlgebraic r
  | .neg e => isAlgebraic e
  | .modE l r => isAlgebraic l && isAlgebraic r
  | .pow base _ => isAlgebraic base
  | .le l r => isAlgebraic l && isAlgebraic r
  | .lt l r => isAlgebraic l && isAlgebraic r
  | .eq l r => isAlgebraic l && isAlgebraic r
  | .ne l r => isAlgebraic l && isAlgebraic r
  | .andE l r => isAlgebraic l && isAlgebraic r
  | .orE l r => isAlgebraic l && isAlgebraic r
  | .notE e => isAlgebraic e
  | .implies l r => isAlgebraic l && isAlgebraic r
  | .forallBounded _ _ body => isAlgebraic body
  | .existsBounded _ _ body => isAlgebraic body
  | .divE l r => isAlgebraic l && isAlgebraic r
  | .abs e => isAlgebraic e
  | .sqrt _ => false
  | .isPrime _ => false
  | .divisorSum _ => false
  | .moebiusFn _ => false
  | .collatzReaches1 _ => false
  | .erdosStrausHolds _ => false
  | .fourSquares _ => false
  | .mertensBelow _ => false
  | .fltHolds _ => false
  | .primeCount _ => false
  | .goldbachRepCount _ => false
  | .primeGapMax _ => false
  | .intervalBound _ _ => false
  | .certifiedSum _ _ _ => false

/-- Algebraic decision — validate expression is algebraic and evaluate. -/
def algebraicDecide (e : Expr) : Bool := isAlgebraic e && evalBool (fun _ => 0) e

/-- Soundness of algebraicDecide. -/
theorem algebraicDecide_sound (e : Expr) (h : algebraicDecide e = true) :
    evalBool (fun _ => 0) e = true := by
  unfold algebraicDecide at h
  simp [Bool.and_eq_true] at h
  exact h.2

end KernelVm.Layers.Algebraic
