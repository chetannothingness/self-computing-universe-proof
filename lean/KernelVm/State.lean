import KernelVm.Instruction

/-!
# KernelVm State

Mirrors `kernel-frc/src/vm.rs` VmState (lines 127-155), VmOutcome (lines 94-109),
VmFault (lines 111-125).

Uses `Int` (arbitrary precision) for stack values. Per-program overflow absence
is a separate obligation, provable because all intermediate values in the 14
programs are bounded by small parameters.
-/

namespace KernelVm

/-- VM fault — deterministic error category.
    Mirrors Rust `VmFault` enum. -/
inductive VmFault where
  | stackUnderflow
  | divisionByZero
  | invalidJump
  | overflow
  | memoryOutOfBounds
  deriving Repr, BEq, DecidableEq

/-- VM execution outcome — always total, never undefined.
    Mirrors Rust `VmOutcome` enum. -/
inductive VmOutcome where
  | halted (code : UInt8)
  | budgetExhausted
  | fault (f : VmFault)
  deriving Repr, BEq, DecidableEq

/-- VM state — complete snapshot for replay and determinism proof.
    Mirrors Rust `VmState` struct.

    Memory is modeled as `Nat → Int` with default 0, matching
    Rust's `BTreeMap<usize, i64>` with `unwrap_or(0)` semantics. -/
structure VmState where
  pc : Nat
  stack : List Int
  memory : Nat → Int
  stepsTaken : Nat
  halted : Bool
  outcome : Option VmOutcome

instance : Repr VmState where
  reprPrec s _ :=
    "{ pc := " ++ repr s.pc ++
    ", stack := " ++ repr s.stack ++
    ", memory := <fn>" ++
    ", stepsTaken := " ++ repr s.stepsTaken ++
    ", halted := " ++ repr s.halted ++
    ", outcome := " ++ repr s.outcome ++ " }"

/-- Default memory: all slots are 0. -/
def defaultMemory : Nat → Int := fun _ => 0

/-- Initial VM state: pc=0, empty stack, zeroed memory, not halted. -/
def VmState.initial : VmState :=
  { pc := 0
  , stack := []
  , memory := defaultMemory
  , stepsTaken := 0
  , halted := false
  , outcome := none }

/-- Update a single memory slot, returning a new memory function. -/
def updateMemory (mem : Nat → Int) (slot : Nat) (val : Int) : Nat → Int :=
  fun s => if s == slot then val else mem s

end KernelVm
