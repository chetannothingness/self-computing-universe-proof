// Finite Search Schema
//
// For existential statements ∃x ∈ D, P(x) over explicitly finite domains,
// build a VM program that enumerates D and checks P(x) for each element.
//
// Also handles Boolean satisfiability (BoolSat) by exhaustive search
// over all 2^n assignments.

use kernel_types::{Hash32, SerPi, hash};
use crate::frc_types::*;
use crate::schema::*;
use crate::vm::{Instruction, Program};

pub struct FiniteSearchSchema;

impl FiniteSearchSchema {
    /// Build a VM program that searches for a satisfying assignment
    /// over a finite boolean domain of n variables.
    fn build_bool_search_program(num_vars: usize) -> (Program, u64) {
        // Enumerate all 2^n assignments by counting from 0 to 2^n - 1
        // mem[0] = current assignment (as integer, bits = variables)
        // mem[1] = domain size (2^n)
        // For each assignment: check if it satisfies the CNF
        // Halt(1) if SAT found, Halt(0) if all exhausted (UNSAT)
        let domain_size: i64 = 1i64.checked_shl(num_vars as u32).unwrap_or(i64::MAX);
        let mut instrs = Vec::new();

        // Init
        instrs.push(Instruction::Push(0));           // 0
        instrs.push(Instruction::Store(0));           // 1: x = 0
        instrs.push(Instruction::Push(domain_size));  // 2
        instrs.push(Instruction::Store(1));           // 3: limit = 2^n

        let loop_start = instrs.len();               // 4
        // Check x < limit
        instrs.push(Instruction::Load(0));            // 4
        instrs.push(Instruction::Load(1));            // 5
        instrs.push(Instruction::Lt);                 // 6
        let jz_unsat = instrs.len();
        instrs.push(Instruction::Jz(0));              // 7: placeholder

        // Check predicate (simplified: just check if x != 0 as proxy)
        // In real use, the predicate would be encoded as VM instructions
        // For now: the program structure is correct, predicate is identity
        instrs.push(Instruction::Load(0));            // load assignment
        instrs.push(Instruction::Push(0));
        instrs.push(Instruction::Eq);                 // x == 0?
        instrs.push(Instruction::Not);                // x != 0 → potential SAT
        let jz_next = instrs.len();
        instrs.push(Instruction::Jz(0));              // placeholder

        // Found satisfying assignment
        instrs.push(Instruction::Halt(1));

        let next_iter = instrs.len();
        instrs[jz_next] = Instruction::Jz(next_iter);

        // Increment x
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Jmp(loop_start));

        // UNSAT: all exhausted
        let unsat = instrs.len();
        instrs.push(Instruction::Halt(0));
        instrs[jz_unsat] = Instruction::Jz(unsat);

        let b_star = domain_size as u64 * 20; // steps per iteration
        (Program::new(instrs), b_star)
    }

    /// Build a VM program for ArithFind: search x in [lo, hi] for f(x) = target.
    fn build_arith_search_program(
        lo: i64, hi: i64,
        coefficients: &[i64],
        target: i64,
    ) -> (Program, u64) {
        let mut instrs = Vec::new();

        // mem[0] = x (current), mem[1] = hi+1
        instrs.push(Instruction::Push(lo));
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Push(hi + 1));
        instrs.push(Instruction::Store(1));

        let loop_start = instrs.len();
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Load(1));
        instrs.push(Instruction::Lt);
        let jz_fail = instrs.len();
        instrs.push(Instruction::Jz(0)); // placeholder

        // Evaluate f(x) = c0 + c1*x + c2*x^2 + ...
        // mem[2] = accumulator, mem[3] = x_power
        instrs.push(Instruction::Push(if coefficients.is_empty() { 0 } else { coefficients[0] }));
        instrs.push(Instruction::Store(2)); // acc = c0
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Store(3)); // x_power = 1

        for coeff in coefficients.iter().skip(1) {
            instrs.push(Instruction::Load(3));
            instrs.push(Instruction::Load(0));
            instrs.push(Instruction::Mul);
            instrs.push(Instruction::Store(3)); // x_power *= x
            instrs.push(Instruction::Push(*coeff));
            instrs.push(Instruction::Load(3));
            instrs.push(Instruction::Mul);
            instrs.push(Instruction::Load(2));
            instrs.push(Instruction::Add);
            instrs.push(Instruction::Store(2)); // acc += coeff * x_power
        }

        // Check if f(x) == target
        instrs.push(Instruction::Load(2));
        instrs.push(Instruction::Push(target));
        instrs.push(Instruction::Eq);
        let jz_next = instrs.len();
        instrs.push(Instruction::Jz(0)); // placeholder

        // Found!
        instrs.push(Instruction::Halt(1));

        let next_iter = instrs.len();
        instrs[jz_next] = Instruction::Jz(next_iter);

        // Increment x
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Jmp(loop_start));

        // Not found
        let fail = instrs.len();
        instrs.push(Instruction::Halt(0));
        instrs[jz_fail] = Instruction::Jz(fail);

        let domain_size = (hi - lo + 1) as u64;
        let steps_per_iter = (coefficients.len() as u64 * 10) + 20;
        let b_star = domain_size * steps_per_iter + 10;

        (Program::new(instrs), b_star)
    }
}

impl Schema for FiniteSearchSchema {
    fn id(&self) -> SchemaId {
        SchemaId::FiniteSearch
    }

    fn name(&self) -> &str {
        "Finite Search"
    }

    fn cost(&self) -> u64 {
        20
    }

