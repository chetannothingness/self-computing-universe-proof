// OPP Runner — Open Problem Package solver.
//
// The universal runner for solving open problems:
//   kernel opp-solve path/to/OPP --out out_dir
//
// For each problem S:
//   1. Parse OPP (Statement + Context + TargetClass + AllowedPrimitives)
//   2. Run FRC search
//   3. If FRC found: execute VM, verify, emit UNIQUE/UNSAT result
//   4. If no FRC: emit INVALID with minimal missing-lemma frontier

use kernel_types::{SerPi, hash};
use kernel_ledger::{Ledger, Event, EventKind};
use crate::frc_types::*;
use crate::frc_search::FrcSearch;
use crate::schema::{StatementDesc, StatementKind, VariableDesc, ReductionContext};
use crate::vm::Vm;

/// Result of solving an OPP.
#[derive(Debug, Clone)]
pub enum OppResult {
    /// Statement is true: FRC found and VM returned 1
    Proof {
        frc: Frc,
        receipt: FrcReceipt,
    },
    /// Statement is false: FRC found and VM returned 0
    Disproof {
        frc: Frc,
        receipt: FrcReceipt,
    },
    /// Statement is not admissible in current schema closure
    Invalid {
        frontier: FrontierWitness,
    },
}

/// The OPP runner.
pub struct OppRunner {
    pub engine: FrcSearch,
}

impl OppRunner {
    pub fn new() -> Self {
        Self {
            engine: FrcSearch::new(),
        }
    }

    /// Solve an Open Problem Package.
    pub fn solve(
        &mut self,
        opp: &OpenProblemPackage,
        ledger: &mut Ledger,
    ) -> OppResult {
        ledger.commit(Event::new(
            EventKind::OppSolveStart,
            &opp.ser_pi(),
            vec![ledger.head()],
            1,
            0,
        ));

        // Convert OPP to internal statement description
        let statement = Self::parse_opp_to_statement(opp);
        let context = ReductionContext {
            available_lemmas: self.engine.motif_library.motif_hashes(),
            max_vm_steps: opp.allowed_primitives.max_vm_steps,
            max_memory_slots: opp.allowed_primitives.max_memory_slots,
        };

        // Search for FRC
        match self.engine.search(opp.opp_hash, &statement, &context, ledger) {
            FrcResult::Found(frc) => {
                // Execute the FRC
                let trace = Vm::run_traced(&frc.program, frc.b_star);

                // Verify trace
                let trace_verified = Vm::verify_trace(&frc.program, &trace);

                let merkle = hash::merkle_root(&[
                    opp.opp_hash,
                    frc.frc_hash,
                    trace.trace_head,
                ]);

                let receipt = FrcReceipt::new(
                    frc.frc_hash,
                    match &trace.outcome {
                        crate::vm::VmOutcome::Halted(code) => *code,
                        _ => 255,
                    },
                    trace.trace_head,
                    merkle,
                    opp.opp_hash,
                    trace_verified,
                );

                match trace.outcome {
                    crate::vm::VmOutcome::Halted(1) => {
                        OppResult::Proof { frc, receipt }
                    }
                    crate::vm::VmOutcome::Halted(0) => {
                        OppResult::Disproof { frc, receipt }
                    }
                    _ => {
                        // Execution error — treat as invalid
                        let frontier = FrontierWitness::new(
                            opp.opp_hash,
                            vec![frc.schema_id.clone()],
                            vec![Gap {
                                goal_hash: opp.opp_hash,
                                goal_statement: format!("VM execution failed: {:?}", trace.outcome),
                                schema_id: frc.schema_id,
                                dependency_hashes: vec![],
                                unresolved_bound: None,
                            }],
                            None,
                        );
                        OppResult::Invalid { frontier }
                    }
                }
            }
            FrcResult::Invalid(frontier) => {
                OppResult::Invalid { frontier }
            }
        }
    }

