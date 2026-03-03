use kernel_types::{Hash32, HASH_ZERO, SerPi, hash, Status};
use kernel_types::receipt::{Receipt, Payload, CompletionProof, SolveOutput};
use kernel_ledger::{Ledger, Event, EventKind};
use kernel_instruments::budget::Budget;
use kernel_contracts::contract::Contract;
use kernel_contracts::quotient::AnswerQuotient;
use crate::evaluator;
use crate::completion::{self, CompletionResult};

/// The deterministic solver (post-A1).
///
/// Implements the canonical refinement operator on (Q, L):
/// 1. COMPLETE(Q): derive B*(Q) or prove inadmissible.
/// 2. If admissible: exhaustive search up to B* → UNIQUE or UNSAT.
/// 3. If inadmissible: UNSAT(admissibility) with refutation witness.
///
/// Ω is deleted. Budgets are theorems, not parameters.
/// The dit gate is {UNIQUE, UNSAT} — no other output exists.
pub struct Solver {
    /// The ledger (shared across solves within a session).
    pub ledger: Ledger,
    /// The build hash (set externally).
    pub build_hash: Hash32,
    /// The kernel serialization hash (set externally).
    pub serpi_k_hash: Hash32,
    /// Running trace head for self-witnessing.
    pub trace_head: Hash32,
    /// Branchpoints collected during this solve.
    pub branchpoints: Vec<Hash32>,
}

impl Solver {
    pub fn new() -> Self {
        Solver {
            ledger: Ledger::new(),
            build_hash: HASH_ZERO,
            serpi_k_hash: HASH_ZERO,
            trace_head: HASH_ZERO,
            branchpoints: Vec::new(),
        }
    }

    /// Emit a trace event (for self-witnessing).
    fn emit_trace(&mut self, event_bytes: &[u8]) {
        self.trace_head = hash::chain(&self.trace_head, event_bytes);
    }

    /// Record a branchpoint.
    fn record_branchpoint(&mut self, label: &[u8]) {
        let bp = hash::H(label);
        self.branchpoints.push(bp);
        self.emit_trace(label);
    }

    /// The main solve function: SOLVE_K(Q).
    ///
    /// Returns UNIQUE or UNSAT with receipts. Deterministic.
    ///
    /// Stage 0: COMPLETE(Q) — derive B*(Q) or prove inadmissible.
    /// Stage 1: If completable, exhaustive search up to B* → UNIQUE/UNSAT.
    ///          If inadmissible, UNSAT(admissibility) with refutation.
    pub fn solve(&mut self, contract: &Contract) -> SolveOutput {
        // Reset per-solve state.
        self.branchpoints.clear();

        // Record contract compilation event.
        let compile_event = Event::new(
            EventKind::ContractCompiled,
            &contract.ser_pi(),
            vec![],
            1,
            0,
        );
        self.ledger.commit(compile_event.clone());
        self.emit_trace(&compile_event.ser_pi());

        self.record_branchpoint(b"contract_dispatch");

        // ─── STAGE 0: COMPLETE(Q) ───
        // Derive B*(Q) or prove the contract inadmissible.
        // This is the A1 axiom in action: budgets are theorems.
        self.record_branchpoint(b"completion_stage");

        let completion_result = completion::complete(contract);

        match completion_result {
            CompletionResult::Complete { b_star, proof_hash, sep_table_summary } => {
                // B*(Q) derived. The contract is admissible.
                // Record completion as ledger event.
                let completion_payload = format!("COMPLETE:B*={}:{}", b_star, sep_table_summary);
                let completion_event = Event::new(
                    EventKind::CertificateVerified,
                    completion_payload.as_bytes(),
                    vec![compile_event.hash()],
                    1,
                    0,
                );
                self.ledger.commit(completion_event.clone());
                self.emit_trace(&completion_event.ser_pi());

                let completion_proof = CompletionProof {
                    b_star: Some(b_star),
                    summary: sep_table_summary,
                    proof_hash,
                };

                // Proceed to exhaustive search with B* as the theorem-derived bound.
                self.solve_enumerable(contract, &compile_event, completion_proof)
            }

            CompletionResult::Inadmissible { refutation } => {
                // No B*(Q) derivable. The contract is NOT admissible.
                // Return UNSAT(admissibility) — "not a real question under A0+A1."
                self.record_branchpoint(b"inadmissible_refutation");

                let refutation_payload = format!(
                    "UNSAT(admissibility): {}",
                    refutation.reason
                );
                let refutation_event = Event::new(
                    EventKind::SolveComplete,
                    refutation_payload.as_bytes(),
                    vec![compile_event.hash()],
                    1,
                    0,
                );
                self.ledger.commit(refutation_event.clone());
                self.emit_trace(&refutation_event.ser_pi());

                let completion_proof = CompletionProof {
                    b_star: None,
                    summary: format!("{} Remedy: {}", refutation.reason, refutation.remedy),
                    proof_hash: refutation.proof_hash,
                };

                SolveOutput {
                    status: Status::Unsat,
                    payload: Payload {
                        answer: String::new(),
                        witness: refutation_payload.into_bytes(),
                    },
                    receipt: self.build_receipt(Some(completion_proof)),
                }
            }
        }
    }

