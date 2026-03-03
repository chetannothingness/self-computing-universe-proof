use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// Addon manifest metadata. All hashes stored as hex strings for readability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddonManifest {
    pub version: String,
    pub kernel_build_hash: String,
    pub catalog_merkle_root: String,
    pub scenario_hash: String,
    pub star_count: usize,
    pub galaxy_count: usize,
    pub nebula_count: usize,
    pub dark_object_count: usize,
    pub cluster_count: usize,
    // Enhanced L2/L3 fields (backward compatible via serde defaults)
    #[serde(default)]
    pub witness_moon_count: usize,
    #[serde(default)]
    pub witness_cluster_count: usize,
    #[serde(default)]
    pub witness_planet_count: usize,
    #[serde(default)]
    pub lensing_proxy_count: usize,
    #[serde(default)]
    pub filament_count: usize,
    #[serde(default)]
    pub frontier_count: usize,
    #[serde(default)]
    pub atlas_hash: String,
}

impl AddonManifest {
    /// Parse catalog_merkle_root back into Hash32.
    pub fn catalog_merkle_root_hash(&self) -> Hash32 {
        hash::from_hex(&self.catalog_merkle_root).unwrap_or([0u8; 32])
    }

    /// Parse kernel_build_hash back into Hash32.
    pub fn kernel_build_hash_hash(&self) -> Hash32 {
        hash::from_hex(&self.kernel_build_hash).unwrap_or([0u8; 32])
    }
}

impl SerPi for AddonManifest {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.version.ser_pi());
        buf.extend_from_slice(&self.kernel_build_hash.ser_pi());
        buf.extend_from_slice(&self.catalog_merkle_root.ser_pi());
        buf.extend_from_slice(&self.scenario_hash.ser_pi());
        buf.extend_from_slice(&(self.star_count as u64).ser_pi());
        buf.extend_from_slice(&(self.galaxy_count as u64).ser_pi());
        buf.extend_from_slice(&(self.nebula_count as u64).ser_pi());
        buf.extend_from_slice(&(self.dark_object_count as u64).ser_pi());
        buf.extend_from_slice(&(self.cluster_count as u64).ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Proof receipt for a verification run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofReceipt {
    pub q_se_prove_qid: String,
    pub verdict: String,
    pub trace_head: String,
    pub composite_hash: String,
}

impl SerPi for ProofReceipt {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.q_se_prove_qid.ser_pi());
        buf.extend_from_slice(&self.verdict.ser_pi());
        buf.extend_from_slice(&self.trace_head.ser_pi());
        buf.extend_from_slice(&self.composite_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Generates manifest and proof receipt files.
pub struct ManifestGenerator;

impl ManifestGenerator {
    pub fn build_manifest(
        version: &str,
        kernel_build_hash: Hash32,
        catalog_merkle_root: Hash32,
        scenario_hash: Hash32,
        star_count: usize,
        galaxy_count: usize,
        nebula_count: usize,
        dark_object_count: usize,
        cluster_count: usize,
    ) -> AddonManifest {
        AddonManifest {
            version: version.to_string(),
            kernel_build_hash: hash::hex(&kernel_build_hash),
            catalog_merkle_root: hash::hex(&catalog_merkle_root),
            scenario_hash: hash::hex(&scenario_hash),
            star_count,
            galaxy_count,
            nebula_count,
            dark_object_count,
            cluster_count,
            witness_moon_count: 0,
            witness_cluster_count: 0,
            witness_planet_count: 0,
            lensing_proxy_count: 0,
            filament_count: 0,
            frontier_count: 0,
            atlas_hash: String::new(),
        }
    }

    pub fn build_receipt(
        q_se_prove_qid: Hash32,
        verdict: &str,
        trace_head: Hash32,
        composite_hash: Hash32,
    ) -> ProofReceipt {
        ProofReceipt {
            q_se_prove_qid: hash::hex(&q_se_prove_qid),
            verdict: verdict.to_string(),
            trace_head: hash::hex(&trace_head),
            composite_hash: hash::hex(&composite_hash),
        }
    }

    /// Build an enhanced manifest with L2/L3 layer counts.
    pub fn build_enhanced_manifest(
        base: AddonManifest,
        witness_moon_count: usize,
        witness_cluster_count: usize,
        witness_planet_count: usize,
        lensing_proxy_count: usize,
        filament_count: usize,
        frontier_count: usize,
        atlas_hash: Hash32,
    ) -> AddonManifest {
        AddonManifest {
            witness_moon_count,
            witness_cluster_count,
            witness_planet_count,
            lensing_proxy_count,
            filament_count,
            frontier_count,
            atlas_hash: hash::hex(&atlas_hash),
            ..base
        }
    }

    pub fn manifest_to_json(manifest: &AddonManifest) -> Vec<u8> {
        serde_json::to_vec_pretty(manifest).expect("manifest serialization must not fail")
    }

    pub fn receipt_to_json(receipt: &ProofReceipt) -> Vec<u8> {
        serde_json::to_vec_pretty(receipt).expect("receipt serialization must not fail")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;

    #[test]
    fn enhanced_manifest_new_fields() {
        let base = ManifestGenerator::build_manifest(
            "1.0", HASH_ZERO, HASH_ZERO, HASH_ZERO, 5, 3, 2, 1, 1,
        );
        let atlas_hash = hash::H(b"atlas");
        let enhanced = ManifestGenerator::build_enhanced_manifest(
            base, 10, 2, 3, 1, 4, 1, atlas_hash,
        );
        assert_eq!(enhanced.witness_moon_count, 10);
        assert_eq!(enhanced.witness_cluster_count, 2);
        assert_eq!(enhanced.witness_planet_count, 3);
        assert_eq!(enhanced.lensing_proxy_count, 1);
        assert_eq!(enhanced.filament_count, 4);
        assert_eq!(enhanced.frontier_count, 1);
        assert_eq!(enhanced.atlas_hash, hash::hex(&atlas_hash));
        // Base fields preserved
        assert_eq!(enhanced.star_count, 5);
        assert_eq!(enhanced.galaxy_count, 3);
    }

    #[test]
    fn enhanced_manifest_backward_compatible() {
        // A base manifest serialized as JSON can be deserialized with enhanced fields defaulting.
        let base = ManifestGenerator::build_manifest(
            "1.0", HASH_ZERO, HASH_ZERO, HASH_ZERO, 1, 1, 1, 0, 0,
        );
        let json = ManifestGenerator::manifest_to_json(&base);
        // Remove enhanced fields from JSON to simulate old format
        let old_json = r#"{
            "version": "1.0",
            "kernel_build_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "catalog_merkle_root": "0000000000000000000000000000000000000000000000000000000000000000",
            "scenario_hash": "0000000000000000000000000000000000000000000000000000000000000000",
            "star_count": 1,
            "galaxy_count": 1,
            "nebula_count": 1,
            "dark_object_count": 0,
            "cluster_count": 0
        }"#;
        let deserialized: AddonManifest = serde_json::from_str(old_json).unwrap();
        assert_eq!(deserialized.witness_moon_count, 0);
        assert_eq!(deserialized.atlas_hash, "");
        // Also verify the full manifest round-trips
        let _round: AddonManifest = serde_json::from_slice(&json).unwrap();
    }
}
