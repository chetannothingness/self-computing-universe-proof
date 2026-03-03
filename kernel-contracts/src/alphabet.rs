use kernel_types::SerPi;
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// The answer alphabet: the finite set of possible answers to a contract.
///
/// All answers must be finite descriptions in D*.
/// |A| < ∞ always — but for FormalProof, the enumeration is
/// structurally impossible under any finite budget, so the
/// kernel MUST return Ω with a frontier.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AnswerAlphabet {
    /// Boolean: {true, false}
    Bool,
    /// Finite set of explicit values.
    Finite(Vec<Vec<u8>>),
    /// Integer range [lo, hi] inclusive.
    IntRange { lo: i64, hi: i64 },
    /// Byte strings up to max_len.
    Bytes { max_len: usize },
    /// Formal proof/disproof: {PROOF, DISPROOF}.
    /// The answer is a proof term (finite string) that must
    /// pass a pinned verifier. The space of valid proof terms
    /// is finite but astronomically large — exhaustive search
    /// is structurally infeasible. The kernel must recognize
    /// this and return Ω with a sharp frontier.
    FormalProof {
        /// Hash of the pinned verifier binary.
        verifier_hash: Vec<u8>,
        /// Name of the formal system (e.g., "Lean4", "Isabelle/HOL").
        formal_system: String,
        /// Hash of the pinned library (e.g., Mathlib commit hash).
        library_hash: Vec<u8>,
    },

    /// Dominance verdict: {DOMINANT, NOT_DOMINANT}.
    /// Binary alphabet for DOMINATE(S, M) meta-contracts.
    /// Always admissible (B* = 2).
    DominanceVerdict {
        /// Hash of the suite being used for comparison.
        suite_hash: Vec<u8>,
    },

    /// SpaceEngine verification verdict: {VERIFIED, NOT_VERIFIED}.
    /// Binary alphabet for Q_SE_PROVE and Q_SE_WITNESS_VERIFY contracts.
    /// Always admissible (B* = 2).
    SpaceEngineVerdict,
}

impl AnswerAlphabet {
    /// Enumerate all values in the alphabet (for finite domains).
    pub fn enumerate(&self) -> Vec<Vec<u8>> {
        match self {
            AnswerAlphabet::Bool => {
                vec![b"TRUE".to_vec(), b"FALSE".to_vec()]
            }
            AnswerAlphabet::Finite(vals) => vals.clone(),
            AnswerAlphabet::IntRange { lo, hi } => {
                (*lo..=*hi).map(|i| canonical_cbor_bytes(&i)).collect()
            }
            AnswerAlphabet::Bytes { max_len } => {
                // Only enumerate for very small max_len (up to 3 bytes).
                if *max_len > 3 {
                    panic!("Cannot enumerate Bytes alphabet with max_len > 3");
                }
                let mut result = Vec::new();
                let limit = 1usize << (8 * max_len);
                for i in 0..limit {
                    let bytes: Vec<u8> = (0..*max_len)
                        .map(|b| ((i >> (8 * b)) & 0xFF) as u8)
                        .collect();
                    result.push(bytes);
                }
                result
            }
            AnswerAlphabet::FormalProof { .. } => {
                // NOT enumerable. Return empty — the solver must check
                // is_enumerable() before calling enumerate().
                // This is a structural guard, not a silent failure.
                vec![]
            }
            AnswerAlphabet::DominanceVerdict { .. } => {
                vec![b"DOMINANT".to_vec(), b"NOT_DOMINANT".to_vec()]
            }
            AnswerAlphabet::SpaceEngineVerdict => {
                vec![b"VERIFIED".to_vec(), b"NOT_VERIFIED".to_vec()]
            }
        }
    }

    /// Whether this alphabet is finitely enumerable under any practical budget.
    pub fn is_enumerable(&self) -> bool {
        !matches!(self, AnswerAlphabet::FormalProof { .. })
    }

    /// Whether this is a SpaceEngine verdict alphabet.
    pub fn is_space_engine(&self) -> bool {
        matches!(self, AnswerAlphabet::SpaceEngineVerdict)
    }

    /// Whether this is a dominance verdict alphabet.
    pub fn is_dominance(&self) -> bool {
        matches!(self, AnswerAlphabet::DominanceVerdict { .. })
    }

    /// Size of the alphabet.
    pub fn size(&self) -> u64 {
        match self {
            AnswerAlphabet::Bool => 2,
            AnswerAlphabet::Finite(vals) => vals.len() as u64,
            AnswerAlphabet::IntRange { lo, hi } => (hi - lo + 1) as u64,
            AnswerAlphabet::Bytes { max_len } => 1u64 << (8 * max_len),
            AnswerAlphabet::FormalProof { .. } => u64::MAX, // structurally infeasible
            AnswerAlphabet::DominanceVerdict { .. } => 2,
            AnswerAlphabet::SpaceEngineVerdict => 2,
        }
    }
}

impl SerPi for AnswerAlphabet {
    fn ser_pi(&self) -> Vec<u8> {
        match self {
            AnswerAlphabet::Bool => canonical_cbor_bytes(&("Bool", 0u8)),
            AnswerAlphabet::Finite(vals) => canonical_cbor_bytes(&("Finite", vals)),
            AnswerAlphabet::IntRange { lo, hi } => canonical_cbor_bytes(&("IntRange", lo, hi)),
            AnswerAlphabet::Bytes { max_len } => canonical_cbor_bytes(&("Bytes", *max_len as u64)),
            AnswerAlphabet::FormalProof { verifier_hash, formal_system, library_hash } => {
                canonical_cbor_bytes(&("FormalProof", verifier_hash, formal_system.as_str(), library_hash))
            }
            AnswerAlphabet::DominanceVerdict { suite_hash } => {
                canonical_cbor_bytes(&("DominanceVerdict", suite_hash))
            }
            AnswerAlphabet::SpaceEngineVerdict => {
                canonical_cbor_bytes(&("SpaceEngineVerdict", 0u8))
            }
        }
    }
}
