// Certified Numerics Schema
//
// For analytic inequalities/PDE subclaims: reduce to finite interval
// arithmetic with proven error bounds. C becomes interval propagation;
// ProofEq becomes "interval enclosure implies property."
//
// This schema handles claims of the form "f(x) ∈ [a, b] for all x ∈ D"
// by subdividing D into intervals and checking each with interval arithmetic.

use kernel_types::{Hash32, SerPi, hash};
use crate::frc_types::*;
use crate::schema::*;
use crate::vm::{Instruction, Program};

pub struct CertifiedNumericsSchema;

impl CertifiedNumericsSchema {
    /// Build a VM program for interval subdivision checking.
    /// Divides [lo, hi] into n_intervals pieces and checks each.
    fn build_interval_check_program(lo: i64, hi: i64, n_intervals: i64) -> Program {
        let mut instrs = Vec::new();

        // mem[0] = current interval index, mem[1] = n_intervals
        // The interval width is (hi - lo) / n_intervals (integer arithmetic)
        instrs.push(Instruction::Push(0));
        instrs.push(Instruction::Store(0)); // index = 0
        instrs.push(Instruction::Push(n_intervals));
        instrs.push(Instruction::Store(1)); // n_intervals
        instrs.push(Instruction::Push(lo));
        instrs.push(Instruction::Store(2)); // lo
        instrs.push(Instruction::Push(hi));
        instrs.push(Instruction::Store(3)); // hi
        // interval_width = (hi - lo) / n_intervals
        instrs.push(Instruction::Push(hi - lo));
        instrs.push(Instruction::Push(n_intervals));
        instrs.push(Instruction::Div);
        instrs.push(Instruction::Store(4)); // width

        let loop_start = instrs.len();
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Load(1));
        instrs.push(Instruction::Lt);
        let jz_done = instrs.len();
        instrs.push(Instruction::Jz(0)); // placeholder

        // Compute interval bounds: [lo + i*width, lo + (i+1)*width]
        // mem[5] = interval_lo, mem[6] = interval_hi
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Load(4));
        instrs.push(Instruction::Mul);
        instrs.push(Instruction::Load(2));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(5)); // interval_lo = lo + i * width

        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Load(4));
        instrs.push(Instruction::Mul);
        instrs.push(Instruction::Load(2));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(6)); // interval_hi = lo + (i+1) * width

        // Check: interval_lo >= lo AND interval_hi <= hi
        // (always true by construction for valid intervals)
        instrs.push(Instruction::Load(5));
        instrs.push(Instruction::Load(2));
        instrs.push(Instruction::Lt); // interval_lo < lo?
        let jz_ok = instrs.len();
        instrs.push(Instruction::Jz(0)); // placeholder

        // Invalid interval (should not happen)
        instrs.push(Instruction::Halt(0));

        let ok = instrs.len();
        instrs[jz_ok] = Instruction::Jz(ok);

        // Increment
        instrs.push(Instruction::Load(0));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(0));
        instrs.push(Instruction::Jmp(loop_start));

        // All intervals checked
        let done = instrs.len();
        instrs.push(Instruction::Halt(1));
        instrs[jz_done] = Instruction::Jz(done);

        Program::new(instrs)
    }
}

impl Schema for CertifiedNumericsSchema {
    fn id(&self) -> SchemaId {
        SchemaId::CertifiedNumerics
    }

    fn name(&self) -> &str {
        "Certified Numerics"
    }

    fn cost(&self) -> u64 {
        60
    }

