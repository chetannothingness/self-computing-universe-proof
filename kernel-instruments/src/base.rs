use kernel_types::{Hash32, SerPi, hash};
use kernel_ledger::{Event, EventKind};
use crate::instrument::{Instrument, InstrumentResult, InstrumentOutcome};
use crate::state::{State, StateDelta};
use crate::budget::Budget;

/// Evaluate a single candidate from a finite domain.
/// This is the base instrument for exhaustive search over finite answer spaces.
///
/// Given a candidate value and an evaluation function (as bytecode/key),
/// it checks whether this candidate satisfies the contract.
pub struct EvalCandidate {
    /// The candidate value to test.
    pub candidate: Vec<u8>,
    /// The evaluation function key (looked up in state).
    pub eval_key: Vec<u8>,
    /// Precomputed ID.
    id: Hash32,
}

impl EvalCandidate {
    pub fn new(candidate: Vec<u8>, eval_key: Vec<u8>) -> Self {
        let mut id_input = Vec::new();
        id_input.extend_from_slice(b"EvalCandidate:");
        id_input.extend_from_slice(&candidate);
        id_input.extend_from_slice(b":");
        id_input.extend_from_slice(&eval_key);
        let id = hash::H(&id_input);
        EvalCandidate { candidate, eval_key, id }
    }
}

impl Instrument for EvalCandidate {
    fn id(&self) -> Hash32 {
        self.id
    }

    fn cost(&self) -> u64 {
        1 // minimal cost for a single evaluation
    }

    fn name(&self) -> &str {
        "EvalCandidate"
    }

    fn apply(&self, state: &State, _budget: &Budget) -> InstrumentResult {
        // Look up the evaluation result for this candidate.
        // The state should contain an entry mapping candidate → result.
        // If not found, the candidate is not in the domain (eliminated).
        let result_key = {
            let mut k = self.eval_key.clone();
            k.extend_from_slice(b":");
            k.extend_from_slice(&self.candidate);
            k
        };

        let (value, shrink) = match state.get(&result_key) {
            Some(v) => (v.clone(), 1u64),
            None => (b"NOT_IN_DOMAIN".to_vec(), 0u64),
        };

        let outcome = InstrumentOutcome {
            value: value.clone(),
            shrink,
        };

        let event = Event::new(
            EventKind::InstrumentApplied,
            &outcome.ser_pi(),
            vec![],
            1,
            shrink,
        );

        InstrumentResult {
            outcome,
            delta: StateDelta::empty(),
            cost: 1,
            events: vec![event],
        }
    }

    fn expected_refinement(&self, _state: &State) -> u64 {
        1 // each candidate evaluation refines by exactly 1
    }
}

/// Check equality: does a given state value equal an expected value?
/// This is the simplest separator instrument.
pub struct CheckEquality {
    pub key: Vec<u8>,
    pub expected: Vec<u8>,
    id: Hash32,
}

impl CheckEquality {
    pub fn new(key: Vec<u8>, expected: Vec<u8>) -> Self {
        let mut id_input = Vec::new();
        id_input.extend_from_slice(b"CheckEquality:");
        id_input.extend_from_slice(&key);
        id_input.extend_from_slice(b":");
        id_input.extend_from_slice(&expected);
        let id = hash::H(&id_input);
        CheckEquality { key, expected, id }
    }
}

impl Instrument for CheckEquality {
    fn id(&self) -> Hash32 {
        self.id
    }

    fn cost(&self) -> u64 {
        1
    }

    fn name(&self) -> &str {
        "CheckEquality"
    }

    fn apply(&self, state: &State, _budget: &Budget) -> InstrumentResult {
        let matches = state.get(&self.key)
            .map(|v| v == &self.expected)
            .unwrap_or(false);

        let value = if matches { b"TRUE".to_vec() } else { b"FALSE".to_vec() };
        let shrink = if matches { 1 } else { 0 };

        let outcome = InstrumentOutcome { value, shrink };
        let event = Event::new(
            EventKind::InstrumentApplied,
            &outcome.ser_pi(),
            vec![],
            1,
            shrink,
        );

        InstrumentResult {
            outcome,
            delta: StateDelta::empty(),
            cost: 1,
            events: vec![event],
        }
    }

    fn expected_refinement(&self, _state: &State) -> u64 {
        1
    }
}

/// Exhaustive finite domain search instrument.
/// Given a finite set of candidates and an eval function,
/// tests all candidates and returns the full partition.
pub struct ExhaustiveSearch {
    pub domain: Vec<Vec<u8>>,
    pub eval_key: Vec<u8>,
    id: Hash32,
}

impl ExhaustiveSearch {
    pub fn new(domain: Vec<Vec<u8>>, eval_key: Vec<u8>) -> Self {
        let mut id_input = Vec::new();
        id_input.extend_from_slice(b"ExhaustiveSearch:");
        for d in &domain {
            id_input.extend_from_slice(d);
            id_input.push(0xFF);
        }
        id_input.extend_from_slice(&eval_key);
        let id = hash::H(&id_input);
        ExhaustiveSearch { domain, eval_key, id }
    }
}

impl Instrument for ExhaustiveSearch {
    fn id(&self) -> Hash32 {
        self.id
    }

    fn cost(&self) -> u64 {
        self.domain.len() as u64
    }

    fn name(&self) -> &str {
        "ExhaustiveSearch"
    }

    fn apply(&self, state: &State, _budget: &Budget) -> InstrumentResult {
        let mut satisfying = Vec::new();
        let mut events = Vec::new();

        for candidate in &self.domain {
            let result_key = {
                let mut k = self.eval_key.clone();
                k.extend_from_slice(b":");
                k.extend_from_slice(candidate);
                k
            };

            if let Some(v) = state.get(&result_key) {
                if v == b"SAT" || v == b"TRUE" || v == b"1" {
                    satisfying.push(candidate.clone());
                }
            }
        }

        let value = if satisfying.is_empty() {
            b"UNSAT".to_vec()
        } else if satisfying.len() == 1 {
            let mut v = b"UNIQUE:".to_vec();
            v.extend_from_slice(&satisfying[0]);
            v
        } else {
            let mut v = b"MULTIPLE:".to_vec();
            v.extend_from_slice(&(satisfying.len() as u64).ser_pi());
            v
        };

        let shrink = (self.domain.len() - satisfying.len()) as u64;

        let outcome = InstrumentOutcome { value, shrink };
        let event = Event::new(
            EventKind::InstrumentApplied,
            &outcome.ser_pi(),
            vec![],
            self.domain.len() as u64,
            shrink,
        );
        events.push(event);

        // Store the satisfying set in state delta for downstream use
        let delta = StateDelta::empty()
            .with_update(
                b"__survivors__".to_vec(),
                kernel_types::serpi::canonical_cbor_bytes(&satisfying),
            );

        InstrumentResult {
            outcome,
            delta,
            cost: self.domain.len() as u64,
            events,
        }
    }

    fn expected_refinement(&self, _state: &State) -> u64 {
        self.domain.len() as u64
    }
}
