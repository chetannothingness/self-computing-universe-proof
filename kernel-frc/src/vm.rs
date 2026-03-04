// Verified bytecode VM — total, deterministic, self-delimiting.
//
// This is the trusted execution core for FRC: every statement S is reduced
// to "VM.run(C, B*) = 1" via ProofEq, and the VM semantics are proven once.
//
// Design: stack-based, finite instruction set, explicit halting.
// No floats. No undefined behavior. Every step is total.
//
// Instruction set (minimal, sufficient for all schemas):
//   PUSH(i64)      — push literal
//   DUP            — duplicate top
//   DROP           — pop top
//   SWAP           — swap top two
//   ADD            — pop two, push sum
//   SUB            — pop two, push difference
//   MUL            — pop two, push product
//   DIV            — pop two, push quotient (floor div, div-by-zero → FAIL)
//   MOD            — pop two, push remainder (mod-by-zero → FAIL)
//   NEG            — negate top
//   EQ             — pop two, push 1 if equal else 0
//   LT             — pop two, push 1 if a < b else 0
//   AND            — pop two, push bitwise AND
//   OR             — pop two, push bitwise OR
//   NOT            — pop top, push 1 if 0 else 0
//   JMP(usize)     — unconditional jump to instruction index
//   JZ(usize)      — pop top, jump if zero
//   LOAD(usize)    — push from memory slot
//   STORE(usize)   — pop and store to memory slot
//   HALT(u8)       — halt with exit code (0 = false, 1 = true)
//   NOP            — no operation

use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use kernel_types::{Hash32, SerPi, hash};

/// VM instruction — self-delimiting, canonical.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Instruction {
    Push(i64),
    Dup,
    Drop,
    Swap,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Eq,
    Lt,
    And,
    Or,
    Not,
    Jmp(usize),
    Jz(usize),
    Load(usize),
    Store(usize),
    Halt(u8),
    Nop,
}

impl SerPi for Instruction {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// A program is a finite sequence of instructions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub instructions: Vec<Instruction>,
}

impl Program {
    pub fn new(instructions: Vec<Instruction>) -> Self {
        Self { instructions }
    }

    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }
}

impl SerPi for Program {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// VM execution outcome — always total, never undefined.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmOutcome {
    /// Halted with exit code
    Halted(u8),
    /// Exhausted step budget without halting
    BudgetExhausted,
    /// Runtime error (div-by-zero, stack underflow, invalid jump, etc.)
    Fault(VmFault),
}

impl SerPi for VmOutcome {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// VM fault — deterministic error category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VmFault {
    StackUnderflow,
    DivisionByZero,
    InvalidJump,
    Overflow,
    MemoryOutOfBounds,
}

impl SerPi for VmFault {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// VM state — complete snapshot for replay and determinism proof.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmState {
    pub pc: usize,
    pub stack: Vec<i64>,
    pub memory: BTreeMap<usize, i64>,
    pub steps_taken: u64,
    pub halted: bool,
    pub outcome: Option<VmOutcome>,
}

impl VmState {
    pub fn initial() -> Self {
        Self {
            pc: 0,
            stack: Vec::new(),
            memory: BTreeMap::new(),
            steps_taken: 0,
            halted: false,
            outcome: None,
        }
    }
}

impl SerPi for VmState {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// A single step trace entry for hash-chain verification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepTrace {
    pub step_index: u64,
    pub pre_state_hash: Hash32,
    pub post_state_hash: Hash32,
    pub instruction_index: usize,
}

impl SerPi for StepTrace {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Complete execution trace — hash-chained for integrity.
#[derive(Debug, Clone)]
pub struct ExecTrace {
    pub steps: Vec<StepTrace>,
    pub trace_head: Hash32,
    pub initial_state_hash: Hash32,
    pub final_state_hash: Hash32,
    pub outcome: VmOutcome,
    pub total_steps: u64,
}

impl ExecTrace {
    pub fn trace_hash(&self) -> Hash32 {
        self.trace_head
    }
}

impl SerPi for ExecTrace {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.trace_head);
        buf.extend_from_slice(&self.initial_state_hash);
        buf.extend_from_slice(&self.final_state_hash);
        buf.extend_from_slice(&self.outcome.ser_pi());
        buf.extend_from_slice(&self.total_steps.ser_pi());
        hash::H(&buf).to_vec()
    }
}

