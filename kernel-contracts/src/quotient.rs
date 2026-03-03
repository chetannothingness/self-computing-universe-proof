use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use std::collections::BTreeSet;

/// The answer quotient: Ans_W(Q) = {q(x) : x ∈ W(L)}.
///
/// Tracks the partition of survivors by their answer values.
/// Refinement (instrument application) can only shrink or maintain
/// the survivor set — never expand it.
#[derive(Debug, Clone)]
pub struct AnswerQuotient {
    /// The surviving answer classes.
    /// Each entry is a canonical answer value.
    survivors: BTreeSet<Vec<u8>>,
    /// Total candidates originally in the domain.
    original_size: usize,
}

impl AnswerQuotient {
    /// Create from the full answer domain (all candidates are survivors initially).
    pub fn from_domain(candidates: Vec<Vec<u8>>) -> Self {
        let original_size = candidates.len();
        let survivors: BTreeSet<Vec<u8>> = candidates.into_iter().collect();
        AnswerQuotient { survivors, original_size }
    }

    /// Number of remaining survivors.
    pub fn size(&self) -> usize {
        self.survivors.len()
    }

    /// Is this UNIQUE? (exactly one survivor)
    pub fn is_unique(&self) -> bool {
        self.survivors.len() == 1
    }

    /// Is this UNSAT? (no survivors)
    pub fn is_unsat(&self) -> bool {
        self.survivors.is_empty()
    }

    /// Get the unique answer (if UNIQUE).
    pub fn unique_answer(&self) -> Option<&Vec<u8>> {
        if self.survivors.len() == 1 {
            self.survivors.iter().next()
        } else {
            None
        }
    }

    /// Eliminate a candidate from the survivor set.
    /// Returns true if it was present (i.e., ΔT > 0).
    pub fn eliminate(&mut self, candidate: &[u8]) -> bool {
        self.survivors.remove(candidate)
    }

    /// Keep only the specified candidates (intersection).
    pub fn retain_only(&mut self, keep: &BTreeSet<Vec<u8>>) {
        self.survivors = self.survivors.intersection(keep).cloned().collect();
    }

    /// How much shrinkage happened from original.
    pub fn shrink(&self) -> usize {
        self.original_size - self.survivors.len()
    }

    /// Get all survivors (for witness construction).
    pub fn survivors(&self) -> &BTreeSet<Vec<u8>> {
        &self.survivors
    }

    /// Canonical hash of the current quotient state.
    pub fn quotient_hash(&self) -> Hash32 {
        let sorted: Vec<Vec<u8>> = self.survivors.iter().cloned().collect();
        hash::H(&canonical_cbor_bytes(&sorted))
    }
}

impl SerPi for AnswerQuotient {
    fn ser_pi(&self) -> Vec<u8> {
        let sorted: Vec<Vec<u8>> = self.survivors.iter().cloned().collect();
        canonical_cbor_bytes(&sorted)
    }
}
