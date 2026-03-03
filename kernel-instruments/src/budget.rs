use kernel_types::SerPi;
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// Execution budget for a contract or instrument.
/// All resources are explicitly bounded — no unbounded computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// Maximum cost (energy) allowed.
    pub max_cost: u64,
    /// Maximum number of refinement steps.
    pub max_steps: u64,
    /// Maximum number of instruments to try.
    pub max_instruments: u64,
}

impl Budget {
    pub fn new(max_cost: u64, max_steps: u64, max_instruments: u64) -> Self {
        Budget { max_cost, max_steps, max_instruments }
    }

    /// Default generous budget for testing.
    pub fn default_test() -> Self {
        Budget {
            max_cost: 1_000_000,
            max_steps: 10_000,
            max_instruments: 1_000,
        }
    }

    /// Check if we can afford a given cost.
    pub fn can_afford(&self, cost: u64, current_cost: u64) -> bool {
        current_cost + cost <= self.max_cost
    }

    /// Check if we have steps remaining.
    pub fn has_steps(&self, current_steps: u64) -> bool {
        current_steps < self.max_steps
    }
}

impl SerPi for Budget {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(self.max_cost, self.max_steps, self.max_instruments))
    }
}
