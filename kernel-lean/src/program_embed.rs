//! Convert kernel_frc::vm::Program → syntactically valid Lean4 definition.

use kernel_frc::vm::{Instruction, Program};

/// Convert a single instruction to its Lean4 representation.
fn instruction_to_lean(instr: &Instruction) -> String {
    match instr {
        Instruction::Push(val) => format!("Instruction.push ({})", val),
        Instruction::Dup => "Instruction.dup".to_string(),
        Instruction::Drop => "Instruction.drop".to_string(),
        Instruction::Swap => "Instruction.swap".to_string(),
        Instruction::Add => "Instruction.add".to_string(),
        Instruction::Sub => "Instruction.sub".to_string(),
        Instruction::Mul => "Instruction.mul".to_string(),
        Instruction::Div => "Instruction.div".to_string(),
        Instruction::Mod => "Instruction.mod".to_string(),
        Instruction::Neg => "Instruction.neg".to_string(),
        Instruction::Eq => "Instruction.eq".to_string(),
        Instruction::Lt => "Instruction.lt".to_string(),
        Instruction::And => "Instruction.and".to_string(),
        Instruction::Or => "Instruction.or".to_string(),
        Instruction::Not => "Instruction.not".to_string(),
        Instruction::Jmp(target) => format!("Instruction.jmp {}", target),
        Instruction::Jz(target) => format!("Instruction.jz {}", target),
        Instruction::Load(slot) => format!("Instruction.load {}", slot),
        Instruction::Store(slot) => format!("Instruction.store {}", slot),
        Instruction::Halt(code) => format!("Instruction.halt {}", code),
        Instruction::Nop => "Instruction.nop".to_string(),
    }
}

/// Convert a Program to a Lean4 definition.
///
/// Generates:
/// ```lean
/// import KernelVm
///
/// open KernelVm
///
/// def <name> : Program :=
///   { instructions := [
///       Instruction.push 42,
///       Instruction.halt 1
///     ] }
/// ```
pub fn embed_program(program: &Program, name: &str) -> String {
    let mut lines = Vec::new();
    lines.push("import KernelVm".to_string());
    lines.push(String::new());
    lines.push("open KernelVm".to_string());
    lines.push(String::new());
    lines.push(format!("def {} : Program :=", name));
    lines.push("  { instructions := [".to_string());

    let instrs: Vec<String> = program
        .instructions
        .iter()
        .map(|i| format!("      {}", instruction_to_lean(i)))
        .collect();

    for (i, instr_line) in instrs.iter().enumerate() {
        if i < instrs.len() - 1 {
            lines.push(format!("{},", instr_line));
        } else {
            lines.push(instr_line.clone());
        }
    }

    lines.push("    ] }".to_string());
    lines.push(String::new());

    lines.join("\n")
}

/// Embed the program's B* bound as a Lean4 definition.
pub fn embed_bstar(b_star: u64, name: &str) -> String {
    format!(
        "import KernelVm\n\ndef {} : Nat := {}\n",
        name, b_star
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embed_simple_program() {
        let prog = Program::new(vec![
            Instruction::Push(42),
            Instruction::Halt(1),
        ]);
        let lean = embed_program(&prog, "testProg");
        assert!(lean.contains("def testProg : Program"));
        assert!(lean.contains("Instruction.push (42)"));
        assert!(lean.contains("Instruction.halt 1"));
    }

    #[test]
    fn embed_program_with_jumps() {
        let prog = Program::new(vec![
            Instruction::Push(0),
            Instruction::Jz(3),
            Instruction::Halt(0),
            Instruction::Halt(1),
        ]);
        let lean = embed_program(&prog, "jumpProg");
        assert!(lean.contains("Instruction.jz 3"));
        assert!(lean.contains("Instruction.halt 0"));
    }

    #[test]
    fn embed_bstar_value() {
        let lean = embed_bstar(10000, "goldbachBstar");
        assert!(lean.contains("def goldbachBstar : Nat := 10000"));
    }

    #[test]
    fn embed_negative_push() {
        let prog = Program::new(vec![
            Instruction::Push(-1),
            Instruction::Halt(1),
        ]);
        let lean = embed_program(&prog, "negProg");
        assert!(lean.contains("Instruction.push (-1)"));
    }

    #[test]
    fn embed_all_instructions() {
        let prog = Program::new(vec![
            Instruction::Push(1), Instruction::Dup, Instruction::Drop,
            Instruction::Swap, Instruction::Add, Instruction::Sub,
            Instruction::Mul, Instruction::Div, Instruction::Mod,
            Instruction::Neg, Instruction::Eq, Instruction::Lt,
            Instruction::And, Instruction::Or, Instruction::Not,
            Instruction::Jmp(0), Instruction::Jz(0),
            Instruction::Load(0), Instruction::Store(0),
            Instruction::Halt(1), Instruction::Nop,
        ]);
        let lean = embed_program(&prog, "allInstr");
        assert!(lean.contains("Instruction.dup"));
        assert!(lean.contains("Instruction.swap"));
        assert!(lean.contains("Instruction.neg"));
        assert!(lean.contains("Instruction.nop"));
    }
}
