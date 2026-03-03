use kernel_types::{SerPi, hash};
use kernel_ledger::{Ledger, Event, EventKind};
use crate::artifact::KernelArtifact;
use crate::capability::Capability;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};

/// Jurisdiction check result.
#[derive(Debug)]
pub enum JmResult {
    /// Capability is valid and authorized.
    Authorized,
    /// Capability failed verification.
    Denied(String),
}

/// The Jurisdiction Manager.
///
/// Verifies capabilities against the kernel artifact.
/// Acceptance requires ALL of:
/// 1. Signature verifies under embedded pk_root
/// 2. Cap binds to build_hash
/// 3. Cap binds to current ledger challenge
/// 4. Nonce not replayed (ledgered)
/// 5. Scope is allowlisted
pub struct JurisdictionChecker {
    artifact: KernelArtifact,
}

impl JurisdictionChecker {
    pub fn new(artifact: KernelArtifact) -> Self {
        JurisdictionChecker { artifact }
    }

    /// Verify a capability against the current kernel state.
    pub fn verify(&self, cap: &Capability, ledger: &mut Ledger) -> JmResult {
        // 1. Check scope is allowlisted.
        if !self.artifact.allow_scopes.contains(&cap.scope) {
            return JmResult::Denied(format!("Scope '{}' not in allow list", cap.scope));
        }

        // 2. Check build_hash matches.
        if cap.build_hash != self.artifact.build_hash {
            return JmResult::Denied(format!(
                "Build hash mismatch: cap={}, artifact={}",
                hash::hex(&cap.build_hash),
                hash::hex(&self.artifact.build_hash),
            ));
        }

        // 3. Check ledger challenge matches current head.
        if cap.ledger_challenge != ledger.head() {
            return JmResult::Denied(format!(
                "Ledger challenge mismatch: cap={}, current={}",
                hash::hex(&cap.ledger_challenge),
                hash::hex(&ledger.head()),
            ));
        }

        // 4. Check nonce not replayed.
        if ledger.nonce_used(&cap.nonce) {
            return JmResult::Denied("Nonce already used (replay detected)".into());
        }

        // 5. Verify Ed25519 signature.
        let pk = match VerifyingKey::from_bytes(&self.artifact.pk_root) {
            Ok(pk) => pk,
            Err(e) => return JmResult::Denied(format!("Invalid pk_root: {}", e)),
        };

        let sig_bytes: [u8; 64] = match cap.signature.as_slice().try_into() {
            Ok(b) => b,
            Err(_) => return JmResult::Denied("Invalid signature length (expected 64 bytes)".into()),
        };

        let signature = Signature::from_bytes(&sig_bytes);
        let message = cap.signed_message();

        match pk.verify(&message, &signature) {
            Ok(()) => {
                // Record the nonce as used.
                ledger.use_nonce(cap.nonce);

                // Record the verification event.
                let event = Event::new(
                    EventKind::CapVerify,
                    &cap.ser_pi(),
                    vec![],
                    1,
                    0,
                );
                ledger.commit(event);

                JmResult::Authorized
            }
            Err(e) => JmResult::Denied(format!("Signature verification failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{SigningKey, Signer};
    use rand::rngs::OsRng;
    use kernel_types::HASH_ZERO;

    #[test]
    fn valid_capability_accepted() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let mut ledger = Ledger::new();
        let artifact = KernelArtifact::new(
            "v0.1.0".into(),
            verifying_key.to_bytes(),
        );

        let nonce = hash::H(b"test-nonce-1");
        let mut cap = Capability {
            scope: "override".into(),
            build_hash: HASH_ZERO,
            ledger_challenge: ledger.head(),
            nonce,
            signature: vec![],
        };

        // Sign the capability.
        let message = cap.signed_message();
        let sig = signing_key.sign(&message);
        cap.signature = sig.to_bytes().to_vec();

        let checker = JurisdictionChecker::new(artifact);
        match checker.verify(&cap, &mut ledger) {
            JmResult::Authorized => {} // expected
            JmResult::Denied(msg) => panic!("Expected Authorized, got Denied: {}", msg),
        }
    }

    #[test]
    fn replay_denied() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let mut ledger = Ledger::new();
        let artifact = KernelArtifact::new(
            "v0.1.0".into(),
            verifying_key.to_bytes(),
        );

        let nonce = hash::H(b"test-nonce-replay");
        let mut cap = Capability {
            scope: "override".into(),
            build_hash: HASH_ZERO,
            ledger_challenge: ledger.head(),
            nonce,
            signature: vec![],
        };

        let message = cap.signed_message();
        let sig = signing_key.sign(&message);
        cap.signature = sig.to_bytes().to_vec();

        let checker = JurisdictionChecker::new(artifact);

        // First use: should succeed.
        match checker.verify(&cap, &mut ledger) {
            JmResult::Authorized => {}
            JmResult::Denied(msg) => panic!("First use should succeed: {}", msg),
        }

        // Second use: ledger head changed, so challenge won't match.
        // But even if we fix the challenge, the nonce should be replayed.
        cap.ledger_challenge = ledger.head();
        let message2 = cap.signed_message();
        let sig2 = signing_key.sign(&message2);
        cap.signature = sig2.to_bytes().to_vec();

        match checker.verify(&cap, &mut ledger) {
            JmResult::Denied(msg) => {
                assert!(msg.contains("replay") || msg.contains("Nonce"), "Expected replay denial, got: {}", msg);
            }
            JmResult::Authorized => panic!("Replay should be denied"),
        }
    }

    #[test]
    fn wrong_scope_denied() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();

        let mut ledger = Ledger::new();
        let artifact = KernelArtifact::new(
            "v0.1.0".into(),
            verifying_key.to_bytes(),
        );

        let nonce = hash::H(b"test-nonce-scope");
        let mut cap = Capability {
            scope: "delete_everything".into(), // not in allow_scopes
            build_hash: HASH_ZERO,
            ledger_challenge: ledger.head(),
            nonce,
            signature: vec![],
        };

        let message = cap.signed_message();
        let sig = signing_key.sign(&message);
        cap.signature = sig.to_bytes().to_vec();

        let checker = JurisdictionChecker::new(artifact);
        match checker.verify(&cap, &mut ledger) {
            JmResult::Denied(msg) => {
                assert!(msg.contains("not in allow list"));
            }
            JmResult::Authorized => panic!("Wrong scope should be denied"),
        }
    }
}