    /// Parse OPP statement field into a StatementDesc.
    fn parse_opp_to_statement(opp: &OpenProblemPackage) -> StatementDesc {
        let text = &opp.statement;

        // Determine statement kind from text patterns
        let kind = if text.starts_with("forall") || text.starts_with("∀") {
            if text.contains("[") && text.contains("]") {
                StatementKind::UniversalFinite
            } else {
                StatementKind::UniversalInfinite
            }
        } else if text.starts_with("exists") || text.starts_with("∃") {
            if text.contains("[") && text.contains("]") {
                StatementKind::ExistentialFinite
            } else {
                StatementKind::ExistentialInfinite
            }
        } else if text.contains("SAT") || text.contains("cnf") {
            StatementKind::BoolSat
        } else if text.contains("find") || text.contains("solve") {
            StatementKind::ArithFind
        } else if text.contains("polynomial") || text.contains("root") {
            StatementKind::Algebraic
        } else if text.contains("bound") || text.contains("interval") {
            StatementKind::Analytic
        } else {
            StatementKind::UniversalInfinite
        };

        // Try to extract variable domain from "[lo,hi]" pattern in the text
        let variables = if text.contains("[") && text.contains("]") {
            // Attempt to parse "[lo,hi]"
            if let Some(start) = text.find('[') {
                if let Some(end) = text.find(']') {
                    let range_str = &text[start+1..end];
                    let parts: Vec<&str> = range_str.split(',').collect();
                    if parts.len() == 2 {
                        let lo = parts[0].trim().parse::<i64>().ok();
                        let hi = parts[1].trim().parse::<i64>().ok();
                        if lo.is_some() && hi.is_some() {
                            vec![VariableDesc {
                                name: "x".to_string(),
                                domain_lo: lo,
                                domain_hi: hi,
                                is_finite: true,
                            }]
                        } else {
                            vec![]
                        }
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        StatementDesc {
            kind,
            text: text.clone(),
            variables,
            predicate: text.clone(),
            params: vec![],
        }
    }
}

impl Default for OppRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solvable_opp() -> OpenProblemPackage {
        OpenProblemPackage::new(
            "exists x in [0,10]: x > 0".to_string(),
            "".to_string(),
            TargetClass {
                allowed_schemas: vec![SchemaId::FiniteSearch],
                grammar_description: "first-order".to_string(),
            },
            AllowedPrimitives {
                max_vm_steps: 100_000,
                max_memory_slots: 256,
                cost_model: "unit".to_string(),
            },
            ExpectedOutput::Either,
        )
    }

    fn make_unsolvable_opp() -> OpenProblemPackage {
        OpenProblemPackage::new(
            "forall x: P(x) is undecidable without modulus".to_string(),
            "".to_string(),
            TargetClass {
                allowed_schemas: vec![SchemaId::FiniteSearch],
                grammar_description: "higher-order".to_string(),
            },
            AllowedPrimitives {
                max_vm_steps: 100_000,
                max_memory_slots: 256,
                cost_model: "unit".to_string(),
            },
            ExpectedOutput::Either,
        )
    }

    #[test]
    fn opp_solve_finds_proof() {
        let mut runner = OppRunner::new();
        let mut ledger = Ledger::new();
        let opp = make_solvable_opp();

        match runner.solve(&opp, &mut ledger) {
            OppResult::Proof { frc, receipt } => {
                assert!(frc.verify_internal());
                assert!(receipt.verified);
                assert_eq!(receipt.execution_outcome, 1);
            }
            other => panic!("Expected Proof, got {:?}", other),
        }
    }

    #[test]
    fn opp_solve_returns_invalid() {
        let mut runner = OppRunner::new();
        let mut ledger = Ledger::new();
        let opp = make_unsolvable_opp();

        match runner.solve(&opp, &mut ledger) {
            OppResult::Invalid { frontier } => {
                assert_eq!(frontier.statement_hash, opp.opp_hash);
                assert!(!frontier.schemas_tried.is_empty());
            }
            other => panic!("Expected Invalid, got {:?}", other),
        }
    }

    #[test]
    fn opp_solve_deterministic() {
        let mut r1 = OppRunner::new();
        let mut r2 = OppRunner::new();
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let opp = make_solvable_opp();

        let res1 = r1.solve(&opp, &mut l1);
        let res2 = r2.solve(&opp, &mut l2);

        match (res1, res2) {
            (OppResult::Proof { receipt: r1, .. }, OppResult::Proof { receipt: r2, .. }) => {
                assert_eq!(r1.receipt_hash, r2.receipt_hash);
            }
            _ => panic!("Both should be Proof"),
        }
    }

    #[test]
    fn opp_emits_ledger_events() {
        let mut runner = OppRunner::new();
        let mut ledger = Ledger::new();
        let opp = make_solvable_opp();

        runner.solve(&opp, &mut ledger);
        assert!(ledger.len() >= 2); // at least OppSolveStart + FrcComplete
    }
}
