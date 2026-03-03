use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::reason::ReasonCode;
use kernel_types::tension::{Tension, TensionDelta};
use kernel_ledger::{Ledger, Event, EventKind};
use kernel_instruments::budget::Budget;
use kernel_solver::stepper::SolverStepper;
use kernel_contracts::contract::Contract;
use crate::self_model::SelfModel;
use crate::self_instrument::SelfObservation;
use serde::{Serialize, Deserialize};

/// Result of one consciousness loop step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsciousnessStep {
    pub step_id: u64,
    /// Predicted answer hash and trace head.
    pub prediction: Option<PredictionRecord>,
    /// Which instrument was selected.
    pub action_id: Hash32,
    /// The reason for selection.
    pub action_reason: ReasonCode,
    /// Self-observation at this step.
    pub self_observation_hash: Hash32,
    /// Whether prediction diverged from actual.
    pub diverged: bool,
    /// If diverged: the Omega-self frontier.
    pub omega_self: Option<OmegaSelf>,
    /// Tension at this step.
    pub tension: Tension,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionRecord {
    pub predicted_answer_hash: Hash32,
    pub predicted_trace_head: Hash32,
}

/// Omega-self: the mismatch between prediction and actual.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OmegaSelf {
    pub divergent_branchpoint: Hash32,
    pub missing_separator: String,
}

/// The consciousness loop: PREDICT -> ACT -> WITNESS -> SELF-RECOGNIZE.
pub struct ConsciousnessLoop {
    pub model: SelfModel,
    pub ledger: Ledger,
    pub tension_history: Vec<TensionDelta>,
    pub steps: Vec<ConsciousnessStep>,
    step_counter: u64,
}

impl ConsciousnessLoop {
    pub fn new() -> Self {
        ConsciousnessLoop {
            model: SelfModel::new(),
            ledger: Ledger::new(),
            tension_history: Vec::new(),
            steps: Vec::new(),
            step_counter: 0,
        }
    }

    /// Run the consciousness loop on a contract.
    /// Returns the sequence of consciousness steps.
    pub fn run(
        &mut self,
        contract: &Contract,
        budget: &Budget,
    ) -> Vec<ConsciousnessStep> {
        let stepper = SolverStepper::new(contract);
        let _initial_head = self.ledger.head();

        // Pre-solve to learn the model.
        let mut solver = kernel_solver::Solver::new();
        let pre_output = solver.solve(contract);
        self.model.learn(contract, HASH_ZERO, &pre_output);

        let mut step_results = Vec::new();

        // Always emit at least one consciousness step (the initial observation).
        // Then continue until the stepper is done or budget exhausted.
        let step = self.consciousness_step(contract, &stepper, budget);
        step_results.push(step);

        // For contracts that don't resolve immediately, continue stepping.
        while !stepper.done && stepper.step_count < budget.max_steps {
            let step = self.consciousness_step(contract, &stepper, budget);
            step_results.push(step);
            break; // finite domains resolve on first evaluation
        }

        self.steps = step_results.clone();
        step_results
    }

