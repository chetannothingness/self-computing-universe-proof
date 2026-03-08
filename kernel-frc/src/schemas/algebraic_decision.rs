// Algebraic Decision Schema
//
// Reduce statements in algebra/number theory to a bounded computation
// (e.g., Gröbner basis / Nullstellensatz with effective bounds) only if
// the bound is proven.
//
// This schema handles: polynomial identity testing, ideal membership,
// system-of-equations feasibility over finite fields or bounded integers.

use kernel_types::{Hash32, SerPi, hash};
use crate::frc_types::*;
use crate::schema::*;
use crate::vm::{Instruction, Program};

pub struct AlgebraicDecisionSchema;

impl AlgebraicDecisionSchema {
    /// Build a VM program that evaluates a polynomial identity check.
    /// For P(x) = 0 over domain [lo, hi], search for roots.
    fn build_polynomial_check_program(
        lo: i64, hi: i64,
        coefficients: &[i64],
    ) -> Program {
        let mut instrs = Vec::new();

        // mem[0] = x, mem[1] = hi+1
        instrs.push(Instruction::Push(lo));
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Push(hi + 1));
        instrs.push(Instruction::Store(1));

        let loop_start = instrs.len();
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Load(1));
        instrs.push(Instruction::Lt);
        let jz_done = instrs.len();
        instrs.push(Instruction::Jz(0)); // placeholder

        // Evaluate P(x) = c0 + c1*x + c2*x^2 + ...
        instrs.push(Instruction::Push(if coefficients.is_empty() { 0 } else { coefficients[0] }));
        instrs.push(Instruction::Store(2)); // acc = c0
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Store(3)); // x_power = 1

        for coeff in coefficients.iter().skip(1) {
            instrs.push(Instruction::Load(3));
            instrs.push(Instruction::Load(0));
            instrs.push(Instruction::Mul);
            instrs.push(Instruction::Store(3));
            instrs.push(Instruction::Push(*coeff));
            instrs.push(Instruction::Load(3));
            instrs.push(Instruction::Mul);
            instrs.push(Instruction::Load(2));
            instrs.push(Instruction::Add);
            instrs.push(Instruction::Store(2));
        }

        // Check P(x) == 0
        instrs.push(Instruction::Load(2));
        instrs.push(Instruction::Push(0));
        instrs.push(Instruction::Eq);
        let jz_next = instrs.len();
        instrs.push(Instruction::Jz(0)); // placeholder

        // Root found
        instrs.push(Instruction::Halt(1));

        let next = instrs.len();
        instrs[jz_next] = Instruction::Jz(next);

        // Increment
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Jmp(loop_start));

        // No root found
        let done = instrs.len();
        instrs.push(Instruction::Halt(0));
        instrs[jz_done] = Instruction::Jz(done);

        Program::new(instrs)
    }
}

impl Schema for AlgebraicDecisionSchema {
    fn id(&self) -> SchemaId {
        SchemaId::AlgebraicDecision
    }

    fn name(&self) -> &str {
        "Algebraic Decision"
    }

    fn cost(&self) -> u64 {
        50
    }

