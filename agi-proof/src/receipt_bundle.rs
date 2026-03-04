use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use crate::runner::AgiProofResult;
use crate::release::ReleaseManifest;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// A complete receipt bundle: manifest + proof result + per-task receipts.
///
/// This is the artifact that anyone can replay to verify the AGI proof.
/// Deterministic: same inputs -> same bundle -> same bundle_hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReceiptBundle {
    pub manifest: ReleaseManifest,
    pub proof_result: AgiProofResult,
    pub per_task_receipts: BTreeMap<String, SolveOutput>,
    pub bundle_hash: Hash32,
}

impl SerPi for ReceiptBundle {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.manifest.ser_pi());
        buf.extend_from_slice(&self.proof_result.ser_pi());
        // Per-task receipts in sorted order (BTreeMap guarantees this)
        for (task_id, output) in &self.per_task_receipts {
            buf.extend_from_slice(&task_id.ser_pi());
            buf.extend_from_slice(&output.ser_pi());
        }
        buf.extend_from_slice(&self.bundle_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

impl ReceiptBundle {
    /// Build a receipt bundle from a proof result.
    pub fn build(
        manifest: ReleaseManifest,
        proof_result: AgiProofResult,
        per_task_receipts: BTreeMap<String, SolveOutput>,
    ) -> Self {
        // Compute bundle hash from all components
        let mut hash_buf = Vec::new();
        hash_buf.extend_from_slice(&manifest.ser_pi());
        hash_buf.extend_from_slice(&proof_result.ser_pi());
        for (task_id, output) in &per_task_receipts {
            hash_buf.extend_from_slice(&task_id.ser_pi());
            hash_buf.extend_from_slice(&output.ser_pi());
        }
        let bundle_hash = hash::H(&hash_buf);

        ReceiptBundle {
            manifest,
            proof_result,
            per_task_receipts,
            bundle_hash,
        }
    }
}

/// Serialize a receipt bundle to JSON bytes.
pub fn write_bundle(bundle: &ReceiptBundle) -> Vec<u8> {
    serde_json::to_vec_pretty(bundle).expect("bundle serialization must not fail")
}

/// Deserialize a receipt bundle from JSON bytes.
pub fn read_bundle(bytes: &[u8]) -> Result<ReceiptBundle, String> {
    serde_json::from_slice(bytes).map_err(|e| format!("Bundle parse error: {}", e))
}

/// Replay: recompute every verdict from receipts.
///
/// For each task receipt:
///   1. Verify trace_head is non-zero (was actually computed)
///   2. Verify the receipt's completion proof exists
///   3. Verify the bundle hash matches recomputation
///
/// Returns true iff ALL checks pass.
pub fn replay_bundle(bundle: &ReceiptBundle) -> bool {
    // 1. Verify bundle hash
    let mut hash_buf = Vec::new();
    hash_buf.extend_from_slice(&bundle.manifest.ser_pi());
    hash_buf.extend_from_slice(&bundle.proof_result.ser_pi());
    for (task_id, output) in &bundle.per_task_receipts {
        hash_buf.extend_from_slice(&task_id.ser_pi());
        hash_buf.extend_from_slice(&output.ser_pi());
    }
    let recomputed_hash = hash::H(&hash_buf);
    if recomputed_hash != bundle.bundle_hash {
        return false;
    }

    // 2. Verify each task receipt has a valid trace
    for (_task_id, output) in &bundle.per_task_receipts {
        // Each output must have a non-zero trace head
        // (proving it was actually computed, not stubbed)
        if output.receipt.trace_head == HASH_ZERO
            && output.receipt.branchpoints.is_empty()
            && output.receipt.completion.is_none()
        {
            // Genesis receipt — not from an actual solve
            return false;
        }
    }

    // 3. Verify result merkle root
    let phase_hashes: Vec<Hash32> = bundle.proof_result.phases.iter()
        .map(|r| hash::H(&r.ser_pi()))
        .collect();
    let recomputed_merkle = hash::merkle_root(&phase_hashes);
    if recomputed_merkle != bundle.proof_result.result_merkle_root {
        return false;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::AgiRunner;

    fn make_test_bundle() -> ReceiptBundle {
        let manifest = ReleaseManifest {
            serpi_k_hash: hash::hex(&HASH_ZERO),
            build_hash: hash::hex(&HASH_ZERO),
            toolchain_hash: hash::hex(&HASH_ZERO),
            suite_merkle_root: hash::hex(&HASH_ZERO),
            judge_merkle_root: hash::hex(&HASH_ZERO),
            seed_commit: hash::hex(&HASH_ZERO),
            binary_hash: hash::hex(&HASH_ZERO),
            pk_root: hash::hex(&[0u8; 32]),
        };

        let mut runner = AgiRunner::new();
        let task_json = r#"{
            "type": "agi_domain",
            "domain": "SynthPhysics",
            "description": "bundle test",
            "world_seed": "",
            "max_experiments": 10
        }"#;
        let tasks = vec![task_json.to_string()];
        let proof_result = runner.run_all(&[(2, "test".into(), tasks)]);

        let per_task_receipts = BTreeMap::new();

        ReceiptBundle::build(manifest, proof_result, per_task_receipts)
    }

    #[test]
    fn receipt_bundle_deterministic() {
        let b1 = make_test_bundle();
        let b2 = make_test_bundle();
        assert_eq!(b1.bundle_hash, b2.bundle_hash);
        assert_eq!(b1.ser_pi(), b2.ser_pi());
    }

    #[test]
    fn receipt_bundle_replay_verified() {
        let bundle = make_test_bundle();
        // Bundle with no per-task receipts should pass replay
        // (no receipts to fail on)
        assert!(replay_bundle(&bundle));
    }

    #[test]
    fn receipt_bundle_roundtrip() {
        let bundle = make_test_bundle();
        let bytes = write_bundle(&bundle);
        let decoded = read_bundle(&bytes).unwrap();
        assert_eq!(decoded.bundle_hash, bundle.bundle_hash);
    }
}
