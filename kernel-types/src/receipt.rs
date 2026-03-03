use serde::{Serialize, Deserialize};
use crate::{Hash32, HASH_ZERO};
use crate::status::Status;
use crate::serpi::{SerPi, canonical_cbor_bytes};

/// Completion proof from A1.
///
/// Every solve carries this: either the B*(Q) derivation for completable
/// contracts, or the admissibility refutation for inadmissible ones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionProof {
    /// The completion bound, if derivable. None for inadmissible contracts.
    pub b_star: Option<u64>,
    /// Summary of the completion derivation (for completable)
    /// or the refutation reason (for inadmissible).
    pub summary: String,
    /// Hash of the completion/refutation proof.
    pub proof_hash: Hash32,
}

impl SerPi for CompletionProof {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.b_star.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.summary.ser_pi());
        buf.extend_from_slice(&self.proof_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// The payload of a kernel output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// The answer (for UNIQUE) or empty string.
    pub answer: String,
    /// The witness data (serialized).
    pub witness: Vec<u8>,
}

impl SerPi for Payload {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.answer.ser_pi());
        buf.extend_from_slice(&self.witness.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// The minimal receipt schema (§15 of the anchor, post-A1).
/// Every kernel output carries this.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    /// Kernel serialization hash.
    pub serpi_k_hash: Hash32,
    /// Build identity hash.
    pub build_hash: Hash32,
    /// Running trace head at completion.
    pub trace_head: Hash32,
    /// Hashes of branchpoint snapshots.
    pub branchpoints: Vec<Hash32>,
    /// Current ledger head.
    pub ledger_head: Hash32,
    /// Completion proof from A1 — every solve carries this.
    pub completion: Option<CompletionProof>,
}

impl Receipt {
    pub fn genesis() -> Self {
        Receipt {
            serpi_k_hash: HASH_ZERO,
            build_hash: HASH_ZERO,
            trace_head: HASH_ZERO,
            branchpoints: Vec::new(),
            ledger_head: HASH_ZERO,
            completion: None,
        }
    }
}

impl SerPi for Receipt {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.serpi_k_hash.ser_pi());
        buf.extend_from_slice(&self.build_hash.ser_pi());
        buf.extend_from_slice(&self.trace_head.ser_pi());
        for bp in &self.branchpoints {
            buf.extend_from_slice(&bp.ser_pi());
        }
        buf.extend_from_slice(&self.ledger_head.ser_pi());
        if let Some(c) = &self.completion {
            buf.extend_from_slice(&c.ser_pi());
        }
        canonical_cbor_bytes(&buf)
    }
}

/// The complete kernel output: SOLVE_K(Q) = Ser_Π(STATUS || PAYLOAD || RECEIPT).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolveOutput {
    pub status: Status,
    pub payload: Payload,
    pub receipt: Receipt,
}

impl SerPi for SolveOutput {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.status.ser_pi());
        buf.extend_from_slice(&self.payload.ser_pi());
        buf.extend_from_slice(&self.receipt.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}
