use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_contracts::contract::{Contract, EvalSpec};
use kernel_contracts::compiler::compile_contract;

/// A mutation applied to a base task.
#[derive(Debug, Clone)]
pub struct Mutation {
    /// ID of the original task being mutated.
    pub original_task_id: String,
    /// Kind of mutation applied.
    pub mutation_kind: MutationKind,
    /// The mutated contract.
    pub mutated_contract: Contract,
}

/// Kinds of mutations for suite expansion.
#[derive(Debug, Clone)]
pub enum MutationKind {
    /// Perturb inputs (change coefficients, bounds, etc.).
    InputPerturb,
    /// Flip expected output polarity (SAT→UNSAT or vice versa).
    OutputFlip,
    /// Target edge cases (boundary values, empty domains).
    EdgeCase,
    /// Adversarial: pathological inputs designed to trip heuristics.
    Adversarial,
}

impl SerPi for MutationKind {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            MutationKind::InputPerturb => 0,
            MutationKind::OutputFlip => 1,
            MutationKind::EdgeCase => 2,
            MutationKind::Adversarial => 3,
        };
        canonical_cbor_bytes(&tag)
    }
}

/// A suite expansion: base suite + mutations.
/// Each expansion becomes part of SerPi(K) identity.
pub struct SuiteExpansion {
    /// The base contracts.
    pub base_contracts: Vec<Contract>,
    /// The mutations applied.
    pub mutations: Vec<Mutation>,
    /// Hash of the expansion (base + mutations).
    pub expansion_hash: Hash32,
}

impl SuiteExpansion {
    /// Build an expansion from a base suite by applying standard mutations.
    pub fn from_base(base_contracts: &[Contract]) -> Self {
        let mut mutations = Vec::new();

        // Generate edge case mutations based on eval type.
        for (i, contract) in base_contracts.iter().enumerate() {
            match &contract.eval {
                EvalSpec::ArithFind { .. } => {
                    let edge_json = format!(
                        r#"{{"type":"arith_find","description":"edge_{}: zero target","coefficients":[0],"target":0,"lo":-1,"hi":1}}"#,
                        i
                    );
                    if let Ok(mutated) = compile_contract(&edge_json) {
                        mutations.push(Mutation {
                            original_task_id: format!("base_{}", i),
                            mutation_kind: MutationKind::EdgeCase,
                            mutated_contract: mutated,
                        });
                    }
                }
                EvalSpec::BoolCnf { .. } => {
                    let perturb_json = format!(
                        r#"{{"type":"bool_cnf","description":"perturb_{}: extra clause","num_vars":2,"clauses":[[1],[2],[-1,-2]]}}"#,
                        i
                    );
                    if let Ok(mutated) = compile_contract(&perturb_json) {
                        mutations.push(Mutation {
                            original_task_id: format!("base_{}", i),
                            mutation_kind: MutationKind::InputPerturb,
                            mutated_contract: mutated,
                        });
                    }
                }
                _ => {}
            }
        }

        // Compute expansion hash.
        let mut hash_buf = Vec::new();
        for c in base_contracts {
            hash_buf.extend_from_slice(&c.qid);
        }
        for m in &mutations {
            hash_buf.extend_from_slice(&m.mutated_contract.qid);
            hash_buf.extend_from_slice(&m.mutation_kind.ser_pi());
        }
        let expansion_hash = hash::H(&hash_buf);

        SuiteExpansion {
            base_contracts: base_contracts.to_vec(),
            mutations,
            expansion_hash,
        }
    }

    /// All contracts in the expansion (base + mutated).
    pub fn all_contracts(&self) -> Vec<&Contract> {
        let mut all: Vec<&Contract> = self.base_contracts.iter().collect();
        for m in &self.mutations {
            all.push(&m.mutated_contract);
        }
        all
    }

    /// Total number of contracts.
    pub fn total(&self) -> usize {
        self.base_contracts.len() + self.mutations.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suite::GoldMasterSuite;

    #[test]
    fn expansion_from_goldmaster() {
        let suite = GoldMasterSuite::v1();
        let expansion = SuiteExpansion::from_base(&suite.contracts);
        assert!(expansion.total() > suite.len());
        assert!(!expansion.mutations.is_empty());
    }

    #[test]
    fn expansion_hash_deterministic() {
        let suite = GoldMasterSuite::v1();
        let e1 = SuiteExpansion::from_base(&suite.contracts);
        let e2 = SuiteExpansion::from_base(&suite.contracts);
        assert_eq!(e1.expansion_hash, e2.expansion_hash);
    }

    #[test]
    fn expansion_includes_base() {
        let suite = GoldMasterSuite::v1();
        let expansion = SuiteExpansion::from_base(&suite.contracts);
        assert!(expansion.all_contracts().len() >= suite.len());
    }
}
