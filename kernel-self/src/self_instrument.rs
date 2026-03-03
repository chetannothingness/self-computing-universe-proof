use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::reason::ReasonCode;
use kernel_ledger::{Event, EventKind};
use kernel_instruments::instrument::{Instrument, InstrumentResult, InstrumentOutcome};
use kernel_instruments::state::{State, StateDelta};
use kernel_instruments::budget::Budget;
use serde::{Serialize, Deserialize};

/// Self-observation: what the kernel sees when it looks at itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfObservation {
    pub state_hash: Hash32,
    pub chosen_instrument_id: Hash32,
    pub reason: ReasonCode,
    pub frontier_hash: Hash32,
    pub prediction_hash: Hash32,
}

impl SerPi for SelfObservation {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.state_hash.ser_pi());
        buf.extend_from_slice(&self.chosen_instrument_id.ser_pi());
        buf.extend_from_slice(&self.reason.ser_pi());
        buf.extend_from_slice(&self.frontier_hash.ser_pi());
        buf.extend_from_slice(&self.prediction_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Self instrument: the kernel observes its own state.
/// Cost = 0 (self-observation IS the kernel). Shrink = 0 (no external information gained).
pub struct SelfInstrument {
    /// The instrument that was chosen at this step.
    pub chosen_instrument_id: Hash32,
    /// Why it was chosen.
    pub reason: ReasonCode,
    /// Hash of the current frontier (if any).
    pub frontier_hash: Hash32,
    /// Hash of the prediction for this step (if any).
    pub prediction_hash: Hash32,
    /// Precomputed ID.
    id: Hash32,
}

impl SelfInstrument {
    pub fn new(
        chosen_instrument_id: Hash32,
        reason: ReasonCode,
        frontier_hash: Hash32,
        prediction_hash: Hash32,
    ) -> Self {
        let mut id_input = Vec::new();
        id_input.extend_from_slice(b"SelfInstrument:");
        id_input.extend_from_slice(&chosen_instrument_id);
        id_input.extend_from_slice(&reason.ser_pi());
        let id = hash::H(&id_input);
        SelfInstrument {
            chosen_instrument_id,
            reason,
            frontier_hash,
            prediction_hash,
            id,
        }
    }
}

impl Instrument for SelfInstrument {
    fn id(&self) -> Hash32 {
        self.id
    }

    fn cost(&self) -> u64 {
        0 // self-observation is free -- it IS the kernel
    }

    fn name(&self) -> &str {
        "SelfInstrument"
    }

    fn apply(&self, state: &State, _budget: &Budget) -> InstrumentResult {
        let state_hash = hash::H(&state.ser_pi());

        let observation = SelfObservation {
            state_hash,
            chosen_instrument_id: self.chosen_instrument_id,
            reason: self.reason.clone(),
            frontier_hash: self.frontier_hash,
            prediction_hash: self.prediction_hash,
        };

        let obs_bytes = observation.ser_pi();
        let outcome = InstrumentOutcome {
            value: obs_bytes.clone(),
            shrink: 0, // self-observation doesn't reduce external ambiguity
        };

        let event = Event::new(
            EventKind::SelfObserve,
            &obs_bytes,
            vec![],
            0, // free
            0, // no shrink
        );

        let delta = StateDelta::empty()
            .with_update(
                b"self:last_observation".to_vec(),
                obs_bytes,
            );

        InstrumentResult {
            outcome,
            delta,
            cost: 0,
            events: vec![event],
        }
    }

    fn expected_refinement(&self, _state: &State) -> u64 {
        0 // self-observation doesn't refine the answer quotient
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn self_instrument_id_deterministic() {
        let a = SelfInstrument::new(HASH_ZERO, ReasonCode::MinCost, HASH_ZERO, HASH_ZERO);
        let b = SelfInstrument::new(HASH_ZERO, ReasonCode::MinCost, HASH_ZERO, HASH_ZERO);
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn self_instrument_cost_is_zero() {
        let inst = SelfInstrument::new(HASH_ZERO, ReasonCode::MinCost, HASH_ZERO, HASH_ZERO);
        assert_eq!(inst.cost(), 0);
    }

    #[test]
    fn self_instrument_apply_total() {
        let inst = SelfInstrument::new(HASH_ZERO, ReasonCode::TensionDriven, HASH_ZERO, HASH_ZERO);
        let state = State::new();
        let budget = Budget::default_test();
        let result = inst.apply(&state, &budget);
        assert_eq!(result.cost, 0);
        assert_eq!(result.outcome.shrink, 0);
        assert!(!result.events.is_empty());
    }

    #[test]
    fn self_observation_deterministic() {
        let obs = SelfObservation {
            state_hash: HASH_ZERO,
            chosen_instrument_id: HASH_ZERO,
            reason: ReasonCode::MaxRefinement,
            frontier_hash: HASH_ZERO,
            prediction_hash: HASH_ZERO,
        };
        assert_eq!(obs.ser_pi(), obs.ser_pi());
    }
}
