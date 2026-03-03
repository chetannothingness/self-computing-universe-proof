use kernel_types::{Hash32, SerPi, hash};
#[cfg(test)]
use kernel_types::HASH_ZERO;
use kernel_contracts::contract::Contract;
use kernel_contracts::compiler::compile_contract;
use kernel_solver::Solver;

/// GoldMaster for SpaceEngine verification.
/// Builds the Q_SE_PROVE contract and pins the catalog Merkle root.
pub struct SpaceEngineGoldMaster {
    pub q_se_prove: Contract,
    pub pinned_catalog_merkle_root: Hash32,
}

impl SpaceEngineGoldMaster {
    /// Build a SpaceEngine goldmaster from a set of goldmaster contracts.
    /// Uses the solve outputs to compute the pinned catalog hash.
    pub fn build(goldmaster_contracts: &[Contract]) -> Self {
        // Solve all goldmaster contracts to get outputs.
        let mut all_hashes = Vec::new();
        for contract in goldmaster_contracts {
            let mut solver = Solver::new();
            let output = solver.solve(contract);
            all_hashes.push(output.ser_pi_hash());
        }

        let pinned_merkle = hash::merkle_root(&all_hashes);
        let pinned_hex = hash::hex(&pinned_merkle);

        // Build the Q_SE_PROVE contract with the pinned catalog hash.
        let json = format!(
            r#"{{"type":"space_engine","description":"Q_SE_PROVE: verify kernel-derived catalog integrity","catalog_hash":"{}","scenario_hash":"pinned","kernel_build_hash":"pinned"}}"#,
            pinned_hex,
        );
        let q_se_prove = compile_contract(&json)
            .expect("Q_SE_PROVE contract must compile");

        SpaceEngineGoldMaster {
            q_se_prove,
            pinned_catalog_merkle_root: pinned_merkle,
        }
    }

    /// Verify that an actual Merkle root matches the pinned one.
    pub fn verify_pinned(&self, actual: &Hash32) -> bool {
        *actual == self.pinned_catalog_merkle_root
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suite::GoldMasterSuite;

    #[test]
    fn space_engine_suite_builds() {
        let gm_suite = GoldMasterSuite::v1();
        let se_gm = SpaceEngineGoldMaster::build(&gm_suite.contracts);
        assert_eq!(se_gm.q_se_prove.description, "Q_SE_PROVE: verify kernel-derived catalog integrity");
        assert!(se_gm.q_se_prove.answer_alphabet.is_space_engine());
    }

    #[test]
    fn pinned_hash_deterministic() {
        let gm_suite = GoldMasterSuite::v1();
        let se1 = SpaceEngineGoldMaster::build(&gm_suite.contracts);
        let se2 = SpaceEngineGoldMaster::build(&gm_suite.contracts);
        assert_eq!(se1.pinned_catalog_merkle_root, se2.pinned_catalog_merkle_root);
    }

    #[test]
    fn goldmaster_verifies() {
        let gm_suite = GoldMasterSuite::v1();
        let se_gm = SpaceEngineGoldMaster::build(&gm_suite.contracts);
        // Pinned hash should verify against itself.
        assert!(se_gm.verify_pinned(&se_gm.pinned_catalog_merkle_root));
        // Different hash should not verify.
        assert!(!se_gm.verify_pinned(&HASH_ZERO));
    }
}