    /// Solve an enumerable domain contract via exhaustive certificate collapse.
    ///
    /// Called only after COMPLETE(Q) has derived B*(Q) — the domain is
    /// guaranteed to be finitely enumerable within the completion bound.
    fn solve_enumerable(
        &mut self,
        contract: &Contract,
        compile_event: &Event,
        completion_proof: CompletionProof,
    ) -> SolveOutput {
        let domain = contract.answer_alphabet.enumerate();

        self.record_branchpoint(b"certificate_collapse_start");

        // Evaluate all candidates against the contract's eval spec.
        let (satisfying, _unsatisfying) = evaluator::evaluate_all(&contract.eval, &domain);

        // Record the evaluation as a ledger event.
        let eval_payload = {
            let mut buf = Vec::new();
            buf.extend_from_slice(b"eval_result:");
            buf.extend_from_slice(&(satisfying.len() as u64).ser_pi());
            buf
        };
        let eval_event = Event::new(
            EventKind::CertificateVerified,
            &eval_payload,
            vec![compile_event.hash()],
            domain.len() as u64,
            (domain.len() - satisfying.len()) as u64,
        );
        self.ledger.commit(eval_event.clone());
        self.emit_trace(&eval_event.ser_pi());

        self.record_branchpoint(b"certificate_collapse_end");

        // Determine status from the answer quotient.
        if satisfying.is_empty() {
            // UNSAT: no candidates satisfy the contract.
            let unsat_event = Event::new(
                EventKind::SolveComplete,
                b"UNSAT",
                vec![eval_event.hash()],
                0,
                0,
            );
            self.ledger.commit(unsat_event.clone());
            self.emit_trace(&unsat_event.ser_pi());

            SolveOutput {
                status: Status::Unsat,
                payload: Payload {
                    answer: String::new(),
                    witness: b"exhaustive_search_no_satisfying_candidates".to_vec(),
                },
                receipt: self.build_receipt(Some(completion_proof)),
            }
        } else if satisfying.len() == 1 {
            // UNIQUE: exactly one candidate satisfies.
            let answer = satisfying[0].clone();
            let answer_hex = hash::hex(&hash::H(&answer));

            let unique_event = Event::new(
                EventKind::SolveComplete,
                b"UNIQUE",
                vec![eval_event.hash()],
                0,
                0,
            );
            self.ledger.commit(unique_event.clone());
            self.emit_trace(&unique_event.ser_pi());

            SolveOutput {
                status: Status::Unique,
                payload: Payload {
                    answer: answer_hex,
                    witness: answer,
                },
                receipt: self.build_receipt(Some(completion_proof)),
            }
        } else {
            // Multiple candidates survive — apply tiebreak.
            let answer = match &contract.tiebreak {
                kernel_contracts::contract::Tiebreak::LexMin => {
                    let mut sorted = satisfying.clone();
                    sorted.sort();
                    sorted[0].clone()
                }
                kernel_contracts::contract::Tiebreak::FirstFound => {
                    satisfying[0].clone()
                }
            };
            let answer_hex = hash::hex(&hash::H(&answer));
            let tag = match &contract.tiebreak {
                kernel_contracts::contract::Tiebreak::LexMin => b"UNIQUE_VIA_TIEBREAK".as_slice(),
                kernel_contracts::contract::Tiebreak::FirstFound => b"UNIQUE_VIA_FIRST_FOUND".as_slice(),
            };

            let unique_event = Event::new(
                EventKind::SolveComplete,
                tag,
                vec![eval_event.hash()],
                0,
                (satisfying.len() - 1) as u64,
            );
            self.ledger.commit(unique_event.clone());
            self.emit_trace(&unique_event.ser_pi());

            SolveOutput {
                status: Status::Unique,
                payload: Payload {
                    answer: answer_hex,
                    witness: answer,
                },
                receipt: self.build_receipt(Some(completion_proof)),
            }
        }
    }