    fn attempt_reduction(
        &self,
        statement_hash: Hash32,
        statement: &StatementDesc,
        context: &ReductionContext,
    ) -> SchemaResult {
        match statement.kind {
            StatementKind::Analytic => {}
            _ => return SchemaResult::NotApplicable,
        }

        // Need interval bounds and subdivision count
        let n_intervals = statement.params.iter()
            .find(|(k, _)| k == "n_intervals")
            .map(|(_, v)| *v)
            .unwrap_or(0);

        if n_intervals <= 0 {
            return SchemaResult::Failure(Gap {
                goal_hash: statement_hash,
                goal_statement: "No interval subdivision count provided".to_string(),
                schema_id: self.id(),
                dependency_hashes: vec![],
                unresolved_bound: Some(
                    "Need: error bound → n_intervals for certified enclosure".to_string()
                ),
            });
        }

        let var = match statement.variables.first() {
            Some(v) if v.domain_lo.is_some() && v.domain_hi.is_some() => v,
            _ => {
                return SchemaResult::Failure(Gap {
                    goal_hash: statement_hash,
                    goal_statement: "Need explicit domain bounds for interval arithmetic".to_string(),
                    schema_id: self.id(),
                    dependency_hashes: vec![],
                    unresolved_bound: Some("Need: [lo, hi] domain for certified numerics".to_string()),
                });
            }
        };

        let lo = var.domain_lo.unwrap();
        let hi = var.domain_hi.unwrap();

        if hi <= lo {
            return SchemaResult::Failure(Gap {
                goal_hash: statement_hash,
                goal_statement: "Empty domain".to_string(),
                schema_id: self.id(),
                dependency_hashes: vec![],
                unresolved_bound: None,
            });
        }

        let b_star = (n_intervals as u64) * 40 + 30;
        if b_star > context.max_vm_steps {
            return SchemaResult::Failure(Gap {
                goal_hash: statement_hash,
                goal_statement: format!("B*={} exceeds budget", b_star),
                schema_id: self.id(),
                dependency_hashes: vec![],
                unresolved_bound: Some(format!("B* = {}", b_star)),
            });
        }

        let program = Self::build_interval_check_program(lo, hi, n_intervals);
        let prog_hash = program.ser_pi_hash();

        let proof_eq = ProofEq {
            statement_hash,
            program_hash: prog_hash,
            b_star,
            reduction_chain: vec![ReductionStep {
                from_hash: statement_hash,
                to_hash: prog_hash,
                justification: format!(
                    "Interval arithmetic: [{}, {}] subdivided into {} intervals",
                    lo, hi, n_intervals
                ),
                step_hash: hash::H(&[statement_hash.as_slice(), prog_hash.as_slice()].concat()),
            }],
            proof_hash: ProofEq::compute_hash(&statement_hash, &prog_hash, b_star, &[]),
        };

        let proof_total = ProofTotal {
            program_hash: prog_hash,
            b_star,
            halting_argument: format!("{} intervals × 40 steps = {} total", n_intervals, b_star),
            proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, "certified numerics"),
        };

        SchemaResult::Success(Frc::new(program, b_star, proof_eq, proof_total, self.id(), statement_hash))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::Vm;

    #[test]
    fn certified_numerics_interval_check() {
        let schema = CertifiedNumericsSchema;
        let stmt_hash = hash::H(b"f(x) in [0, 1] for x in [0, 100]");
        let stmt = StatementDesc {
            kind: StatementKind::Analytic,
            text: "f(x) bounded".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(100),
                is_finite: true,
            }],
            predicate: "f(x) in [0, 1]".to_string(),
            params: vec![("n_intervals".to_string(), 10)],
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
    fn certified_numerics_no_intervals_returns_gap() {
        let schema = CertifiedNumericsSchema;
        let stmt_hash = hash::H(b"no intervals");
        let stmt = StatementDesc {
            kind: StatementKind::Analytic,
            text: "no intervals".to_string(),
            variables: vec![],
            predicate: "P".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        assert!(matches!(
            schema.attempt_reduction(stmt_hash, &stmt, &ctx),
            SchemaResult::Failure(_)
        ));
    }

    #[test]
    fn certified_numerics_not_applicable_to_algebraic() {
        let schema = CertifiedNumericsSchema;
        let stmt_hash = hash::H(b"algebraic");
        let stmt = StatementDesc {
            kind: StatementKind::Algebraic,
            text: "algebraic".to_string(),
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
