use kernel_types::Hash32;
use crate::instrument::Instrument;
use crate::state::State;
use std::collections::BinaryHeap;
use std::cmp::Ordering;

/// A scored instrument entry for the canonical enumerator.
struct ScoredInstrument {
    /// Lower cost = higher priority.
    cost: u64,
    /// Higher refinement = higher priority (secondary).
    expected_refinement: u64,
    /// Canonical tie-break: instrument ID (lexicographic).
    id: Hash32,
    /// Index into the instruments vector.
    index: usize,
}

impl PartialEq for ScoredInstrument {
    fn eq(&self, other: &Self) -> bool {
        self.cost == other.cost
            && self.expected_refinement == other.expected_refinement
            && self.id == other.id
    }
}

impl Eq for ScoredInstrument {}

impl PartialOrd for ScoredInstrument {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ScoredInstrument {
    fn cmp(&self, other: &Self) -> Ordering {
        // BinaryHeap is max-heap, so we reverse for min-cost ordering.
        // Lower cost → higher priority.
        other.cost.cmp(&self.cost)
            // Higher refinement → higher priority.
            .then(self.expected_refinement.cmp(&other.expected_refinement))
            // Tie-break: lexicographic on ID (deterministic).
            .then(other.id.cmp(&self.id))
    }
}

/// The canonical enumerator E_Δ: ℕ → Δ*.
///
/// Cost-monotone, Π-canonical enumeration.
/// "What gets tried" is a deterministic prefix under any budget.
/// No runtime randomness. No "best effort."
pub struct DeltaEnumerator {
    instruments: Vec<Box<dyn Instrument>>,
}

impl DeltaEnumerator {
    pub fn new() -> Self {
        DeltaEnumerator {
            instruments: Vec::new(),
        }
    }

    /// Register an instrument in the universe.
    pub fn register(&mut self, instrument: Box<dyn Instrument>) {
        self.instruments.push(instrument);
    }

    /// Number of registered instruments.
    pub fn len(&self) -> usize {
        self.instruments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.instruments.is_empty()
    }

    /// Get the canonically ordered sequence of instruments for a given state.
    /// Returns indices in priority order (lowest cost, highest refinement, lexicographic ID).
    pub fn canonical_order(&self, state: &State) -> Vec<usize> {
        let mut heap = BinaryHeap::new();
        for (i, inst) in self.instruments.iter().enumerate() {
            heap.push(ScoredInstrument {
                cost: inst.cost(),
                expected_refinement: inst.expected_refinement(state),
                id: inst.id(),
                index: i,
            });
        }
        let mut result = Vec::with_capacity(self.instruments.len());
        while let Some(si) = heap.pop() {
            result.push(si.index);
        }
        result
    }

    /// Get instrument by index.
    pub fn get(&self, index: usize) -> Option<&dyn Instrument> {
        self.instruments.get(index).map(|b| b.as_ref())
    }

    /// Get all instruments.
    pub fn instruments(&self) -> &[Box<dyn Instrument>] {
        &self.instruments
    }
}

impl Default for DeltaEnumerator {
    fn default() -> Self {
        Self::new()
    }
}
