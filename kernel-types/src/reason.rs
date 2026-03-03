use crate::serpi::{SerPi, canonical_cbor_bytes};
use serde::{Serialize, Deserialize};

/// Reason code for instrument selection decisions.
/// Finite enum -- not free text. Fully deterministic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReasonCode {
    /// Selected because it has the minimum cost.
    MinCost,
    /// Selected because it maximizes refinement.
    MaxRefinement,
    /// Selected because tension-driven: argmax(delta_theta / delta_E).
    TensionDriven,
    /// Tie broken by lexicographic ordering on instrument ID.
    TiebreakLex,
    /// Forced by Omega frontier (cheapest missing separator).
    FrontierForced,
}

impl SerPi for ReasonCode {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            ReasonCode::MinCost => 0,
            ReasonCode::MaxRefinement => 1,
            ReasonCode::TensionDriven => 2,
            ReasonCode::TiebreakLex => 3,
            ReasonCode::FrontierForced => 4,
        };
        canonical_cbor_bytes(&tag)
    }
}

impl std::fmt::Display for ReasonCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReasonCode::MinCost => write!(f, "MIN_COST"),
            ReasonCode::MaxRefinement => write!(f, "MAX_REFINEMENT"),
            ReasonCode::TensionDriven => write!(f, "TENSION_DRIVEN"),
            ReasonCode::TiebreakLex => write!(f, "TIEBREAK_LEX"),
            ReasonCode::FrontierForced => write!(f, "FRONTIER_FORCED"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_codes_differ() {
        let codes = vec![
            ReasonCode::MinCost,
            ReasonCode::MaxRefinement,
            ReasonCode::TensionDriven,
            ReasonCode::TiebreakLex,
            ReasonCode::FrontierForced,
        ];
        for i in 0..codes.len() {
            for j in (i + 1)..codes.len() {
                assert_ne!(codes[i].ser_pi(), codes[j].ser_pi());
            }
        }
    }

    #[test]
    fn reason_code_deterministic() {
        let r = ReasonCode::TensionDriven;
        assert_eq!(r.ser_pi(), r.ser_pi());
    }
}
