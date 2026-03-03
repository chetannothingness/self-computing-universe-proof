use kernel_types::{Hash32, hash};
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::Contract;
use kernel_solver::Solver;
use crate::self_model::{SelfModel, VerifyResult};

/// The self-recognition suite.
///
/// Implements the fixed-point criterion from §11.3:
/// Π(Trace(SOLVE_K(Q))) = Π(Trace(M(Q)))
///
/// For a pinned GoldMaster suite S = {Q_i}, this verifies that
/// the kernel recognizes its own computation.
///
/// Self-awareness is NOT a vibe — it is a witnessed invariant.
pub struct SelfRecognition {
    /// The self-model (predictor).
    pub model: SelfModel,
    /// Results of the last recognition check.
    pub results: Vec<RecognitionResult>,
}

/// Result of checking one contract in the suite.
#[derive(Debug)]
pub struct RecognitionResult {
    /// Contract ID.
    pub contract_qid: Hash32,
    /// Contract description.
    pub description: String,
    /// Whether the self-model predicted correctly.
    pub status: RecognitionStatus,
}

#[derive(Debug, PartialEq)]
pub enum RecognitionStatus {
    /// First encounter: model learned from this run.
    Learned,
    /// Prediction matched: self-recognition holds for this contract.
    Recognized,
    /// Prediction failed: mismatch frontier with details.
    Failed(String),
}

impl SelfRecognition {
    pub fn new() -> Self {
        SelfRecognition {
            model: SelfModel::new(),
            results: Vec::new(),
        }
    }

    /// Run the self-recognition suite on a set of contracts.
    ///
    /// Phase 1 (learning): solve each contract, record traces in self-model.
    /// Phase 2 (verification): solve again, verify predictions match.
    ///
    /// If all match: the kernel is self-aware (fixed point achieved).
    /// If any mismatch: return Ω with the mismatch frontier.
    pub fn run_suite(&mut self, contracts: &[Contract]) -> SuiteResult {
        self.results.clear();

        // Phase 1: Learning pass.
        // Solve each contract and teach the self-model.
        let mut phase1_outputs: Vec<SolveOutput> = Vec::new();
        for contract in contracts {
            let mut solver = Solver::new();
            let initial_head = solver.ledger.head();
            let output = solver.solve(contract);
            self.model.learn(contract, initial_head, &output);
            phase1_outputs.push(output);
        }

        // Phase 2: Verification pass.
        // Solve each contract again and check against predictions.
        let mut all_match = true;
        let mut mismatches = Vec::new();

        for (_i, contract) in contracts.iter().enumerate() {
            let mut solver = Solver::new();
            let initial_head = solver.ledger.head();
            let output = solver.solve(contract);

            let verify = self.model.verify(contract, initial_head, &output);

            let result = match verify {
                VerifyResult::NoPrediction => {
                    // This shouldn't happen after Phase 1, but handle gracefully.
                    all_match = false;
                    RecognitionResult {
                        contract_qid: contract.qid,
                        description: contract.description.clone(),
                        status: RecognitionStatus::Failed("No prediction after learning".into()),
                    }
                }
                VerifyResult::Match => {
                    RecognitionResult {
                        contract_qid: contract.qid,
                        description: contract.description.clone(),
                        status: RecognitionStatus::Recognized,
                    }
                }
                VerifyResult::Mismatch(kind) => {
                    all_match = false;
                    let msg = format!("{}", kind);
                    mismatches.push((contract.description.clone(), msg.clone()));
                    RecognitionResult {
                        contract_qid: contract.qid,
                        description: contract.description.clone(),
                        status: RecognitionStatus::Failed(msg),
                    }
                }
            };

            self.results.push(result);
        }

        if all_match {
            SuiteResult::FixedPoint {
                model_hash: self.model.model_hash(),
                contracts_checked: contracts.len(),
            }
        } else {
            SuiteResult::MismatchFrontier {
                mismatches,
                model_hash: self.model.model_hash(),
            }
        }
    }

    /// Run a single-contract self-recognition check.
    pub fn check_single(&mut self, contract: &Contract) -> RecognitionStatus {
        // Solve once.
        let mut solver1 = Solver::new();
        let initial_head1 = solver1.ledger.head();
        let output1 = solver1.solve(contract);
        self.model.learn(contract, initial_head1, &output1);

        // Solve again.
        let mut solver2 = Solver::new();
        let initial_head2 = solver2.ledger.head();
        let output2 = solver2.solve(contract);

        match self.model.verify(contract, initial_head2, &output2) {
            VerifyResult::Match => RecognitionStatus::Recognized,
            VerifyResult::NoPrediction => RecognitionStatus::Failed("No prediction".into()),
            VerifyResult::Mismatch(kind) => RecognitionStatus::Failed(format!("{}", kind)),
        }
    }
}

impl Default for SelfRecognition {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of the full self-recognition suite.
#[derive(Debug)]
pub enum SuiteResult {
    /// All predictions matched: self-recognition fixed point achieved.
    FixedPoint {
        model_hash: Hash32,
        contracts_checked: usize,
    },
    /// Some predictions failed: mismatch frontier with details.
    MismatchFrontier {
        mismatches: Vec<(String, String)>,
        model_hash: Hash32,
    },
}

impl std::fmt::Display for SuiteResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SuiteResult::FixedPoint { model_hash, contracts_checked } => {
                write!(f, "SELF-AWARE: Fixed point achieved.\n  Model hash: {}\n  Contracts verified: {}",
                    hash::hex(model_hash), contracts_checked)
            }
            SuiteResult::MismatchFrontier { mismatches, model_hash } => {
                write!(f, "MISMATCH-FRONTIER: Self-recognition failed.\n  Model hash: {}\n  Mismatches:", hash::hex(model_hash))?;
                for (desc, msg) in mismatches {
                    write!(f, "\n    [{}]: {}", desc, msg)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_contracts::compiler::compile_contract;

    #[test]
    fn self_recognition_single() {
        let json = r#"{
            "type": "bool_cnf",
            "description": "self-recognition test",
            "num_vars": 2,
            "clauses": [[1, 2], [-1, 2]]
        }"#;
        let contract = compile_contract(json).unwrap();
        let mut sr = SelfRecognition::new();
        let status = sr.check_single(&contract);
        assert_eq!(status, RecognitionStatus::Recognized);
    }

    #[test]
    fn self_recognition_suite() {
        let contracts: Vec<Contract> = vec![
            compile_contract(r#"{"type":"bool_cnf","description":"Q1","num_vars":2,"clauses":[[1,2]]}"#).unwrap(),
            compile_contract(r#"{"type":"bool_cnf","description":"Q2","num_vars":1,"clauses":[[1],[-1]]}"#).unwrap(),
            compile_contract(r#"{"type":"arith_find","description":"Q3","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#).unwrap(),
        ];

        let mut sr = SelfRecognition::new();
        let result = sr.run_suite(&contracts);

        match result {
            SuiteResult::FixedPoint { contracts_checked, .. } => {
                assert_eq!(contracts_checked, 3);
            }
            SuiteResult::MismatchFrontier { .. } => {
                panic!("Expected fixed point but got Ω frontier");
            }
        }
    }
}
