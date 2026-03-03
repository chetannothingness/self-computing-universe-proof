use kernel_types::{Hash32, SerPi, hash};
use kernel_ledger::Event;
use crate::state::{State, StateDelta};
use crate::budget::Budget;

/// An outcome from an instrument application.
/// Must be SerPi (canonically serializable).
#[derive(Debug, Clone)]
pub struct InstrumentOutcome {
    /// The observed value.
    pub value: Vec<u8>,
    /// How many survivors were eliminated (ΔT surrogate).
    pub shrink: u64,
}

impl SerPi for InstrumentOutcome {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.value.ser_pi());
        buf.extend_from_slice(&self.shrink.ser_pi());
        kernel_types::serpi::canonical_cbor_bytes(&buf)
    }
}

/// The result of applying an instrument.
pub struct InstrumentResult {
    /// The outcome (observation).
    pub outcome: InstrumentOutcome,
    /// The state delta (how state was refined).
    pub delta: StateDelta,
    /// The cost of this application (ΔE).
    pub cost: u64,
    /// Ledger events generated.
    pub events: Vec<Event>,
}

/// The Instrument trait.
///
/// An instrument is a total function: state → (outcome, state').
/// - Classical regime: instruments commute.
/// - Quantum regime: instruments may not commute (order is ledger-recorded).
///
/// Every instrument is:
/// - Total (always terminates with an outcome, even if FAIL/TIMEOUT)
/// - Deterministic (same state → same outcome)
/// - Receipt-logged (produces events)
pub trait Instrument: Send + Sync {
    /// Canonical identity of this instrument: H(Ser_Π(definition)).
    fn id(&self) -> Hash32;

    /// The irreducible cost of executing this instrument: π_K(I).
    fn cost(&self) -> u64;

    /// A human-readable name for debugging.
    fn name(&self) -> &str;

    /// Apply the instrument to the state under the given budget.
    /// Returns (outcome, delta, cost, events).
    /// This is TOTAL: it must always return, even on failure.
    fn apply(&self, state: &State, budget: &Budget) -> InstrumentResult;

    /// How much this instrument would refine the answer quotient.
    /// Higher = more useful. Used for separator selection.
    fn expected_refinement(&self, state: &State) -> u64;
}
