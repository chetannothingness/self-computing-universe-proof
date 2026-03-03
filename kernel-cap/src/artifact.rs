use kernel_types::{Hash32, HASH_ZERO, SerPi};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// The kernel artifact: the canonical identity of this build.
///
/// Embeds pk_root so that capabilities can be verified
/// without external key distribution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelArtifact {
    /// Kernel version string.
    pub version: String,
    /// Root public key (Ed25519, 32 bytes).
    pub pk_root: [u8; 32],
    /// Allowed override scopes.
    pub allow_scopes: Vec<String>,
    /// Hash of the Δ* closure rules.
    pub delta_rules_hash: Hash32,
    /// Build identity hash (computed from GoldMaster suite).
    pub build_hash: Hash32,
}

impl KernelArtifact {
    /// Create a new artifact with a fresh keypair.
    pub fn new(version: String, pk_root: [u8; 32]) -> Self {
        KernelArtifact {
            version,
            pk_root,
            allow_scopes: vec!["override".into(), "extend_budget".into()],
            delta_rules_hash: HASH_ZERO,
            build_hash: HASH_ZERO,
        }
    }

    /// Update the build hash (set by kernel-goldmaster).
    pub fn set_build_hash(&mut self, bh: Hash32) {
        self.build_hash = bh;
    }

    /// Update the delta rules hash.
    pub fn set_delta_rules_hash(&mut self, drh: Hash32) {
        self.delta_rules_hash = drh;
    }

    /// The canonical serialization hash of this artifact.
    /// This is Ser_Π(K) — the kernel's identity.
    pub fn serpi_k_hash(&self) -> Hash32 {
        self.ser_pi_hash()
    }
}

impl SerPi for KernelArtifact {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.version.ser_pi());
        buf.extend_from_slice(&self.pk_root.ser_pi());
        // Sort allow_scopes for canonical ordering.
        let mut sorted_scopes = self.allow_scopes.clone();
        sorted_scopes.sort();
        for scope in &sorted_scopes {
            buf.extend_from_slice(&scope.ser_pi());
        }
        buf.extend_from_slice(&self.delta_rules_hash.ser_pi());
        buf.extend_from_slice(&self.build_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}
