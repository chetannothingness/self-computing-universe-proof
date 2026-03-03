use serde::{Serialize, Deserialize};
use crate::serpi::{SerPi, canonical_cbor_bytes};

/// The dit gate (post-A1). The ONLY admissible output statuses.
/// No other output type exists in the kernel.
///
/// Under A1 (Completion), Ω is deleted. Budgets are theorems, not parameters.
/// A contract either derives B*(Q) and is solved to UNIQUE/UNSAT,
/// or it is proved inadmissible (UNSAT with admissibility refutation).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Status {
    /// |Ans_W(Q)| = 1. One answer, with a replay witness
    /// collapsing all survivors to that answer.
    Unique,

    /// Ans_W(Q) = ∅. Either:
    /// (a) exhaustive search found no satisfying candidate, OR
    /// (b) contract is inadmissible — no finite B*(Q) derivable (UNSAT(admissibility)).
    Unsat,
}

impl SerPi for Status {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            Status::Unique => 0,
            Status::Unsat => 1,
        };
        canonical_cbor_bytes(&tag)
    }
}

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Unique => write!(f, "UNIQUE"),
            Status::Unsat => write!(f, "UNSAT"),
        }
    }
}
