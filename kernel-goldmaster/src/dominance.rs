use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_contracts::contract::Contract;
use kernel_contracts::compiler::compile_contract;

/// A dominance suite: a collection of contracts pinned for DOMINATE comparison.
pub struct DominanceSuite {
    /// The pinned contracts used for comparison.
    pub contracts: Vec<Contract>,
    /// Hash of the suite (Merkle root of contract qids).
    pub suite_hash: Hash32,
    /// Competitor identifiers to compare against.
    pub competitor_ids: Vec<String>,
}

impl DominanceSuite {
    /// Build a dominance suite from competitor identifiers.
    /// Generates DOMINATE contracts for each competitor.
    pub fn build(competitor_ids: Vec<String>) -> Self {
        let mut contracts = Vec::new();

        for competitor_id in &competitor_ids {
            let json = format!(
                r#"{{"type":"dominate","description":"DOMINATE vs {}","competitor_id":"{}","suite_hash":"pinned","scoring":"lex:verified_success,false_claims,cost"}}"#,
                competitor_id, competitor_id
            );
            contracts.push(compile_contract(&json).expect("Dominate contract must compile"));
        }

        let qid_hashes: Vec<Hash32> = contracts.iter()
            .map(|c| c.qid)
            .collect();
        let suite_hash = hash::merkle_root(&qid_hashes);

        DominanceSuite {
            contracts,
            suite_hash,
            competitor_ids,
        }
    }

    /// Number of contracts in the suite.
    pub fn len(&self) -> usize {
        self.contracts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.contracts.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominance_suite_builds() {
        let suite = DominanceSuite::build(vec!["gpt-4".into(), "gemini".into()]);
        assert_eq!(suite.len(), 2);
        assert_eq!(suite.competitor_ids.len(), 2);
    }

    #[test]
    fn dominance_suite_hash_deterministic() {
        let suite1 = DominanceSuite::build(vec!["agent-a".into()]);
        let suite2 = DominanceSuite::build(vec!["agent-a".into()]);
        assert_eq!(suite1.suite_hash, suite2.suite_hash);
    }

    #[test]
    fn dominance_suite_hash_differs_by_competitor() {
        let suite1 = DominanceSuite::build(vec!["agent-a".into()]);
        let suite2 = DominanceSuite::build(vec!["agent-b".into()]);
        assert_ne!(suite1.suite_hash, suite2.suite_hash);
    }
}
