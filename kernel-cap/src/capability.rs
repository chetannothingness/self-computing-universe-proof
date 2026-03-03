use kernel_types::{Hash32, SerPi};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// A capability: a signed authorization for an override action.
///
/// No "secret admin strings," ever. Override is proven only by:
/// - sig verifies under embedded pk_root
/// - cap binds to build_hash and ledger challenge
/// - nonce not replayed
/// - scope allowlisted
/// - behavioral divergence proven by trace (door lemma)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    /// The scope of this capability (must be in allow_scopes).
    pub scope: String,
    /// The build hash this capability binds to.
    pub build_hash: Hash32,
    /// Challenge: binds to current ledger head.
    pub ledger_challenge: Hash32,
    /// Nonce: unique per capability, non-replayable.
    pub nonce: Hash32,
    /// Ed25519 signature over (scope || build_hash || ledger_challenge || nonce).
    pub signature: Vec<u8>,
}

impl Capability {
    /// The message that was signed.
    pub fn signed_message(&self) -> Vec<u8> {
        let mut msg = Vec::new();
        msg.extend_from_slice(self.scope.as_bytes());
        msg.extend_from_slice(&self.build_hash);
        msg.extend_from_slice(&self.ledger_challenge);
        msg.extend_from_slice(&self.nonce);
        msg
    }
}

impl SerPi for Capability {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.scope.ser_pi());
        buf.extend_from_slice(&self.build_hash.ser_pi());
        buf.extend_from_slice(&self.ledger_challenge.ser_pi());
        buf.extend_from_slice(&self.nonce.ser_pi());
        buf.extend_from_slice(&self.signature.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}
