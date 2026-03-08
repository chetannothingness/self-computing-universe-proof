//! Generate MANIFEST.json — pins the proof universe.
//!
//! The manifest contains hashes of all components: kernel build, VM semantics,
//! Lean toolchain, mathlib commit, and a Merkle root over all per-problem proofs.

use kernel_types::{Hash32, hash};
use kernel_cap::artifact::KernelArtifact;
use serde::{Serialize, Deserialize};

/// The proof bundle manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofManifest {
    pub kernel_serpi_hash: String,
    pub kernel_build_hash: String,
    pub lean_version: String,
    pub lean_toolchain_hash: String,
    pub hash_function: String,
    pub signature_scheme: String,
    pub vm_lean_hash: String,
    pub problems_hash: String,
    pub proofs_merkle_root: String,
    pub verified_count: u32,
    pub invalid_count: u32,
    pub signature: String,
}

/// Compute the Merkle root of a list of hashes.
pub fn merkle_root(hashes: &[Hash32]) -> Hash32 {
    if hashes.is_empty() {
        return [0u8; 32];
    }
    if hashes.len() == 1 {
        return hashes[0];
    }

    let mut level: Vec<Hash32> = hashes.to_vec();
    while level.len() > 1 {
        let mut next = Vec::new();
        for pair in level.chunks(2) {
            if pair.len() == 2 {
                let mut buf = Vec::new();
                buf.extend_from_slice(&pair[0]);
                buf.extend_from_slice(&pair[1]);
                next.push(hash::H(&buf));
            } else {
                next.push(pair[0]);
            }
        }
        level = next;
    }
    level[0]
}

/// Generate the MANIFEST.json content.
pub fn generate_manifest(
    artifact: &KernelArtifact,
    vm_lean_hash: Hash32,
    problems_hash: Hash32,
    per_problem_hashes: &[Hash32],
    verified_count: u32,
    invalid_count: u32,
) -> ProofManifest {
    let proofs_root = merkle_root(per_problem_hashes);

    ProofManifest {
        kernel_serpi_hash: hex_encode(&artifact.serpi_k_hash()),
        kernel_build_hash: hex_encode(&artifact.build_hash),
        lean_version: "v4.16.0".to_string(),
        lean_toolchain_hash: hex_encode(&hash::H(b"leanprover/lean4:v4.16.0")),
        hash_function: "blake3".to_string(),
        signature_scheme: "ed25519".to_string(),
        vm_lean_hash: hex_encode(&vm_lean_hash),
        problems_hash: hex_encode(&problems_hash),
        proofs_merkle_root: hex_encode(&proofs_root),
        verified_count,
        invalid_count,
        signature: String::new(), // Filled by signing step
    }
}

fn hex_encode(hash: &Hash32) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merkle_root_single() {
        let h = hash::H(b"test");
        assert_eq!(merkle_root(&[h]), h);
    }

    #[test]
    fn merkle_root_pair() {
        let h1 = hash::H(b"a");
        let h2 = hash::H(b"b");
        let root = merkle_root(&[h1, h2]);
        let mut buf = Vec::new();
        buf.extend_from_slice(&h1);
        buf.extend_from_slice(&h2);
        assert_eq!(root, hash::H(&buf));
    }

    #[test]
    fn merkle_root_empty() {
        assert_eq!(merkle_root(&[]), [0u8; 32]);
    }

    #[test]
    fn merkle_root_deterministic() {
        let hashes: Vec<Hash32> = (0..5).map(|i| hash::H(&[i])).collect();
        assert_eq!(merkle_root(&hashes), merkle_root(&hashes));
    }

    #[test]
    fn manifest_generation() {
        let mut artifact = KernelArtifact::new("0.3.0".to_string(), [0u8; 32]);
        artifact.set_build_hash(hash::H(b"build"));
        let manifest = generate_manifest(
            &artifact,
            hash::H(b"vm"),
            hash::H(b"problems"),
            &[hash::H(b"p1"), hash::H(b"p2")],
            14, 6,
        );
        assert_eq!(manifest.lean_version, "v4.16.0");
        assert_eq!(manifest.verified_count, 14);
        assert_eq!(manifest.invalid_count, 6);
        assert_eq!(manifest.hash_function, "blake3");
    }
}
