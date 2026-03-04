// Label-based assembler for building complex VM programs.
//
// The predicate compiler handles simple expression trees, but cannot express
// nested loops with inline subroutines (e.g., primality testing inside a
// Goldbach verification loop). This assembler provides:
//   - Symbolic labels with deferred jump resolution
//   - All VM instructions as builder methods
//   - Forward and backward jump support
//   - Build() that resolves all labels and produces a Program

use std::collections::BTreeMap;
use crate::vm::{Instruction, Program};

/// An entry in the assembler: either an instruction or a label definition.
enum AsmEntry {
    Instr(Instruction),
    Label(String),
    /// Jump to a symbolic label (resolved at build time).
    Jmp(String),
    /// Conditional jump to a symbolic label (resolved at build time).
    Jz(String),
}

/// Label-based program assembler.
pub struct Asm {
    entries: Vec<AsmEntry>,
    labels: BTreeMap<String, Option<usize>>,
}

impl Asm {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            labels: BTreeMap::new(),
        }
    }

    /// Define a label at the current instruction position.
    pub fn label(&mut self, name: &str) {
        self.labels.insert(name.to_string(), None);
        self.entries.push(AsmEntry::Label(name.to_string()));
    }

    // --- Instruction emitters ---

    pub fn push(&mut self, val: i64) {
        self.entries.push(AsmEntry::Instr(Instruction::Push(val)));
    }

    pub fn load(&mut self, slot: usize) {
        self.entries.push(AsmEntry::Instr(Instruction::Load(slot)));
    }

    pub fn store(&mut self, slot: usize) {
        self.entries.push(AsmEntry::Instr(Instruction::Store(slot)));
    }

    pub fn add(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Add));
    }

    pub fn sub(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Sub));
    }

    pub fn mul(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Mul));
    }

    pub fn div(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Div));
    }

    pub fn mod_(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Mod));
    }

    pub fn neg(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Neg));
    }

    pub fn eq(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Eq));
    }

    pub fn lt(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Lt));
    }

    pub fn and(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::And));
    }

    pub fn or(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Or));
    }

    pub fn not(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Not));
    }

    pub fn dup(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Dup));
    }

    pub fn drop(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Drop));
    }

    pub fn swap(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Swap));
    }

    pub fn nop(&mut self) {
        self.entries.push(AsmEntry::Instr(Instruction::Nop));
    }

    /// Symbolic unconditional jump.
    pub fn jmp(&mut self, label: &str) {
        self.entries.push(AsmEntry::Jmp(label.to_string()));
    }

    /// Symbolic conditional jump (pop top, jump if zero).
    pub fn jz(&mut self, label: &str) {
        self.entries.push(AsmEntry::Jz(label.to_string()));
    }

    /// Halt with exit code.
    pub fn halt(&mut self, code: u8) {
        self.entries.push(AsmEntry::Instr(Instruction::Halt(code)));
    }

    /// Number of actual instructions (excluding labels).
    pub fn len(&self) -> usize {
        self.entries.iter().filter(|e| !matches!(e, AsmEntry::Label(_))).count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Resolve all labels and produce a Program.
    pub fn build(self) -> Result<Program, String> {
        // First pass: assign instruction indices, record label positions.
        let mut label_positions: BTreeMap<String, usize> = BTreeMap::new();
        let mut instr_index = 0usize;

        for entry in &self.entries {
            match entry {
                AsmEntry::Label(name) => {
                    label_positions.insert(name.clone(), instr_index);
                }
                _ => {
                    instr_index += 1;
                }
            }
        }

        // Second pass: emit instructions, resolving symbolic jumps.
        let mut instructions = Vec::new();

        for entry in &self.entries {
            match entry {
                AsmEntry::Instr(instr) => {
                    instructions.push(instr.clone());
                }
                AsmEntry::Label(_) => {
                    // Labels don't emit instructions.
                }
                AsmEntry::Jmp(label) => {
                    let target = label_positions.get(label)
                        .ok_or_else(|| format!("undefined label: {}", label))?;
                    instructions.push(Instruction::Jmp(*target));
                }
                AsmEntry::Jz(label) => {
                    let target = label_positions.get(label)
                        .ok_or_else(|| format!("undefined label: {}", label))?;
                    instructions.push(Instruction::Jz(*target));
                }
            }
        }

        Ok(Program::new(instructions))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{Vm, VmOutcome};

    #[test]
    fn label_resolution_forward_jump() {
        let mut asm = Asm::new();
        asm.jmp("end");
        asm.halt(0); // should be skipped
        asm.label("end");
        asm.halt(1);

        let prog = asm.build().unwrap();
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn label_resolution_backward_jump() {
        // Loop: push 0 store[0], label "top", load[0] push 3 lt, jz "done",
        //        load[0] push 1 add store[0], jmp "top", label "done", halt(1)
        let mut asm = Asm::new();
        asm.push(0);
        asm.store(0);
        asm.label("top");
        asm.load(0);
        asm.push(3);
        asm.lt();
        asm.jz("done");
        asm.load(0);
        asm.push(1);
        asm.add();
        asm.store(0);
        asm.jmp("top");
        asm.label("done");
        asm.halt(1);

        let prog = asm.build().unwrap();
        let (outcome, state) = Vm::run(&prog, 1000);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(*state.memory.get(&0).unwrap(), 3);
    }

    #[test]
    fn undefined_label_error() {
        let mut asm = Asm::new();
        asm.jmp("nonexistent");
        asm.halt(1);

        let result = asm.build();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("undefined label"));
    }

    #[test]
    fn simple_loop_program() {
        // Sum 1..5: mem[0]=sum, mem[1]=i
        let mut asm = Asm::new();
        asm.push(0);
        asm.store(0); // sum = 0
        asm.push(1);
        asm.store(1); // i = 1
        asm.label("loop");
        asm.load(1);
        asm.push(6);
        asm.lt();           // i < 6?
        asm.jz("done");
        asm.load(0);
        asm.load(1);
        asm.add();
        asm.store(0);       // sum += i
        asm.load(1);
        asm.push(1);
        asm.add();
        asm.store(1);       // i += 1
        asm.jmp("loop");
        asm.label("done");
        asm.halt(1);

        let prog = asm.build().unwrap();
        let (outcome, state) = Vm::run(&prog, 1000);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(*state.memory.get(&0).unwrap(), 15); // 1+2+3+4+5
    }

    #[test]
    fn len_excludes_labels() {
        let mut asm = Asm::new();
        asm.label("start");
        asm.push(1);
        asm.label("mid");
        asm.push(2);
        asm.label("end");
        asm.halt(1);

        assert_eq!(asm.len(), 3); // push, push, halt — labels not counted
    }

    #[test]
    fn conditional_jump_jz() {
        // If top==0, jump to "zero", else fall through to halt(0)
        let mut asm = Asm::new();
        asm.push(0);
        asm.jz("zero");
        asm.halt(0);
        asm.label("zero");
        asm.halt(1);

        let prog = asm.build().unwrap();
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn conditional_no_jump() {
        let mut asm = Asm::new();
        asm.push(1); // nonzero → don't jump
        asm.jz("skip");
        asm.halt(1);
        asm.label("skip");
        asm.halt(0);

        let prog = asm.build().unwrap();
        let (outcome, _) = Vm::run(&prog, 100);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }
}