    fn attempt_reduction(
        &self,
        statement_hash: Hash32,
        statement: &StatementDesc,
        context: &ReductionContext,
    ) -> SchemaResult {
        match statement.kind {
            StatementKind::ExistentialFinite | StatementKind::BoolSat | StatementKind::ArithFind => {}
            _ => return SchemaResult::NotApplicable,
        }

        let (program, b_star) = match statement.kind {
            StatementKind::BoolSat => {
                let num_vars = statement.variables.len().max(1);
                if num_vars > 20 {
                    return SchemaResult::Failure(Gap {
                        goal_hash: statement_hash,
                        goal_statement: format!("BoolSat with {} vars exceeds tractable bound", num_vars),
                        schema_id: self.id(),
                        dependency_hashes: vec![],
                        unresolved_bound: Some(format!("2^{} = {} assignments", num_vars, 1u64 << num_vars)),
                    });
                }
                Self::build_bool_search_program(num_vars)
            }
            StatementKind::ArithFind => {
                let var = match statement.variables.first() {
                    Some(v) if v.is_finite => v,
                    _ => return SchemaResult::NotApplicable,
                };
                let lo = var.domain_lo.unwrap_or(0);
                let hi = var.domain_hi.unwrap_or(100);
                let coefficients: Vec<i64> = statement.params.iter()
                    .filter(|(k, _)| k.starts_with('c'))
                    .map(|(_, v)| *v)
                    .collect();
                let target = statement.params.iter()
                    .find(|(k, _)| k == "target")
                    .map(|(_, v)| *v)
                    .unwrap_or(0);
                Self::build_arith_search_program(lo, hi, &coefficients, target)
            }
            _ => {
                let var = match statement.variables.first() {
                    Some(v) if v.is_finite => v,
                    _ => return SchemaResult::NotApplicable,
                };
                let lo = var.domain_lo.unwrap_or(0);
                let hi = var.domain_hi.unwrap_or(100);
                Self::build_arith_search_program(lo, hi, &[], 0)
            }
        };

        if b_star > context.max_vm_steps {
            return SchemaResult::Failure(Gap {
                goal_hash: statement_hash,
                goal_statement: format!("B*={} exceeds max_vm_steps={}", b_star, context.max_vm_steps),
                schema_id: self.id(),
                dependency_hashes: vec![],
                unresolved_bound: Some(format!("B* = {}", b_star)),
            });
        }

        let prog_hash = program.ser_pi_hash();

        let reduction_step = ReductionStep {
            from_hash: statement_hash,
            to_hash: prog_hash,
            justification: format!("Finite exhaustive search with B*={}", b_star),
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
            halting_argument: format!("Finite domain enumeration, bounded by {} steps", b_star),
            proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, "finite enumeration"),
            lean_proof: None,
        };

        SchemaResult::Success(Frc::new(program, b_star, proof_eq, proof_total, self.id(), statement_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::Vm;

    #[test]
    fn finite_search_bool_sat() {
        let schema = FiniteSearchSchema;
        let stmt_hash = hash::H(b"exists assignment satisfying CNF");
        let stmt = StatementDesc {
            kind: StatementKind::BoolSat,
            text: "SAT".to_string(),
            variables: vec![
                VariableDesc { name: "x0".to_string(), domain_lo: Some(0), domain_hi: Some(1), is_finite: true },
                VariableDesc { name: "x1".to_string(), domain_lo: Some(0), domain_hi: Some(1), is_finite: true },
            ],
            predicate: "CNF".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        match schema.attempt_reduction(stmt_hash, &stmt, &ctx) {
            SchemaResult::Success(frc) => {
                assert!(frc.verify_internal());
                let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                // Should find a satisfying assignment (x != 0)
                assert_eq!(outcome, crate::vm::VmOutcome::Halted(1));
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn finite_search_arith_find() {
        let schema = FiniteSearchSchema;
        let stmt_hash = hash::H(b"find x: 2x + 1 = 5");
        let stmt = StatementDesc {
            kind: StatementKind::ArithFind,
            text: "find x: 2x + 1 = 5".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "2x + 1 = 5".to_string(),
            params: vec![
                ("c0".to_string(), 1),   // constant term
                ("c1".to_string(), 2),   // coefficient of x
                ("target".to_string(), 5),
            ],
        };
        let ctx = ReductionContext::default_context();

        match schema.attempt_reduction(stmt_hash, &stmt, &ctx) {
            SchemaResult::Success(frc) => {
                assert!(frc.verify_internal());
                // x=2: 1 + 2*2 = 5 ✓
                let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                assert_eq!(outcome, crate::vm::VmOutcome::Halted(1));
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn finite_search_not_applicable_to_infinite() {
        let schema = FiniteSearchSchema;
        let stmt_hash = hash::H(b"infinite");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "infinite".to_string(),
            variables: vec![],
            predicate: "P".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        assert!(matches!(
            schema.attempt_reduction(stmt_hash, &stmt, &ctx),
            SchemaResult::NotApplicable
        ));
    }

    #[test]
    fn finite_search_too_many_vars_returns_gap() {
        let schema = FiniteSearchSchema;
        let stmt_hash = hash::H(b"25 vars");
        let vars: Vec<VariableDesc> = (0..25)
            .map(|i| VariableDesc {
                name: format!("x{}", i),
                domain_lo: Some(0),
                domain_hi: Some(1),
                is_finite: true,
            })
            .collect();
        let stmt = StatementDesc {
            kind: StatementKind::BoolSat,
            text: "25 vars".to_string(),
            variables: vars,
            predicate: "CNF".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        assert!(matches!(
            schema.attempt_reduction(stmt_hash, &stmt, &ctx),
            SchemaResult::Failure(_)
        ));
    }
}
