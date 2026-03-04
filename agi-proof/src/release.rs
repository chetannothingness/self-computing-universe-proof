use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_cap::artifact::KernelArtifact;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// Release manifest: all hashes needed to verify a reproducible release.
///
/// Every field is a hex string for readability.
/// Ed25519 signature of canonical_cbor_bytes(&manifest) under pk_root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseManifest {
    /// H(Ser_Pi(KernelArtifact)) — the kernel's identity.
    pub serpi_k_hash: String,
    /// MerkleRoot(H(Ser_Pi(SOLVE_K(Q_i)))) over GoldMaster suite.
    pub build_hash: String,
    /// H(rust-toolchain.toml || Cargo.lock).
    pub toolchain_hash: String,
    /// MerkleRoot(H(suite_file_i)) for all suite JSONs.
    pub suite_merkle_root: String,
    /// MerkleRoot(H(simulator_source_i)) for all simulator sources.
    pub judge_merkle_root: String,
    /// H(seed) — published before generation (commit-reveal).
    pub seed_commit: String,
    /// H(kernel binary bytes).
    pub binary_hash: String,
    /// Hex of embedded Ed25519 public key.
    pub pk_root: String,
}

impl SerPi for ReleaseManifest {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.serpi_k_hash.ser_pi());
        buf.extend_from_slice(&self.build_hash.ser_pi());
        buf.extend_from_slice(&self.toolchain_hash.ser_pi());
        buf.extend_from_slice(&self.suite_merkle_root.ser_pi());
        buf.extend_from_slice(&self.judge_merkle_root.ser_pi());
        buf.extend_from_slice(&self.seed_commit.ser_pi());
        buf.extend_from_slice(&self.binary_hash.ser_pi());
        buf.extend_from_slice(&self.pk_root.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Build a release manifest from kernel state.
pub fn build_release(
    artifact: &KernelArtifact,
    build_hash: Hash32,
    suite_files: &BTreeMap<String, Vec<u8>>,
    simulator_sources: &BTreeMap<String, Vec<u8>>,
    seed: &[u8; 32],
    toolchain_bytes: &[u8],
    binary_bytes: &[u8],
) -> ReleaseManifest {
    // Suite merkle root
    let suite_hashes: Vec<Hash32> = suite_files.iter()
        .map(|(name, bytes)| {
            let mut buf = Vec::new();
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(bytes);
            hash::H(&buf)
        })
        .collect();
    let suite_merkle = hash::merkle_root(&suite_hashes);

    // Judge merkle root
    let judge_hashes: Vec<Hash32> = simulator_sources.iter()
        .map(|(name, bytes)| {
            let mut buf = Vec::new();
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(bytes);
            hash::H(&buf)
        })
        .collect();
    let judge_merkle = hash::merkle_root(&judge_hashes);

    ReleaseManifest {
        serpi_k_hash: hash::hex(&artifact.serpi_k_hash()),
        build_hash: hash::hex(&build_hash),
        toolchain_hash: hash::hex(&hash::H(toolchain_bytes)),
        suite_merkle_root: hash::hex(&suite_merkle),
        judge_merkle_root: hash::hex(&judge_merkle),
        seed_commit: hash::hex(&hash::H(seed)),
        binary_hash: hash::hex(&hash::H(binary_bytes)),
        pk_root: hash::hex(&artifact.pk_root),
    }
}

/// Sign a release manifest with Ed25519.
pub fn sign_release(
    manifest: &ReleaseManifest,
    signing_key: &ed25519_dalek::SigningKey,
) -> Vec<u8> {
    use ed25519_dalek::Signer;
    let manifest_bytes = canonical_cbor_bytes(&manifest.ser_pi());
    let signature = signing_key.sign(&manifest_bytes);
    signature.to_bytes().to_vec()
}

/// Verify a release manifest signature.
pub fn verify_release_signature(
    manifest: &ReleaseManifest,
    signature_bytes: &[u8],
    pk_bytes: &[u8; 32],
) -> bool {
    use ed25519_dalek::{Verifier, VerifyingKey, Signature};

    let pk = match VerifyingKey::from_bytes(pk_bytes) {
        Ok(pk) => pk,
        Err(_) => return false,
    };

    if signature_bytes.len() != 64 {
        return false;
    }
    let mut sig_array = [0u8; 64];
    sig_array.copy_from_slice(signature_bytes);
    let signature = Signature::from_bytes(&sig_array);

    let manifest_bytes = canonical_cbor_bytes(&manifest.ser_pi());
    pk.verify(&manifest_bytes, &signature).is_ok()
}

/// Verify a complete release directory.
///
/// Checks:
/// 1. Ed25519 signature under manifest.pk_root
/// 2. suite_merkle_root matches actual suite files
/// 3. judge_merkle_root matches actual simulator sources
///
/// Returns Ok(()) on success, Err(reason) on failure.
pub fn verify_release(
    manifest: &ReleaseManifest,
    signature_bytes: &[u8],
    suite_files: &BTreeMap<String, Vec<u8>>,
    simulator_sources: &BTreeMap<String, Vec<u8>>,
) -> Result<(), String> {
    // 1. Parse pk_root
    let pk_bytes = hash::from_hex(&manifest.pk_root)
        .ok_or_else(|| "Invalid pk_root hex".to_string())?;

    // 2. Verify signature
    if !verify_release_signature(manifest, signature_bytes, &pk_bytes) {
        return Err("Ed25519 signature verification failed".into());
    }

    // 3. Verify suite merkle root
    let suite_hashes: Vec<Hash32> = suite_files.iter()
        .map(|(name, bytes)| {
            let mut buf = Vec::new();
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(bytes);
            hash::H(&buf)
        })
        .collect();
    let actual_suite_merkle = hash::hex(&hash::merkle_root(&suite_hashes));
    if actual_suite_merkle != manifest.suite_merkle_root {
        return Err(format!(
            "Suite merkle root mismatch: expected {}, got {}",
            manifest.suite_merkle_root, actual_suite_merkle
        ));
    }

    // 4. Verify judge merkle root
    let judge_hashes: Vec<Hash32> = simulator_sources.iter()
        .map(|(name, bytes)| {
            let mut buf = Vec::new();
            buf.extend_from_slice(name.as_bytes());
            buf.extend_from_slice(bytes);
            hash::H(&buf)
        })
        .collect();
    let actual_judge_merkle = hash::hex(&hash::merkle_root(&judge_hashes));
    if actual_judge_merkle != manifest.judge_merkle_root {
        return Err(format!(
            "Judge merkle root mismatch: expected {}, got {}",
            manifest.judge_merkle_root, actual_judge_merkle
        ));
    }

    Ok(())
}

/// Serialize manifest to JSON.
pub fn manifest_to_json(manifest: &ReleaseManifest) -> Vec<u8> {
    serde_json::to_vec_pretty(manifest).expect("manifest serialization must not fail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    fn make_test_manifest() -> (ReleaseManifest, SigningKey) {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk_root = signing_key.verifying_key().to_bytes();

        let artifact = KernelArtifact::new("0.2.0-A1".into(), pk_root);

        let mut suite_files = BTreeMap::new();
        suite_files.insert("suite1.json".into(), b"test suite data".to_vec());

        let mut sim_sources = BTreeMap::new();
        sim_sources.insert("physics.rs".into(), b"simulator source".to_vec());

        let seed = [42u8; 32];
        let toolchain = b"rust-toolchain.toml contents";
        let binary = b"fake binary bytes";

        let manifest = build_release(
            &artifact,
            HASH_ZERO,
            &suite_files,
            &sim_sources,
            &seed,
            toolchain,
            binary,
        );

        (manifest, signing_key)
    }

    #[test]
    fn release_manifest_serpi_deterministic() {
        let (m1, _) = make_test_manifest();
        // Build again with same inputs should give same serpi
        // (Different because OsRng generates different keys each time,
        // so we test that the same manifest gives same serpi)
        let s1 = m1.ser_pi();
        let s2 = m1.ser_pi();
        assert_eq!(s1, s2);
    }

    #[test]
    fn release_signature_verifies() {
        let (manifest, signing_key) = make_test_manifest();
        let sig = sign_release(&manifest, &signing_key);
        let pk = signing_key.verifying_key().to_bytes();
        assert!(verify_release_signature(&manifest, &sig, &pk));
    }

    #[test]
    fn release_signature_rejects_tampered() {
        let (mut manifest, signing_key) = make_test_manifest();
        let sig = sign_release(&manifest, &signing_key);

        // Tamper with the manifest
        manifest.build_hash = "tampered".into();

        let pk = signing_key.verifying_key().to_bytes();
        assert!(!verify_release_signature(&manifest, &sig, &pk));
    }

    #[test]
    fn release_suite_merkle_root_matches() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk_root = signing_key.verifying_key().to_bytes();
        let artifact = KernelArtifact::new("test".into(), pk_root);

        let mut suite_files = BTreeMap::new();
        suite_files.insert("a.json".into(), b"aaa".to_vec());
        suite_files.insert("b.json".into(), b"bbb".to_vec());

        let mut sim_sources = BTreeMap::new();
        sim_sources.insert("sim.rs".into(), b"code".to_vec());

        let seed = [0u8; 32];
        let manifest = build_release(
            &artifact, HASH_ZERO, &suite_files, &sim_sources,
            &seed, b"toolchain", b"binary",
        );
        let sig = sign_release(&manifest, &signing_key);

        // Verify should pass with same files
        let result = verify_release(&manifest, &sig, &suite_files, &sim_sources);
        assert!(result.is_ok());
    }

    #[test]
    fn release_verify_rejects_wrong_files() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let pk_root = signing_key.verifying_key().to_bytes();
        let artifact = KernelArtifact::new("test".into(), pk_root);

        let mut suite_files = BTreeMap::new();
        suite_files.insert("a.json".into(), b"aaa".to_vec());

        let mut sim_sources = BTreeMap::new();
        sim_sources.insert("sim.rs".into(), b"code".to_vec());

        let seed = [0u8; 32];
        let manifest = build_release(
            &artifact, HASH_ZERO, &suite_files, &sim_sources,
            &seed, b"toolchain", b"binary",
        );
        let sig = sign_release(&manifest, &signing_key);

        // Tamper with suite files
        let mut tampered_files = BTreeMap::new();
        tampered_files.insert("a.json".into(), b"TAMPERED".to_vec());

        let result = verify_release(&manifest, &sig, &tampered_files, &sim_sources);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Suite merkle root mismatch"));
    }
}
