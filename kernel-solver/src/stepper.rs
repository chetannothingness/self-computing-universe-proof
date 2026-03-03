use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::status::Status;
use kernel_types::receipt::{Receipt, Payload, SolveOutput};
use kernel_ledger::Ledger;
use kernel_instruments::state::State;
use kernel_instruments::budget::Budget;
use kernel_instruments::enumerator::DeltaEnumerator;
use kernel_contracts::contract::Contract;
use kernel_contracts::quotient::AnswerQuotient;
use crate::evaluator;

/// Step result from one iteration of the solver.
#[derive(Debug)]
pub struct StepResult {
    /// Index of the instrument that was applied.
    pub instrument_index: usize,
    /// Cost of this step.
    pub cost: u64,
    /// Shrink achieved by this step.
    pub shrink: u64,
    /// Whether solving is now complete.
    pub done: bool,
}

/// SolverStepper: drives the solver step-by-step.
/// Breaks the circular dependency between kernel-self and kernel-solver
/// by exposing an incremental interface.
pub struct SolverStepper {
    pub ledger: Ledger,
    pub state: State,
    pub quotient: AnswerQuotient,
    pub step_count: u64,
    pub done: bool,
    pub trace_head: Hash32,
    pub branchpoints: Vec<Hash32>,
    pub total_cost: u64,
    status: Option<Status>,
    answer: Option<Vec<u8>>,
}

impl SolverStepper {
    /// Initialize a stepper from a contract.
    pub fn new(contract: &Contract) -> Self {
        let domain = contract.answer_alphabet.enumerate();
        let quotient = AnswerQuotient::from_domain(domain);
        let mut state = State::new();

        // Set up evaluation state.
        let (satisfying, _) = evaluator::evaluate_all(&contract.eval, &quotient.survivors().iter().cloned().collect::<Vec<_>>());

        // Record satisfying candidates in state.
        for candidate in &satisfying {
            let key = format!("candidate:{}", hash::hex(&hash::H(candidate)));
            state.set(key.into_bytes(), b"SAT".to_vec());
        }

        // Build initial quotient from evaluation.
        let quotient = AnswerQuotient::from_domain(satisfying);

        let done = quotient.is_unique() || quotient.is_unsat();
        let status = if quotient.is_unique() {
            Some(Status::Unique)
        } else if quotient.is_unsat() {
            Some(Status::Unsat)
        } else {
            None
        };
        let answer = quotient.unique_answer().cloned();

        SolverStepper {
            ledger: Ledger::new(),
            state,
            quotient,
            step_count: 0,
            done,
            trace_head: HASH_ZERO,
            branchpoints: Vec::new(),
            total_cost: 0,
            status,
            answer,
        }
    }

    /// Current state hash.
    pub fn state_hash(&self) -> Hash32 {
        hash::H(&self.state.ser_pi())
    }

    /// Current quotient hash.
    pub fn quotient_hash(&self) -> Hash32 {
        self.quotient.quotient_hash()
    }

    /// Number of remaining survivors.
    pub fn survivors(&self) -> usize {
        self.quotient.size()
    }

    /// Apply one instrument step. Returns the step result.
    pub fn step(&mut self, instrument_index: usize, enumerator: &DeltaEnumerator, budget: &Budget) -> Option<StepResult> {
        if self.done {
            return None;
        }

        let instrument = enumerator.get(instrument_index)?;

        if !budget.can_afford(instrument.cost(), self.total_cost) {
            self.done = true;
            return None;
        }

        let result = instrument.apply(&self.state, budget);

        // Update state.
        self.state.apply_delta(&result.delta);

        // Update trace.
        for event in &result.events {
            let event_bytes = event.ser_pi();
            self.trace_head = hash::chain(&self.trace_head, &event_bytes);
            self.ledger.commit(event.clone());
        }

        self.total_cost += result.cost;
        self.step_count += 1;

        let step_result = StepResult {
            instrument_index,
            cost: result.cost,
            shrink: result.outcome.shrink,
            done: self.done,
        };

        // Check if solved.
        if self.quotient.is_unique() || self.quotient.is_unsat() {
            self.done = true;
            self.status = if self.quotient.is_unique() {
                Some(Status::Unique)
            } else {
                Some(Status::Unsat)
            };
            self.answer = self.quotient.unique_answer().cloned();
        }

        Some(step_result)
    }

    /// Finalize and produce the solve output.
    pub fn finalize(&self) -> SolveOutput {
        let status = self.status.clone().unwrap_or(Status::Unsat);
        let answer = self.answer.clone().unwrap_or_default();
        let answer_hex = if answer.is_empty() {
            String::new()
        } else {
            hash::hex(&hash::H(&answer))
        };

        SolveOutput {
            status,
            payload: Payload {
                answer: answer_hex,
                witness: answer,
            },
            receipt: Receipt {
                serpi_k_hash: HASH_ZERO,
                build_hash: HASH_ZERO,
                trace_head: self.trace_head,
                branchpoints: self.branchpoints.clone(),
                ledger_head: self.ledger.head(),
                completion: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_contracts::compiler::compile_contract;

    #[test]
    fn stepper_initializes() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "stepper test",
            "num_vars": 2,
            "clauses": [[1], [2]]
        }"#).unwrap();
        let stepper = SolverStepper::new(&contract);
        // With forced assignment x1=T, x2=T, should resolve immediately.
        assert!(stepper.done);
        assert_eq!(stepper.survivors(), 1);
    }

    #[test]
    fn stepper_finalize_produces_output() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "stepper finalize",
            "num_vars": 1,
            "clauses": [[1]]
        }"#).unwrap();
        let stepper = SolverStepper::new(&contract);
        let output = stepper.finalize();
        assert_eq!(output.status, Status::Unique);
    }

    #[test]
    fn stepper_unsat() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "stepper unsat",
            "num_vars": 1,
            "clauses": [[1], [-1]]
        }"#).unwrap();
        let stepper = SolverStepper::new(&contract);
        assert!(stepper.done);
        let output = stepper.finalize();
        assert_eq!(output.status, Status::Unsat);
    }
}