    /// Build the receipt for the current solve.
    fn build_receipt(&self, completion: Option<CompletionProof>) -> Receipt {
        Receipt {
            serpi_k_hash: self.serpi_k_hash,
            build_hash: self.build_hash,
            trace_head: self.trace_head,
            branchpoints: self.branchpoints.clone(),
            ledger_head: self.ledger.head(),
            completion,
        }
    }

    /// Replay a receipt: verify that the given receipt matches
    /// what the solver would produce for the given contract.
    pub fn replay_verify(&mut self, contract: &Contract, expected: &SolveOutput) -> bool {
        let actual = self.solve(contract);

        // Compare status.
        if actual.status != expected.status {
            return false;
        }

        // Compare payload answer.
        if actual.payload.answer != expected.payload.answer {
            return false;
        }

        // Compare trace head (determinism check).
        if actual.receipt.trace_head != expected.receipt.trace_head {
            return false;
        }

        true
    }
}

impl Default for Solver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_contracts::compiler::compile_contract;

    #[test]
    fn solve_simple_sat() {
        let json = r#"{
            "type": "bool_cnf",
            "description": "x1 OR x2",
            "num_vars": 2,
            "clauses": [[1, 2]]
        }"#;
        let contract = compile_contract(json).unwrap();
        let mut solver = Solver::new();
        let output = solver.solve(&contract);
        assert_eq!(output.status, Status::Unique);
    }

    #[test]
    fn solve_unsat() {
        // x AND NOT x
        let json = r#"{
            "type": "bool_cnf",
            "description": "x AND NOT x (unsatisfiable)",
            "num_vars": 1,
            "clauses": [[1], [-1]]
        }"#;
        let contract = compile_contract(json).unwrap();
        let mut solver = Solver::new();
        let output = solver.solve(&contract);
        assert_eq!(output.status, Status::Unsat);
    }

    #[test]
    fn solve_deterministic() {
        let json = r#"{
            "type": "bool_cnf",
            "description": "determinism test",
            "num_vars": 3,
            "clauses": [[1, 2, 3], [-1, 2], [-2, 3]]
        }"#;
        let contract = compile_contract(json).unwrap();

        let mut solver1 = Solver::new();
        let out1 = solver1.solve(&contract);

        let mut solver2 = Solver::new();
        let out2 = solver2.solve(&contract);

        assert_eq!(out1.status, out2.status);
        assert_eq!(out1.payload.answer, out2.payload.answer);
        assert_eq!(out1.receipt.trace_head, out2.receipt.trace_head);
    }

    #[test]
    fn solve_arith() {
        // Find x such that 2x + 3 = 7, i.e. x = 2
        let json = r#"{
            "type": "arith_find",
            "description": "2x + 3 = 7",
            "coefficients": [3, 2],
            "target": 7,
            "lo": -10,
            "hi": 10
        }"#;
        let contract = compile_contract(json).unwrap();
        let mut solver = Solver::new();
        let output = solver.solve(&contract);
        assert_eq!(output.status, Status::Unique);
    }

    #[test]
    fn replay_matches() {
        let json = r#"{
            "type": "bool_cnf",
            "description": "replay test",
            "num_vars": 2,
            "clauses": [[1], [2]]
        }"#;
        let contract = compile_contract(json).unwrap();

        let mut solver1 = Solver::new();
        let output = solver1.solve(&contract);

        let mut solver2 = Solver::new();
        assert!(solver2.replay_verify(&contract, &output));
    }

    #[test]
    fn formal_proof_is_unsat_admissibility() {
        let json = r#"{
            "type": "formal_proof",
            "description": "P vs NP test",
            "statement": "P = NP or P ≠ NP",
            "formal_system": "Lean4"
        }"#;
        let contract = compile_contract(json).unwrap();
        let mut solver = Solver::new();
        let output = solver.solve(&contract);

        // Post-A1: formal proofs return UNSAT(admissibility), not Ω.
        assert_eq!(output.status, Status::Unsat);

        // The completion proof should indicate inadmissibility.
        let completion = output.receipt.completion.as_ref().unwrap();
        assert!(completion.b_star.is_none());
        assert!(completion.summary.contains("INADMISSIBLE"));
    }
}
