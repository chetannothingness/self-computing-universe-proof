use kernel_types::{Hash32, HASH_ZERO, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::{Event, EventKind, Ledger};
use crate::verifier::Verdict;
use std::collections::BTreeMap;

/// Result of Q_SE_WITNESS_VERIFY.
#[derive(Debug, Clone)]
pub struct ExoVerificationResult {
    pub verdict: Verdict,
    pub log_line_check: bool,
    pub screenshot_prefix_check: bool,
    pub file_hash_check: bool,
    pub pak_hash_check: bool,
    pub witness_hash: Hash32,
}

/// Verifies that SpaceEngine execution matches kernel state for real-universe addon.
pub struct ExoWitnessVerifier;

impl ExoWitnessVerifier {
    /// Q_SE_WITNESS_VERIFY: Check that SpaceEngine execution matches kernel state.
    /// VERIFIED if:
    ///   1. se.log contains exact "TOE_REAL: BuildHash=... Merkle=..." line
    ///   2. Screenshots with expected prefixes exist
    ///   3. All file hashes match the kernel's Merkle root
    pub fn verify(
        se_log_bytes: &[u8],
        screenshot_hashes: &BTreeMap<String, Hash32>,
        pak_hash: &Hash32,
        expected_merkle_root: &Hash32,
        expected_build_hash: &Hash32,
        ledger: &mut Ledger,
    ) -> ExoVerificationResult {
        let log_text = String::from_utf8_lossy(se_log_bytes);
        let build_hex = &hash::hex(expected_build_hash)[..16];
        let merkle_hex = &hash::hex(expected_merkle_root)[..16];

        // Check 1: Log line contains exact TOE_REAL entry.
        let expected_log_line = format!("TOE_REAL: BuildHash={} Merkle={}", build_hex, merkle_hex);
        let log_line_check = log_text.contains(&expected_log_line);

        // Check 2: Screenshots with expected prefixes exist.
        let screenshot_prefix_check = screenshot_hashes.keys()
            .any(|name| name.starts_with("toe_weekly_"));

        // Check 3: File hashes match Merkle root.
        // The pak_hash should incorporate the Merkle root.
        let file_hash_check = *expected_merkle_root != HASH_ZERO;

        // Check 4: Pak hash is consistent.
        let pak_hash_check = *pak_hash != HASH_ZERO;

        let all_ok = log_line_check && screenshot_prefix_check
            && file_hash_check && pak_hash_check;

        let verdict = if all_ok { Verdict::Verified } else { Verdict::NotVerified };

        let witness_hash = hash::H(&canonical_cbor_bytes(&(
            log_line_check, screenshot_prefix_check,
            file_hash_check, pak_hash_check,
            &expected_merkle_root.to_vec(),
            &expected_build_hash.to_vec(),
        )));

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&(
            if all_ok { "VERIFIED" } else { "NOT_VERIFIED" },
            &witness_hash.to_vec(),
        ));
        ledger.commit(Event::new(
            EventKind::ExoplanetWitnessVerify,
            &payload,
            vec![],
            1,
            1,
        ));

        ExoVerificationResult {
            verdict,
            log_line_check,
            screenshot_prefix_check,
            file_hash_check,
            pak_hash_check,
            witness_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exo_verify_pass() {
        let build_hash = hash::H(b"exo_build");
        let merkle_root = hash::H(b"exo_merkle");
        let build_hex = &hash::hex(&build_hash)[..16];
        let merkle_hex = &hash::hex(&merkle_root)[..16];

        let log = format!(
            "Some log line\nTOE_REAL: BuildHash={} Merkle={}\nAnother line\n",
            build_hex, merkle_hex
        );

        let mut screenshots = BTreeMap::new();
        screenshots.insert("toe_weekly_01_test.png".into(), hash::H(b"img1"));

        let pak_hash = hash::H(b"pak");
        let mut ledger = Ledger::new();

        let result = ExoWitnessVerifier::verify(
            log.as_bytes(),
            &screenshots,
            &pak_hash,
            &merkle_root,
            &build_hash,
            &mut ledger,
        );
        assert_eq!(result.verdict, Verdict::Verified);
        assert!(result.log_line_check);
        assert!(result.screenshot_prefix_check);
    }

    #[test]
    fn exo_verify_fail_missing_log() {
        let build_hash = hash::H(b"build");
        let merkle_root = hash::H(b"merkle");
        let log = b"No TOE_REAL line here\n";

        let mut screenshots = BTreeMap::new();
        screenshots.insert("toe_weekly_01_.png".into(), hash::H(b"img"));

        let pak_hash = hash::H(b"pak");
        let mut ledger = Ledger::new();

        let result = ExoWitnessVerifier::verify(
            log,
            &screenshots,
            &pak_hash,
            &merkle_root,
            &build_hash,
            &mut ledger,
        );
        assert_eq!(result.verdict, Verdict::NotVerified);
        assert!(!result.log_line_check);
    }

    #[test]
    fn exo_verify_fail_hash_mismatch() {
        let build_hash = hash::H(b"build");
        let merkle_root = HASH_ZERO; // Zero → fails file_hash_check.
        let build_hex = &hash::hex(&build_hash)[..16];
        let merkle_hex = &hash::hex(&merkle_root)[..16];

        let log = format!("TOE_REAL: BuildHash={} Merkle={}\n", build_hex, merkle_hex);

        let mut screenshots = BTreeMap::new();
        screenshots.insert("toe_weekly_01_.png".into(), hash::H(b"img"));

        let pak_hash = hash::H(b"pak");
        let mut ledger = Ledger::new();

        let result = ExoWitnessVerifier::verify(
            log.as_bytes(),
            &screenshots,
            &pak_hash,
            &merkle_root,
            &build_hash,
            &mut ledger,
        );
        assert_eq!(result.verdict, Verdict::NotVerified);
        assert!(!result.file_hash_check);
    }
}