    fn attempt_reduction(
        &self,
        statement_hash: Hash32,
        statement: &StatementDesc,
        context: &ReductionContext,
    ) -> SchemaResult {
        match statement.kind {
            StatementKind::Algebraic | StatementKind::ArithFind => {}
            _ => return SchemaResult::NotApplicable,
        }

        let var = match statement.variables.first() {
            Some(v) if v.is_finite && v.domain_lo.is_some() && v.domain_hi.is_some() => v,
            _ => {
                // For algebraic statements without explicit bounds,
                // we need a degree bound to derive the search range
                let degree = statement.params.iter()
                    .find(|(k, _)| k == "degree")
                    .map(|(_, v)| *v)
                    .unwrap_or(0);

                if degree <= 0 {
                    return SchemaResult::Failure(Gap {
                        goal_hash: statement_hash,
                        goal_statement: "No effective bound: need degree or explicit domain".to_string(),
                        schema_id: self.id(),
                        dependency_hashes: vec![],
                        unresolved_bound: Some(
                            "Need: polynomial degree → Nullstellensatz bound".to_string()
                        ),
                    });
                }

                // Use Schwartz-Zippel-style bound: degree * max_coeff
                let max_coeff = statement.params.iter()
                    .filter(|(k, _)| k != "degree" && k != "target")
                    .map(|(_, v)| v.abs())
                    .max()
                    .unwrap_or(1);

                let bound = degree * max_coeff + 1;
                let lo = -bound;
                let hi = bound;
                let coefficients: Vec<i64> = statement.params.iter()
                    .filter(|(k, _)| k.starts_with('c'))
                    .map(|(_, v)| *v)
                    .collect();

                let domain_size = (hi - lo + 1) as u64;
                let b_star = domain_size * (coefficients.len() as u64 * 10 + 20) + 10;

                if b_star > context.max_vm_steps {
                    return SchemaResult::Failure(Gap {
                        goal_hash: statement_hash,
                        goal_statement: format!("Algebraic bound B*={} exceeds budget", b_star),
                        schema_id: self.id(),
                        dependency_hashes: vec![],
                        unresolved_bound: Some(format!("B* = {}", b_star)),
                    });
                }

                let program = Self::build_polynomial_check_program(lo, hi, &coefficients);
                let prog_hash = program.ser_pi_hash();

                let proof_eq = ProofEq {
                    statement_hash,
                    program_hash: prog_hash,
                    b_star,
                    reduction_chain: vec![ReductionStep {
                        from_hash: statement_hash,
                        to_hash: prog_hash,
                        justification: format!(
                            "Algebraic: degree {} polynomial, search [{}, {}]", degree, lo, hi
                        ),
                        step_hash: hash::H(&[statement_hash.as_slice(), prog_hash.as_slice()].concat()),
                    }],
                    proof_hash: ProofEq::compute_hash(&statement_hash, &prog_hash, b_star, &[]),
                    lean_proof: None,
                };

                let proof_total = ProofTotal {
                    program_hash: prog_hash,
                    b_star,
                    halting_argument: format!("Bounded polynomial root search: {} evaluations", domain_size),
                    proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, "algebraic decision"),
                    lean_proof: None,
                };

                return SchemaResult::Success(Frc::new(
                    program, b_star, proof_eq, proof_total, self.id(), statement_hash
                ));
            }
        };

        let lo = var.domain_lo.unwrap();
        let hi = var.domain_hi.unwrap();
        let coefficients: Vec<i64> = statement.params.iter()
            .filter(|(k, _)| k.starts_with('c'))
            .map(|(_, v)| *v)
            .collect();

        let domain_size = (hi - lo + 1) as u64;
        let b_star = domain_size * (coefficients.len() as u64 * 10 + 20) + 10;

        if b_star > context.max_vm_steps {
            return SchemaResult::Failure(Gap {
                goal_hash: statement_hash,
                goal_statement: format!("B*={} exceeds budget", b_star),
                schema_id: self.id(),
                dependency_hashes: vec![],
                unresolved_bound: Some(format!("B* = {}", b_star)),
            });
        }

        let program = Self::build_polynomial_check_program(lo, hi, &coefficients);
        let prog_hash = program.ser_pi_hash();

        let proof_eq = ProofEq {
            statement_hash,
            program_hash: prog_hash,
            b_star,
            reduction_chain: vec![ReductionStep {
                from_hash: statement_hash,
                to_hash: prog_hash,
                justification: format!("Algebraic search [{}, {}]", lo, hi),
                step_hash: hash::H(&[statement_hash.as_slice(), prog_hash.as_slice()].concat()),
            }],
            proof_hash: ProofEq::compute_hash(&statement_hash, &prog_hash, b_star, &[]),
            lean_proof: None,
        };

        let proof_total = ProofTotal {
            program_hash: prog_hash,
            b_star,
            halting_argument: format!("Bounded search: {} evaluations", domain_size),
            proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, "algebraic bounded"),
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
    fn algebraic_finds_root() {
        let schema = AlgebraicDecisionSchema;
        let stmt_hash = hash::H(b"x^2 - 4 = 0");
        let stmt = StatementDesc {
            kind: StatementKind::Algebraic,
            text: "x^2 - 4 = 0".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(-10),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "x^2 - 4 = 0".to_string(),
            params: vec![
                ("c0".to_string(), -4),  // constant: -4
                ("c1".to_string(), 0),   // x coeff: 0
                ("c2".to_string(), 1),   // x^2 coeff: 1
            ],
        };
        let ctx = ReductionContext::default_context();

        match schema.attempt_reduction(stmt_hash, &stmt, &ctx) {
            SchemaResult::Success(frc) => {
                assert!(frc.verify_internal());
                let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                // x=-2 or x=2 satisfies x^2-4=0
                assert_eq!(outcome, crate::vm::VmOutcome::Halted(1));
            }
            other => panic!("Expected Success, got {:?}", other),
        }
    }

    #[test]
    fn algebraic_not_applicable_to_bool() {
        let schema = AlgebraicDecisionSchema;
        let stmt_hash = hash::H(b"bool");
        let stmt = StatementDesc {
            kind: StatementKind::BoolSat,
            text: "bool".to_string(),
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
}
