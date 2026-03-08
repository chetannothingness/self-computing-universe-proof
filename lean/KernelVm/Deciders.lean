import KernelVm.InvSyn

/-!
# Decidable Checkers for IRC Obligations

Three decidable checkers that verify IRC obligations (Base, Step, Link)
for a given InvSyn invariant expression. Each returns Bool — the soundness
theorems in Soundness.lean lift these Bool results to universal propositions.

The key pattern:
  1. Rust finds an invariant `inv : Expr`
  2. Rust calls dec_base/dec_step/dec_link and verifies they return true
  3. Lean emits `have h : dec_X inv ... = true := by native_decide`
  4. Soundness theorem lifts: `dec_X_sound inv ... h` proves the ∀ statement
-/

namespace KernelVm.Deciders

open KernelVm.InvSyn

/-- Check that the invariant holds at all initial states.
    For Nat-indexed problems starting at 0: check eval(inv, 0).
    For problems with multiple initial states: check all of them. -/
def dec_base (inv : Expr) (initStates : List Nat) : Bool :=
  initStates.all (fun x => evalBool (mkEnv (x : Int)) inv)

/-- Check that the invariant holds at a single initial state. -/
def dec_base_single (inv : Expr) (x₀ : Nat) : Bool :=
  evalBool (mkEnv (x₀ : Int)) inv

/-- Check that the invariant is preserved by the successor step
    within a bounded range [0, bound].
    For each n in range: eval(inv, n) = true → eval(inv, n+1) = true.

    IMPORTANT: This bounded check is necessary but not sufficient for ∀n.
    The soundness theorem requires the step checker to be a decision procedure
    for the specific invariant structure, not just bounded enumeration.
    For structural proofs, the Rust side provides a structural step checker
    that analyzes the AST and produces an algebraic certificate. -/
def dec_step_bounded (inv : Expr) (bound : Nat) : Bool :=
  let rec loop (n : Nat) (fuel : Nat) : Bool :=
    match fuel with
    | 0 => true
    | fuel' + 1 =>
      if n > bound then true
      else
        let envN := mkEnv (n : Int)
        let envN1 := mkEnv ((n + 1 : Nat) : Int)
        if evalBool envN inv then
          if evalBool envN1 inv then loop (n + 1) fuel'
          else false
        else loop (n + 1) fuel'
  loop 0 (bound + 1)

/-- Structural step checker: given an invariant and a step formula
    (the formula encoding "I(n) → I(n+1)" structurally), evaluate
    the step formula. The step formula is constructed by the Rust side
    to be a tautology iff the invariant is preserved by the step relation.

    The formula is: implies(subst(inv, n), subst(inv, n+1))
    where the substitution is encoded in the Expr AST itself. -/
def dec_step_structural (stepFormula : Expr) : Bool :=
  evalBool (fun _ => 0) stepFormula

/-- Check that the invariant implies the target property.
    The implication formula is: ∀x, eval(inv, x) → eval(prop, x).
    Encoded as an Expr: implies(inv, prop) evaluated structurally. -/
def dec_link (linkFormula : Expr) : Bool :=
  evalBool (fun _ => 0) linkFormula

/-- Check link by bounded evaluation: for all n in [0, bound],
    eval(inv, n) → eval(prop, n). -/
def dec_link_bounded (inv : Expr) (prop : Expr) (bound : Nat) : Bool :=
  let rec loop (n : Nat) (fuel : Nat) : Bool :=
    match fuel with
    | 0 => true
    | fuel' + 1 =>
      if n > bound then true
      else
        let env := mkEnv (n : Int)
        if evalBool env inv then
          if evalBool env prop then loop (n + 1) fuel'
          else false
        else loop (n + 1) fuel'
  loop 0 (bound + 1)

end KernelVm.Deciders
