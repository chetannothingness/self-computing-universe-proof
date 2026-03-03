use kernel_types::{Hash32, SerPi, hash};
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::Contract;
use kernel_solver::Solver;
use crate::suite::GoldMasterSuite;

/// Compute BuildHash(K) = MerkleRoot(H(Ser_Π(SOLVE_K(Q_i))))_{i=1}^n
///
/// This is Π-correct identity: two builds are "the same" iff
/// indistinguishable on the GoldMaster suite S.
pub fn compute_build_hash(suite: &GoldMasterSuite) -> (Hash32, Vec<SolveOutput>) {
    let mut output_hashes: Vec<Hash32> = Vec::new();
    let mut outputs: Vec<SolveOutput> = Vec::new();

    for contract in &suite.contracts {
        let mut solver = Solver::new();
        let output = solver.solve(contract);
        let output_hash = hash::H(&output.ser_pi());
        output_hashes.push(output_hash);
        outputs.push(output);
    }

    let build_hash = hash::merkle_root(&output_hashes);
    (build_hash, outputs)
}

/// Verify that a build hash matches the expected outputs.
pub fn verify_build_hash(suite: &GoldMasterSuite, expected_build_hash: &Hash32) -> BuildHashVerification {
    let (actual_hash, outputs) = compute_build_hash(suite);

    if actual_hash == *expected_build_hash {
        BuildHashVerification::Match {
            build_hash: actual_hash,
            contracts_verified: suite.len(),
        }
    } else {
        // Find first divergence by re-running individually.
        let mut first_mismatch = None;
        let mut solver_check = Solver::new();
        for (i, contract) in suite.contracts.iter().enumerate() {
            let mut s1 = Solver::new();
            let mut s2 = Solver::new();
            let o1 = s1.solve(contract);
            let o2 = s2.solve(contract);
            if o1.ser_pi() != o2.ser_pi() {
                first_mismatch = Some((i, contract.description.clone()));
                break;
            }
        }

        BuildHashVerification::Mismatch {
            expected: *expected_build_hash,
            actual: actual_hash,
            first_divergence: first_mismatch,
        }
    }
}

/// Result of build hash verification.
#[derive(Debug)]
pub enum BuildHashVerification {
    Match {
        build_hash: Hash32,
        contracts_verified: usize,
    },
    Mismatch {
        expected: Hash32,
        actual: Hash32,
        first_divergence: Option<(usize, String)>,
    },
}

impl std::fmt::Display for BuildHashVerification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildHashVerification::Match { build_hash, contracts_verified } => {
                write!(f, "BUILD HASH VERIFIED: {}\n  Contracts: {}",
                    hash::hex(build_hash), contracts_verified)
            }
            BuildHashVerification::Mismatch { expected, actual, first_divergence } => {
                write!(f, "BUILD HASH MISMATCH:\n  Expected: {}\n  Actual:   {}",
                    hash::hex(expected), hash::hex(actual))?;
                if let Some((i, desc)) = first_divergence {
                    write!(f, "\n  First divergence at Q{}: {}", i, desc)?;
                }
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_hash_deterministic() {
        let suite = GoldMasterSuite::v1();
        let (h1, _) = compute_build_hash(&suite);
        let (h2, _) = compute_build_hash(&suite);
        assert_eq!(h1, h2, "BuildHash must be deterministic");
    }

    #[test]
    fn build_hash_nonzero() {
        let suite = GoldMasterSuite::v1();
        let (h, _) = compute_build_hash(&suite);
        assert_ne!(h, kernel_types::HASH_ZERO, "BuildHash must not be zero");
    }

    #[test]
    fn build_hash_verifies() {
        let suite = GoldMasterSuite::v1();
        let (h, _) = compute_build_hash(&suite);
        match verify_build_hash(&suite, &h) {
            BuildHashVerification::Match { .. } => {} // expected
            other => panic!("Expected Match, got {:?}", other),
        }
    }
}
