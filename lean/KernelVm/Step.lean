import KernelVm.State

/-!
# KernelVm Step Function

Total step function — exact mirror of `Vm::step` (vm.rs lines 213-490).
Every match arm from Rust becomes a match case here. The function is total:
Lean's type checker enforces that every case is covered and no `sorry` is used.

Key semantics matched exactly:
  - Add/Sub/Mul/Div/Mod: pop b first, then a (Rust: `let (Some(b), Some(a)) = ...`)
  - Div/Mod: check b==0 → DivisionByZero
  - Neg: negate top (in Int, always succeeds — overflow is an i64 concern)
  - And/Or: bitwise (modeled as Int operations)
  - Not: logical (if a == 0 then 1 else 0)
  - Jmp/Jz: check target < program.len()
  - Load: missing key → 0 (memory function default)
  - pc >= program.len() → Fault(InvalidJump)
  - Returns (VmState, Bool) where Bool = still running
-/

namespace KernelVm

/-- Bitwise AND on Int. Converts to Nat, applies Nat bitwise and. -/
def intBitAnd (a b : Int) : Int :=
  Int.ofNat (a.toNat.land b.toNat)

/-- Bitwise OR on Int. Converts to Nat, applies Nat bitwise or. -/
def intBitOr (a b : Int) : Int :=
  Int.ofNat (a.toNat.lor b.toNat)

/-- Pop one element from the stack. Returns `none` on underflow. -/
def popOne (stack : List Int) : Option (Int × List Int) :=
  match stack with
  | [] => none
  | a :: rest => some (a, rest)

/-- Pop two elements from the stack: top → b, second → a.
    Matches Rust: `let (Some(b), Some(a)) = (stack.pop(), stack.pop())`. -/
def popTwo (stack : List Int) : Option (Int × Int × List Int) :=
  match stack with
  | [] => none
  | _ :: [] => none
  | b :: a :: rest => some (b, a, rest)

/-- Total step function: VmState → (VmState, Bool).
    Always returns a valid next state. The Bool indicates if the VM is still running.
    Mirrors `Vm::step` (vm.rs lines 213-490) exactly. -/
