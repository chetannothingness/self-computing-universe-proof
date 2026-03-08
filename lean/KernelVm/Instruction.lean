/-!
# KernelVm Instruction Set

Exact mirror of `kernel-frc/src/vm.rs` Instruction enum (lines 38-60).
21 instructions: stack-based, finite, total, deterministic.
-/

namespace KernelVm

/-- VM instruction — self-delimiting, canonical.
    Mirrors Rust `Instruction` enum exactly. -/
inductive Instruction where
  | push (val : Int)
  | dup
  | drop
  | swap
  | add
  | sub
  | mul
  | div
  | mod
  | neg
  | eq
  | lt
  | and
  | or
  | not
  | jmp (target : Nat)
  | jz (target : Nat)
  | load (slot : Nat)
  | store (slot : Nat)
  | halt (code : UInt8)
  | nop
  deriving Repr, BEq, DecidableEq

/-- A program is a finite sequence of instructions. -/
structure Program where
  instructions : List Instruction
  deriving Repr, BEq

namespace Program

def len (p : Program) : Nat := p.instructions.length

def isEmpty (p : Program) : Bool := p.instructions.isEmpty

def get? (p : Program) (i : Nat) : Option Instruction :=
  p.instructions.get? i

end Program

end KernelVm
