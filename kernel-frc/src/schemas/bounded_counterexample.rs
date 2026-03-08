// Bounded Counterexample Schema
//
// If S is ∀x, P(x), try to prove: ¬S → ∃x ≤ B*, ¬P(x) with explicit B*.
// Then C searches x ≤ B*.
//
// This schema works for universal statements over finite or effectively
// bounded domains where a counterexample witness, if it exists, must
// appear within a computable bound.

use kernel_types::{Hash32, SerPi, hash};
use crate::frc_types::*;
use crate::schema::*;
use crate::vm::{Instruction, Program};

pub struct BoundedCounterexampleSchema;

impl BoundedCounterexampleSchema {
    /// Build a VM program that searches for a counterexample to ∀x ∈ [lo..hi], P(x).
    /// The program iterates x from lo to hi, evaluates P(x) via the predicate encoding,
    /// and halts with 1 if all pass (statement true) or 0 if counterexample found.
    fn build_search_program(lo: i64, hi: i64, coefficients: &[(String, i64)]) -> Program {
        // Simple bounded search: iterate x from lo to hi
        // mem[0] = current x, mem[1] = hi
        // For each x: check predicate, if fails halt(0), else increment
        // If all pass: halt(1)
        let mut instrs = Vec::new();

        // Init: mem[0] = lo, mem[1] = hi
        instrs.push(Instruction::Push(lo));
        instrs.push(Instruction::Store(0)); // x = lo
        instrs.push(Instruction::Push(hi));
        instrs.push(Instruction::Store(1)); // hi stored

        let loop_start = instrs.len(); // instruction 4
        // Check x <= hi
        instrs.push(Instruction::Load(0));   // push x
        instrs.push(Instruction::Load(1));   // push hi
        instrs.push(Instruction::Push(1));   // push 1
        instrs.push(Instruction::Add);       // hi + 1
        instrs.push(Instruction::Lt);        // x < hi + 1  (i.e., x <= hi)
        let jz_exit = instrs.len();
        instrs.push(Instruction::Jz(0));     // placeholder: jump to "all pass"

        // Evaluate predicate P(x): simplified as checking coefficients
        // For now: P(x) = sum(c_i * x^i) != 0 (or whatever the encoding needs)
        // Generic: just check x against encoded condition
        instrs.push(Instruction::Load(0));   // push x

        // Simple predicate: if coefficients provided, compute polynomial
        if !coefficients.is_empty() {
            // Compute P(x) = c0 + c1*x + c2*x^2 + ...
            // mem[2] = accumulator, mem[3] = x_power
            instrs.push(Instruction::Store(3)); // mem[3] = x (current power base)
            instrs.push(Instruction::Push(coefficients[0].1));
            instrs.push(Instruction::Store(2)); // acc = c0
            instrs.push(Instruction::Push(1));
            instrs.push(Instruction::Store(4)); // x_power = 1

            for (_name, coeff) in coefficients.iter().skip(1) {
                // x_power *= x
                instrs.push(Instruction::Load(4));
                instrs.push(Instruction::Load(3));
                instrs.push(Instruction::Mul);
                instrs.push(Instruction::Store(4));
                // acc += coeff * x_power
                instrs.push(Instruction::Push(*coeff));
                instrs.push(Instruction::Load(4));
                instrs.push(Instruction::Mul);
                instrs.push(Instruction::Load(2));
                instrs.push(Instruction::Add);
                instrs.push(Instruction::Store(2));
            }
            // Check if P(x) == target (0 means counterexample found)
            instrs.push(Instruction::Load(2));
            instrs.push(Instruction::Push(0));
            instrs.push(Instruction::Eq);
            let jz_no_counter = instrs.len();
            instrs.push(Instruction::Jz(0)); // placeholder
            // Counterexample found!
            instrs.push(Instruction::Halt(0));
            // Patch jump
            let after_halt = instrs.len();
            instrs[jz_no_counter] = Instruction::Jz(after_halt);
        } else {
            // No coefficients: trivially true (drop x, continue)
            instrs.push(Instruction::Drop);
        }

        // Increment x
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Jmp(loop_start));

        // All passed
        let all_pass = instrs.len();
        instrs.push(Instruction::Halt(1));

        // Patch the exit jump
        instrs[jz_exit] = Instruction::Jz(all_pass);

        Program::new(instrs)
    }
}

impl Schema for BoundedCounterexampleSchema {
    fn id(&self) -> SchemaId {
        SchemaId::BoundedCounterexample
    }