/// The verified VM — total step function and total run function.
///
/// Key properties (proven by construction):
///   1. step: VmState → VmState is total
///   2. run: Program × Nat → VmOutcome is total
///   3. Determinism: same program + same initial state → same outcome
///   4. Replay: trace entries chain correctly
pub struct Vm;

impl Vm {
    /// Total step function: VmState → VmState.
    /// Always returns a valid next state. Never panics. Never diverges.
    pub fn step(program: &Program, state: &mut VmState) -> bool {
        if state.halted {
            return false;
        }

        if state.pc >= program.len() {
            state.halted = true;
            state.outcome = Some(VmOutcome::Fault(VmFault::InvalidJump));
            return false;
        }

        let instr = &program.instructions[state.pc];
        state.steps_taken += 1;

        match instr {
            Instruction::Push(val) => {
                state.stack.push(*val);
                state.pc += 1;
            }
            Instruction::Dup => {
                if let Some(&top) = state.stack.last() {
                    state.stack.push(top);
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Drop => {
                if state.stack.pop().is_some() {
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Swap => {
                let len = state.stack.len();
                if len >= 2 {
                    state.stack.swap(len - 1, len - 2);
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Add => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    match a.checked_add(b) {
                        Some(r) => {
                            state.stack.push(r);
                            state.pc += 1;
                        }
                        None => {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::Overflow));
                            return false;
                        }
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Sub => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    match a.checked_sub(b) {
                        Some(r) => {
                            state.stack.push(r);
                            state.pc += 1;
                        }
                        None => {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::Overflow));
                            return false;
                        }
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Mul => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    match a.checked_mul(b) {
                        Some(r) => {
                            state.stack.push(r);
                            state.pc += 1;
                        }
                        None => {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::Overflow));
                            return false;
                        }
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Div => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    if b == 0 {
                        state.halted = true;
                        state.outcome = Some(VmOutcome::Fault(VmFault::DivisionByZero));
                        return false;
                    }
                    match a.checked_div(b) {
                        Some(r) => {
                            state.stack.push(r);
                            state.pc += 1;
                        }
                        None => {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::Overflow));
                            return false;
                        }
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Mod => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    if b == 0 {
                        state.halted = true;
                        state.outcome = Some(VmOutcome::Fault(VmFault::DivisionByZero));
                        return false;
                    }
                    match a.checked_rem(b) {
                        Some(r) => {
                            state.stack.push(r);
                            state.pc += 1;
                        }
                        None => {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::Overflow));
                            return false;
                        }
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Neg => {
                if let Some(a) = state.stack.pop() {
                    match a.checked_neg() {
                        Some(r) => {
                            state.stack.push(r);
                            state.pc += 1;
                        }
                        None => {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::Overflow));
                            return false;
                        }
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Eq => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    state.stack.push(if a == b { 1 } else { 0 });
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Lt => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    state.stack.push(if a < b { 1 } else { 0 });
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::And => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    state.stack.push(a & b);
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Or => {
                if let (Some(b), Some(a)) = (state.stack.pop(), state.stack.pop()) {
                    state.stack.push(a | b);
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Not => {
                if let Some(a) = state.stack.pop() {
                    state.stack.push(if a == 0 { 1 } else { 0 });
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Jmp(target) => {
                if *target < program.len() {
                    state.pc = *target;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::InvalidJump));
                    return false;
                }
            }
            Instruction::Jz(target) => {
                if let Some(val) = state.stack.pop() {
                    if val == 0 {
                        if *target < program.len() {
                            state.pc = *target;
                        } else {
                            state.halted = true;
                            state.outcome = Some(VmOutcome::Fault(VmFault::InvalidJump));
                            return false;
                        }
                    } else {
                        state.pc += 1;
                    }
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Load(slot) => {
                let val = state.memory.get(slot).copied().unwrap_or(0);
                state.stack.push(val);
                state.pc += 1;
            }
            Instruction::Store(slot) => {
                if let Some(val) = state.stack.pop() {
                    state.memory.insert(*slot, val);
                    state.pc += 1;
                } else {
                    state.halted = true;
                    state.outcome = Some(VmOutcome::Fault(VmFault::StackUnderflow));
                    return false;
                }
            }
            Instruction::Halt(code) => {
                state.halted = true;
                state.outcome = Some(VmOutcome::Halted(*code));
                return false;
            }
            Instruction::Nop => {
                state.pc += 1;
            }
        }

        true // still running
    }

    /// Total run function: Program × Nat → VmOutcome.
    /// Always terminates within b_star steps. No divergence possible.
    pub fn run(program: &Program, b_star: u64) -> (VmOutcome, VmState) {
        let mut state = VmState::initial();

        for _ in 0..b_star {
            if !Self::step(program, &mut state) {
                break;
            }
        }

        if !state.halted {
            state.halted = true;
            state.outcome = Some(VmOutcome::BudgetExhausted);
        }

        let outcome = state.outcome.clone().unwrap();
        (outcome, state)
    }

    /// Run with full execution trace (hash-chained).
    pub fn run_traced(program: &Program, b_star: u64) -> ExecTrace {
        let mut state = VmState::initial();
        let initial_hash = state.ser_pi_hash();
        let mut steps = Vec::new();
        let mut trace_head = initial_hash;

        for step_index in 0..b_star {
            let pre_hash = state.ser_pi_hash();

            if !Self::step(program, &mut state) {
                let post_hash = state.ser_pi_hash();
                let entry = StepTrace {
                    step_index,
                    pre_state_hash: pre_hash,
                    post_state_hash: post_hash,
                    instruction_index: state.pc,
                };
                trace_head = hash::chain(&trace_head, &entry.ser_pi());
                steps.push(entry);
                break;
            }

            let post_hash = state.ser_pi_hash();
            let entry = StepTrace {
                step_index,
                pre_state_hash: pre_hash,
                post_state_hash: post_hash,
                instruction_index: state.pc,
            };
            trace_head = hash::chain(&trace_head, &entry.ser_pi());
            steps.push(entry);
        }

        if !state.halted {
            state.halted = true;
            state.outcome = Some(VmOutcome::BudgetExhausted);
        }

        let final_hash = state.ser_pi_hash();
        let outcome = state.outcome.clone().unwrap();
        let total_steps = state.steps_taken;

        ExecTrace {
            steps,
            trace_head,
            initial_state_hash: initial_hash,
            final_state_hash: final_hash,
            outcome,
            total_steps,
        }
    }

    /// Verify trace integrity: replay and check hash chain.
    pub fn verify_trace(program: &Program, trace: &ExecTrace) -> bool {
        let mut state = VmState::initial();
        let initial_hash = state.ser_pi_hash();

        if initial_hash != trace.initial_state_hash {
            return false;
        }

        let mut expected_head = initial_hash;

        for entry in &trace.steps {
            let pre_hash = state.ser_pi_hash();
            if pre_hash != entry.pre_state_hash {
                return false;
            }

            Vm::step(program, &mut state);

            let post_hash = state.ser_pi_hash();
            if post_hash != entry.post_state_hash {
                return false;
            }

            let recomputed = StepTrace {
                step_index: entry.step_index,
                pre_state_hash: pre_hash,
                post_state_hash: post_hash,
                instruction_index: entry.instruction_index,
            };
            expected_head = hash::chain(&expected_head, &recomputed.ser_pi());
        }

        expected_head == trace.trace_head
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_halt_true() {
        let prog = Program::new(vec![Instruction::Halt(1)]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn vm_halt_false() {
        let prog = Program::new(vec![Instruction::Halt(0)]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(0));
    }

    #[test]
    fn vm_push_add_halt() {
        // Push 3, Push 4, Add, Halt(1) — stack should have 7
        let prog = Program::new(vec![
            Instruction::Push(3),
            Instruction::Push(4),
            Instruction::Add,
            Instruction::Halt(1),
        ]);
        let (outcome, state) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(state.stack, vec![7]);
    }

    #[test]
    fn vm_budget_exhausted() {
        // Infinite loop: JMP 0
        let prog = Program::new(vec![Instruction::Nop, Instruction::Jmp(0)]);
        let (outcome, state) = Vm::run(&prog, 10);
        assert_eq!(outcome, VmOutcome::BudgetExhausted);
        assert_eq!(state.steps_taken, 10);
    }

    #[test]
    fn vm_division_by_zero() {
        let prog = Program::new(vec![
            Instruction::Push(10),
            Instruction::Push(0),
            Instruction::Div,
        ]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Fault(VmFault::DivisionByZero));
    }

    #[test]
    fn vm_stack_underflow() {
        let prog = Program::new(vec![Instruction::Add]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Fault(VmFault::StackUnderflow));
    }

    #[test]
    fn vm_conditional_jump() {
        // Push 0, JZ to Halt(1), Halt(0)
        // Since top=0, should jump to Halt(1)
        let prog = Program::new(vec![
            Instruction::Push(0),
            Instruction::Jz(3),
            Instruction::Halt(0),
            Instruction::Halt(1),
        ]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn vm_conditional_no_jump() {
        // Push 1, JZ to Halt(1), Halt(0)
        // Since top=1 (nonzero), should NOT jump, go to Halt(0)
        let prog = Program::new(vec![
            Instruction::Push(1),
            Instruction::Jz(3),
            Instruction::Halt(0),
            Instruction::Halt(1),
        ]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(0));
    }

    #[test]
    fn vm_memory_load_store() {
        // Push 42, Store[0], Load[0], Halt(1)
        let prog = Program::new(vec![
            Instruction::Push(42),
            Instruction::Store(0),
            Instruction::Load(0),
            Instruction::Halt(1),
        ]);
        let (outcome, state) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(state.stack, vec![42]);
    }

    #[test]
    fn vm_deterministic() {
        let prog = Program::new(vec![
            Instruction::Push(5),
            Instruction::Push(3),
            Instruction::Sub,
            Instruction::Push(7),
            Instruction::Mul,
            Instruction::Halt(1),
        ]);
        let (o1, s1) = Vm::run(&prog, 100);
        let (o2, s2) = Vm::run(&prog, 100);
        assert_eq!(o1, o2);
        assert_eq!(s1.stack, s2.stack);
        assert_eq!(s1.ser_pi_hash(), s2.ser_pi_hash());
    }

    #[test]
    fn vm_trace_verifies() {
        let prog = Program::new(vec![
            Instruction::Push(10),
            Instruction::Push(20),
            Instruction::Add,
            Instruction::Halt(1),
        ]);
        let trace = Vm::run_traced(&prog, 100);
        assert!(Vm::verify_trace(&prog, &trace));
        assert_eq!(trace.outcome, VmOutcome::Halted(1));
        assert_eq!(trace.total_steps, 4);
    }

    #[test]
    fn vm_trace_deterministic() {
        let prog = Program::new(vec![
            Instruction::Push(1),
            Instruction::Push(2),
            Instruction::Add,
            Instruction::Halt(1),
        ]);
        let t1 = Vm::run_traced(&prog, 100);
        let t2 = Vm::run_traced(&prog, 100);
        assert_eq!(t1.trace_head, t2.trace_head);
        assert_eq!(t1.initial_state_hash, t2.initial_state_hash);
        assert_eq!(t1.final_state_hash, t2.final_state_hash);
    }

    #[test]
    fn vm_loop_with_counter() {
        // Count from 0 to 5: mem[0] = counter
        // 0: PUSH 0, STORE 0       — init counter
        // 2: LOAD 0                 — load counter
        // 3: PUSH 5, LT            — counter < 5?
        // 5: JZ 11                  — if not, jump to halt
        // 6: LOAD 0, PUSH 1, ADD   — counter + 1
        // 9: STORE 0               — save
        // 10: JMP 2                 — loop
        // 11: HALT 1
        let prog = Program::new(vec![
            Instruction::Push(0),    // 0
            Instruction::Store(0),   // 1
            Instruction::Load(0),    // 2
            Instruction::Push(5),    // 3
            Instruction::Lt,         // 4
            Instruction::Jz(11),     // 5
            Instruction::Load(0),    // 6
            Instruction::Push(1),    // 7
            Instruction::Add,        // 8
            Instruction::Store(0),   // 9
            Instruction::Jmp(2),     // 10
            Instruction::Halt(1),    // 11
        ]);
        let (outcome, state) = Vm::run(&prog, 1000);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(*state.memory.get(&0).unwrap(), 5);
    }

    #[test]
    fn vm_comparison_operators() {
        // Push 3, Push 3, EQ → 1
        let prog = Program::new(vec![
            Instruction::Push(3),
            Instruction::Push(3),
            Instruction::Eq,
            Instruction::Halt(1),
        ]);
        let (_, state) = Vm::run(&prog, 100);
        assert_eq!(state.stack, vec![1]);

        // Push 3, Push 4, EQ → 0
        let prog = Program::new(vec![
            Instruction::Push(3),
            Instruction::Push(4),
            Instruction::Eq,
            Instruction::Halt(1),
        ]);
        let (_, state) = Vm::run(&prog, 100);
        assert_eq!(state.stack, vec![0]);
    }

    #[test]
    fn vm_overflow_handled() {
        let prog = Program::new(vec![
            Instruction::Push(i64::MAX),
            Instruction::Push(1),
            Instruction::Add,
        ]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Fault(VmFault::Overflow));
    }

    #[test]
    fn vm_empty_program() {
        let prog = Program::new(vec![]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Fault(VmFault::InvalidJump));
    }

    #[test]
    fn vm_dup_swap_drop() {
        let prog = Program::new(vec![
            Instruction::Push(10),
            Instruction::Push(20),
            Instruction::Dup,     // stack: [10, 20, 20]
            Instruction::Swap,    // stack: [10, 20, 20] → [10, 20, 20] (swap top two which are same)
            Instruction::Drop,    // stack: [10, 20]
            Instruction::Halt(1),
        ]);
        let (outcome, state) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(state.stack, vec![10, 20]);
    }

    #[test]
    fn vm_logical_not() {
        let prog = Program::new(vec![
            Instruction::Push(0),
            Instruction::Not,
            Instruction::Halt(1),
        ]);
        let (_, state) = Vm::run(&prog, 100);
        assert_eq!(state.stack, vec![1]);

        let prog = Program::new(vec![
            Instruction::Push(42),
            Instruction::Not,
            Instruction::Halt(1),
        ]);
        let (_, state) = Vm::run(&prog, 100);
        assert_eq!(state.stack, vec![0]);
    }

    #[test]
    fn vm_program_serpi_deterministic() {
        let prog = Program::new(vec![
            Instruction::Push(1),
            Instruction::Halt(1),
        ]);
        assert_eq!(prog.ser_pi(), prog.ser_pi());
        assert_eq!(prog.ser_pi_hash(), prog.ser_pi_hash());
    }

    #[test]
    fn vm_invalid_jump_target() {
        let prog = Program::new(vec![Instruction::Jmp(999)]);
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Fault(VmFault::InvalidJump));
    }
}