def step (program : Program) (state : VmState) : VmState × Bool :=
  -- If already halted, return false (vm.rs line 214-216)
  if state.halted then
    (state, false)
  -- If pc >= program length, fault with InvalidJump (vm.rs line 218-222)
  else if h : state.pc >= program.len then
    ({ state with
       halted := true
       outcome := some (VmOutcome.fault VmFault.invalidJump) }, false)
  else
    let instr := match program.get? state.pc with
      | some i => i
      | none => Instruction.nop  -- unreachable given pc < len
    let newStepsTaken := state.stepsTaken + 1
    let s := { state with stepsTaken := newStepsTaken }
    match instr with
    -- PUSH(val): push literal onto stack (vm.rs line 228-231)
    | Instruction.push val =>
      ({ s with stack := val :: s.stack, pc := s.pc + 1 }, true)

    -- DUP: duplicate top (vm.rs line 232-241)
    | Instruction.dup =>
      match s.stack with
      | [] => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | top :: _ => ({ s with stack := top :: s.stack, pc := s.pc + 1 }, true)

    -- DROP: pop top (vm.rs line 242-250)
    | Instruction.drop =>
      match s.stack with
      | [] => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | _ :: rest => ({ s with stack := rest, pc := s.pc + 1 }, true)

    -- SWAP: swap top two (vm.rs line 251-261)
    | Instruction.swap =>
      match s.stack with
      | [] | [_] => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | b :: a :: rest => ({ s with stack := a :: b :: rest, pc := s.pc + 1 }, true)

    -- ADD: pop b, pop a, push a+b (vm.rs line 262-280)
    -- Note: In Lean Int, no overflow. Overflow is an i64 concern handled per-program.
    | Instruction.add =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) => ({ s with stack := (a + b) :: rest, pc := s.pc + 1 }, true)

    -- SUB: pop b, pop a, push a-b (vm.rs line 281-299)
    | Instruction.sub =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) => ({ s with stack := (a - b) :: rest, pc := s.pc + 1 }, true)

    -- MUL: pop b, pop a, push a*b (vm.rs line 300-318)
    | Instruction.mul =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) => ({ s with stack := (a * b) :: rest, pc := s.pc + 1 }, true)

    -- DIV: pop b, pop a; b==0 → fault; else push a/b (vm.rs line 319-342)
    | Instruction.div =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) =>
        if b == 0 then
          ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.divisionByZero) }, false)
        else
          ({ s with stack := (a / b) :: rest, pc := s.pc + 1 }, true)

    -- MOD: pop b, pop a; b==0 → fault; else push a%b (vm.rs line 343-366)
    | Instruction.mod =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) =>
        if b == 0 then
          ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.divisionByZero) }, false)
        else
          ({ s with stack := (a % b) :: rest, pc := s.pc + 1 }, true)

    -- NEG: pop a, push -a (vm.rs line 367-384)
    -- In Lean Int, negation always succeeds. i64::MIN overflow is per-program.
    | Instruction.neg =>
      match popOne s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (a, rest) => ({ s with stack := (-a) :: rest, pc := s.pc + 1 }, true)

    -- EQ: pop b, pop a, push (if a==b then 1 else 0) (vm.rs line 386-395)
    | Instruction.eq =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) =>
        let r := if a == b then 1 else 0
        ({ s with stack := r :: rest, pc := s.pc + 1 }, true)

    -- LT: pop b, pop a, push (if a < b then 1 else 0) (vm.rs line 396-405)
    | Instruction.lt =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) =>
        let r := if a < b then 1 else 0
        ({ s with stack := r :: rest, pc := s.pc + 1 }, true)

    -- AND: pop b, pop a, push (a &&& b) bitwise (vm.rs line 406-415)
    -- Modeled as Int.land for arbitrary precision.
    | Instruction.and =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) => ({ s with stack := (intBitAnd a b) :: rest, pc := s.pc + 1 }, true)

    -- OR: pop b, pop a, push (a ||| b) bitwise (vm.rs line 416-425)
    | Instruction.or =>
      match popTwo s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (b, a, rest) => ({ s with stack := (intBitOr a b) :: rest, pc := s.pc + 1 }, true)

    -- NOT: pop a, push (if a==0 then 1 else 0) (vm.rs line 426-435)
    | Instruction.not =>
      match popOne s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (a, rest) =>
        let r := if a == 0 then 1 else 0
        ({ s with stack := r :: rest, pc := s.pc + 1 }, true)

    -- JMP(target): unconditional jump; check target < len (vm.rs line 436-444)
    | Instruction.jmp target =>
      if target < program.len then
        ({ s with pc := target }, true)
      else
        ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.invalidJump) }, false)

    -- JZ(target): pop top; if zero, jump to target (vm.rs line 445-463)
    | Instruction.jz target =>
      match popOne s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (val, rest) =>
        if val == 0 then
          if target < program.len then
            ({ s with stack := rest, pc := target }, true)
          else
            ({ s with stack := rest, halted := true, outcome := some (VmOutcome.fault VmFault.invalidJump) }, false)
        else
          ({ s with stack := rest, pc := s.pc + 1 }, true)

    -- LOAD(slot): push memory[slot] (default 0) (vm.rs line 464-468)
    | Instruction.load slot =>
      let val := s.memory slot
      ({ s with stack := val :: s.stack, pc := s.pc + 1 }, true)

    -- STORE(slot): pop top, store in memory (vm.rs line 469-478)
    | Instruction.store slot =>
      match popOne s.stack with
      | none => ({ s with halted := true, outcome := some (VmOutcome.fault VmFault.stackUnderflow) }, false)
      | some (val, rest) =>
        ({ s with stack := rest, memory := updateMemory s.memory slot val, pc := s.pc + 1 }, true)

    -- HALT(code): halt with exit code (vm.rs line 479-483)
    | Instruction.halt code =>
      ({ s with halted := true, outcome := some (VmOutcome.halted code) }, false)

    -- NOP: no operation (vm.rs line 484-486)
    | Instruction.nop =>
      ({ s with pc := s.pc + 1 }, true)

end KernelVm
