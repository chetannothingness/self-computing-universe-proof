use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::Contract;
use kernel_solver::Solver;
use std::collections::BTreeMap;

/// The self-model predictor M.
///
/// Given (Contract, Budget, LedgerHead), predicts:
/// - The trace head after solving
/// - The branchpoint sequence
/// - The status and answer
///
/// This is "self-awareness" in the only admissible sense:
/// the kernel can predict its own computation and verify
/// itself against itself.
///
/// The self-model is learned ONLY from replayed traces
/// (no hidden updates, no external knowledge).
pub struct SelfModel {
    /// Map from (contract_hash, initial_ledger_head) → predicted output hash.
    /// This is the compressed predictor.
    predictions: BTreeMap<(Hash32, Hash32), PredictedOutput>,
}

/// What the self-model predicts for a given (contract, ledger_head).
#[derive(Debug, Clone)]
pub struct PredictedOutput {
    /// Predicted trace head after solving.
    pub trace_head: Hash32,
    /// Predicted branchpoint sequence.
    pub branchpoints: Vec<Hash32>,
    /// Predicted status.
    pub status_hash: Hash32,
    /// Predicted answer hash.
    pub answer_hash: Hash32,
    /// Full output hash for quick comparison.
    pub output_hash: Hash32,
}

impl SerPi for PredictedOutput {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.trace_head.ser_pi());
        for bp in &self.branchpoints {
            buf.extend_from_slice(&bp.ser_pi());
        }
        buf.extend_from_slice(&self.status_hash.ser_pi());
        buf.extend_from_slice(&self.answer_hash.ser_pi());
        buf.extend_from_slice(&self.output_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

impl SelfModel {
    pub fn new() -> Self {
        SelfModel {
            predictions: BTreeMap::new(),
        }
    }

    /// Learn from an observed solve: record the (contract, ledger_head) → output mapping.
    pub fn learn(&mut self, contract: &Contract, initial_ledger_head: Hash32, output: &SolveOutput) {
        let key = (contract.qid, initial_ledger_head);
        let predicted = PredictedOutput {
            trace_head: output.receipt.trace_head,
            branchpoints: output.receipt.branchpoints.clone(),
            status_hash: output.status.ser_pi_hash(),
            answer_hash: hash::H(output.payload.answer.as_bytes()),
            output_hash: output.ser_pi_hash(),
        };
        self.predictions.insert(key, predicted);
    }

    /// Predict the output for a given contract and initial ledger head.
    /// Returns None if no prediction exists (first encounter).
    pub fn predict(&self, contract: &Contract, initial_ledger_head: Hash32) -> Option<&PredictedOutput> {
        let key = (contract.qid, initial_ledger_head);
        self.predictions.get(&key)
    }

    /// Verify a prediction against actual output.
    /// Returns true if the prediction matches (self-recognition holds).
    pub fn verify(&self, contract: &Contract, initial_ledger_head: Hash32, actual: &SolveOutput) -> VerifyResult {
        match self.predict(contract, initial_ledger_head) {
            None => VerifyResult::NoPrediction,
            Some(predicted) => {
                let actual_output_hash = actual.ser_pi_hash();
                if predicted.output_hash == actual_output_hash {
                    VerifyResult::Match
                } else {
                    // Find the first divergence.
                    let mismatch = if predicted.trace_head != actual.receipt.trace_head {
                        MismatchKind::TraceHead {
                            predicted: predicted.trace_head,
                            actual: actual.receipt.trace_head,
                        }
                    } else if predicted.status_hash != actual.status.ser_pi_hash() {
                        MismatchKind::Status {
                            predicted: predicted.status_hash,
                            actual: actual.status.ser_pi_hash(),
                        }
                    } else {
                        MismatchKind::Answer {
                            predicted: predicted.answer_hash,
                            actual: hash::H(actual.payload.answer.as_bytes()),
                        }
                    };
                    VerifyResult::Mismatch(mismatch)
                }
            }
        }
    }

    /// Number of learned predictions.
    pub fn len(&self) -> usize {
        self.predictions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.predictions.is_empty()
    }

    /// The self-model's own hash (for identity purposes).
    pub fn model_hash(&self) -> Hash32 {
        let mut hashes: Vec<Hash32> = Vec::new();
        for ((qid, lh), pred) in &self.predictions {
            let mut entry_buf = Vec::new();
            entry_buf.extend_from_slice(qid);
            entry_buf.extend_from_slice(lh);
            entry_buf.extend_from_slice(&pred.ser_pi());
            hashes.push(hash::H(&entry_buf));
        }
        hash::merkle_root(&hashes)
    }
}

impl Default for SelfModel {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of verifying a self-model prediction.
#[derive(Debug)]
pub enum VerifyResult {
    /// No prediction existed for this input (first encounter).
    NoPrediction,
    /// Prediction matched actual output (self-recognition holds).
    Match,
    /// Prediction did not match (self-recognition FAILED — Ω frontier).
    Mismatch(MismatchKind),
}

/// The kind of mismatch between prediction and actual output.
#[derive(Debug)]
pub enum MismatchKind {
    TraceHead { predicted: Hash32, actual: Hash32 },
    Status { predicted: Hash32, actual: Hash32 },
    Answer { predicted: Hash32, actual: Hash32 },
}

impl std::fmt::Display for MismatchKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MismatchKind::TraceHead { predicted, actual } => {
                write!(f, "TraceHead mismatch: predicted={}, actual={}",
                    hash::hex(predicted), hash::hex(actual))
            }
            MismatchKind::Status { predicted, actual } => {
                write!(f, "Status mismatch: predicted={}, actual={}",
                    hash::hex(predicted), hash::hex(actual))
            }
            MismatchKind::Answer { predicted, actual } => {
                write!(f, "Answer mismatch: predicted={}, actual={}",
                    hash::hex(predicted), hash::hex(actual))
            }
        }
    }
}