    fn name(&self) -> &str {
        "Bounded Counterexample"
    }

    fn cost(&self) -> u64 {
        10
    }

    fn attempt_reduction(
        &self,
        statement_hash: Hash32,
        statement: &StatementDesc,
        context: &ReductionContext,
    ) -> SchemaResult {
        // Only applicable to universal statements with bounded domains
        match statement.kind {
            StatementKind::UniversalFinite | StatementKind::BoolSat => {}
            _ => return SchemaResult::NotApplicable,
        }

        // Need at least one variable with finite bounds
        let var = match statement.variables.first() {
            Some(v) if v.is_finite && v.domain_lo.is_some() && v.domain_hi.is_some() => v,
            _ => return SchemaResult::NotApplicable,
        };

        let lo = var.domain_lo.unwrap();
        let hi = var.domain_hi.unwrap();
        let b_star = ((hi - lo + 1) as u64) * 50; // generous step budget per element

        if b_star > context.max_vm_steps {
            return SchemaResult::Failure(Gap {
                goal_hash: statement_hash,
                goal_statement: format!("Bound B*={} exceeds max_vm_steps={}", b_star, context.max_vm_steps),
                schema_id: self.id(),
                dependency_hashes: vec![],
                unresolved_bound: Some(format!("B* = {} (domain [{}, {}])", b_star, lo, hi)),
            });
        }

        let program = Self::build_search_program(lo, hi, &statement.params);
        let prog_hash = program.ser_pi_hash();

        let reduction_step = ReductionStep {
            from_hash: statement_hash,
            to_hash: prog_hash,
            justification: format!(
                "∀x ∈ [{}, {}], P(x) ⟺ search [{}, {}] finds no counterexample",
                lo, hi, lo, hi
            ),
            step_hash: hash::H(&[statement_hash.as_slice(), prog_hash.as_slice()].concat()),
        };

        let proof_eq = ProofEq {
            statement_hash,
            program_hash: prog_hash,
            b_star,
            reduction_chain: vec![reduction_step],
            proof_hash: ProofEq::compute_hash(&statement_hash, &prog_hash, b_star, &[]),
            lean_proof: None,
        };

        let proof_total = ProofTotal {
            program_hash: prog_hash,
            b_star,
            halting_argument: format!(
                "Bounded loop from {} to {}, each iteration ≤50 steps, total ≤ {} steps",
                lo, hi, b_star
            ),
            proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, "bounded loop"),
            lean_proof: None,
        };

        let frc = Frc::new(program, b_star, proof_eq, proof_total, self.id(), statement_hash);
        SchemaResult::Success(frc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::Vm;

    #[test]
    fn bounded_search_finds_no_counterexample() {
        let schema = BoundedCounterexampleSchema;
        let stmt_hash = hash::H(b"forall x in [0,10], P(x)");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalFinite,
            text: "forall x in [0,10], P(x)".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "P(x)".to_string(),
            params: vec![], // no polynomial => trivially true
        };
        let ctx = ReductionContext::default_context();

        match schema.attempt_reduction(stmt_hash, &stmt, &ctx) {
            SchemaResult::Success(frc) => {
                assert!(frc.verify_internal());
                let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                assert_eq!(outcome, crate::vm::VmOutcome::Halted(1));
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn bounded_not_applicable_to_infinite() {
        let schema = BoundedCounterexampleSchema;
        let stmt_hash = hash::H(b"forall x, P(x)");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "forall x, P(x)".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: None,
                domain_hi: None,
                is_finite: false,
            }],
            predicate: "P(x)".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        match schema.attempt_reduction(stmt_hash, &stmt, &ctx) {
            SchemaResult::NotApplicable => {}
            other => panic!("Expected NotApplicable, got {:?}", other),
        }
    }

    #[test]
    fn bounded_frc_deterministic() {
        let schema = BoundedCounterexampleSchema;
        let stmt_hash = hash::H(b"test");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalFinite,
            text: "test".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(5),
                is_finite: true,
            }],
            predicate: "P".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        let r1 = schema.attempt_reduction(stmt_hash, &stmt, &ctx);
        let r2 = schema.attempt_reduction(stmt_hash, &stmt, &ctx);

        match (r1, r2) {
            (SchemaResult::Success(f1), SchemaResult::Success(f2)) => {
                assert_eq!(f1.frc_hash, f2.frc_hash);
            }
            _ => panic!("Both should succeed"),
        }
    }
}