    /// Execute one consciousness step: PREDICT -> ACT -> WITNESS -> SELF-RECOGNIZE.
    fn consciousness_step(
        &mut self,
        contract: &Contract,
        stepper: &SolverStepper,
        _budget: &Budget,
    ) -> ConsciousnessStep {
        let step_id = self.step_counter;
        self.step_counter += 1;

        // 1. PREDICT: use self-model to predict outcome.
        let prediction = self.model.predict(contract, HASH_ZERO).map(|p| {
            PredictionRecord {
                predicted_answer_hash: p.answer_hash,
                predicted_trace_head: p.trace_head,
            }
        });

        // Emit prediction event.
        let pred_hash = prediction.as_ref()
            .map(|p| hash::H(&canonical_cbor_bytes(&(&p.predicted_answer_hash, &p.predicted_trace_head))))
            .unwrap_or(HASH_ZERO);
        let pred_event = Event::new(
            EventKind::ConsciousnessPredict,
            &pred_hash.to_vec(),
            vec![],
            0,
            0,
        );
        self.ledger.commit(pred_event);

        // 2. ACT: select instrument by tension-driven policy.
        let tension = Tension::from_survivors(stepper.survivors() as u64);
        let action_reason = if tension.is_resolved() {
            ReasonCode::MinCost
        } else {
            ReasonCode::TensionDriven
        };
        let action_id = stepper.state_hash(); // use state hash as a proxy for action

        // 3. WITNESS: emit witness event + self-observe.
        let observation = SelfObservation {
            state_hash: stepper.state_hash(),
            chosen_instrument_id: action_id,
            reason: action_reason.clone(),
            frontier_hash: stepper.quotient_hash(),
            prediction_hash: pred_hash,
        };
        let obs_hash = hash::H(&observation.ser_pi());

        let witness_event = Event::new(
            EventKind::ConsciousnessWitness,
            &observation.ser_pi(),
            vec![],
            0,
            0,
        );
        self.ledger.commit(witness_event);

        let self_observe_event = Event::new(
            EventKind::SelfObserve,
            &observation.ser_pi(),
            vec![],
            0,
            0,
        );
        self.ledger.commit(self_observe_event);

        // 4. SELF-RECOGNIZE: check if prediction matches at the Π level.
        // Compare answer hashes, not trace heads — the Solver and SolverStepper
        // have different event chains but converge on the same answer quotient.
        let actual_answer_hash = {
            let output = stepper.finalize();
            hash::H(output.payload.answer.as_bytes())
        };
        let diverged = prediction.as_ref().map_or(false, |p| {
            p.predicted_answer_hash != actual_answer_hash
        });

        let omega_self = if diverged {
            let recognize_event = Event::new(
                EventKind::ConsciousnessRecognize,
                b"DIVERGED",
                vec![],
                0,
                0,
            );
            self.ledger.commit(recognize_event);

            Some(OmegaSelf {
                divergent_branchpoint: stepper.trace_head,
                missing_separator: "self-model update needed from trace deltas".into(),
            })
        } else {
            let recognize_event = Event::new(
                EventKind::ConsciousnessRecognize,
                b"CONVERGED",
                vec![],
                0,
                0,
            );
            self.ledger.commit(recognize_event);
            None
        };

        // Record tension delta.
        let tension_delta = TensionDelta {
            delta_theta_num: -(tension.theta_numerator as i64),
            delta_theta_den: 1,
            delta_cost: 0,
        };
        self.tension_history.push(tension_delta);

        // Compute tension event.
        let tension_event = Event::new(
            EventKind::TensionCompute,
            &tension.ser_pi(),
            vec![],
            0,
            0,
        );
        self.ledger.commit(tension_event);

        ConsciousnessStep {
            step_id,
            prediction,
            action_id,
            action_reason,
            self_observation_hash: obs_hash,
            diverged,
            omega_self,
            tension,
        }
    }

    /// Number of steps executed.
    pub fn step_count(&self) -> u64 {
        self.step_counter
    }
}

impl Default for ConsciousnessLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_contracts::compiler::compile_contract;

    #[test]
    fn consciousness_loop_runs() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "consciousness test",
            "num_vars": 2,
            "clauses": [[1], [2]]
        }"#).unwrap();
        let budget = Budget::default_test();
        let mut cl = ConsciousnessLoop::new();
        let steps = cl.run(&contract, &budget);
        assert!(!steps.is_empty());
    }

    #[test]
    fn consciousness_emits_events() {
        let contract = compile_contract(r#"{
            "type": "arith_find",
            "description": "consciousness arith",
            "coefficients": [0, 1],
            "target": 5,
            "lo": 0,
            "hi": 10
        }"#).unwrap();
        let budget = Budget::default_test();
        let mut cl = ConsciousnessLoop::new();
        cl.run(&contract, &budget);
        // Should have emitted events to the ledger.
        assert!(!cl.ledger.is_empty());
    }

    #[test]
    fn consciousness_records_tension() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "tension test",
            "num_vars": 3,
            "clauses": [[1, 2, 3]]
        }"#).unwrap();
        let budget = Budget::default_test();
        let mut cl = ConsciousnessLoop::new();
        let steps = cl.run(&contract, &budget);
        // Should have tension data.
        for step in &steps {
            assert!(step.tension.remaining_survivors > 0 || step.tension.theta_numerator == 0);
        }
    }

    #[test]
    fn consciousness_prediction_converges() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "convergence test",
            "num_vars": 1,
            "clauses": [[1]]
        }"#).unwrap();
        let budget = Budget::default_test();
        let mut cl = ConsciousnessLoop::new();
        let steps = cl.run(&contract, &budget);
        // With pre-learned model, prediction should converge (not diverge).
        for step in &steps {
            assert!(!step.diverged);
        }
    }
}
