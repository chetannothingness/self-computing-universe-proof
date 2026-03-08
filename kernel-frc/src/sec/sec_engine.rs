//! Main SEC (Self-Extending Calculus) engine.
//!
//! The SEC engine is lazy: it only mines new rules when a Gap is encountered.
//! Flow: enumerate candidates → generate Lean → run `lake build` →
//! check_no_sorry → accept/reject → add to RuleDb → retry structural check.

use std::path::Path;
use kernel_types::{Hash32, hash};

use crate::invsyn::ast::Expr;
use crate::frc_types::ObligationKind;
use super::rule_syn::RuleSyn;
use super::rule_db::{RuleDb, ProvenRule};
use super::rule_enum::enumerate_for_gap;
use super::rule_lean_gen::{generate_soundness_file, theorem_name_for_rule};

/// A gap target that SEC will attempt to close.
#[derive(Debug, Clone)]
pub struct GapTarget {
    /// Hash of the gap.
    pub gap_hash: Hash32,
    /// Human-readable statement of the gap.
    pub gap_statement: String,
    /// Which obligation is failing.
    pub obligation_kind: ObligationKind,
    /// Problem identifier.
    pub problem_id: String,
    /// The invariant expression (if available).
    pub inv_expr: Option<Expr>,
    /// The property expression (if available).
    pub prop_expr: Option<Expr>,
    /// The step delta.
    pub delta: i64,
}

/// Result of SEC mining.
#[derive(Debug)]
pub enum SecResult {
    /// New rules were discovered and added to the database.
    NewRules(Vec<ProvenRule>),
    /// No new rules found after trying all candidates.
    NoNewRules { candidates_tried: usize },
}

/// The SEC engine.
pub struct SecEngine {
    /// The rule database (persistent, Merkle-committed).
    pub rule_db: RuleDb,
    /// Maximum rule size to enumerate.
    pub max_rule_size: usize,
    /// Path to the Lean project directory.
    pub lean_dir: Option<String>,
    /// Current epoch for rule discovery timestamps.
    epoch: u64,
}

impl SecEngine {
    /// Create a new SEC engine.
    pub fn new() -> Self {
        Self {
            rule_db: RuleDb::new(),
            max_rule_size: 3,
            lean_dir: None,
            epoch: 0,
        }
    }

    /// Create a new SEC engine with a Lean project directory.
    pub fn with_lean_dir(lean_dir: &str) -> Self {
        Self {
            rule_db: RuleDb::new(),
            max_rule_size: 3,
            lean_dir: Some(lean_dir.to_string()),
            epoch: 0,
        }
    }

    /// Mine for new rules to close a gap.
    ///
    /// Enumerates candidate rules, generates Lean soundness proofs,
    /// verifies them, and adds accepted rules to the database.
    pub fn mine_for_gap(&mut self, gap: &GapTarget) -> SecResult {
        self.epoch += 1;

        // Enumerate candidates targeted at this gap
        let candidates = enumerate_for_gap(
            self.max_rule_size,
            gap.inv_expr.as_ref(),
            gap.prop_expr.as_ref(),
            gap.delta,
        );

        let mut new_rules = Vec::new();
        let mut tried = 0;

        for candidate in &candidates {
            tried += 1;

            // Skip rules we already have
            let candidate_hash = candidate.rule_hash();
            if self.rule_db.rules().iter().any(|r| r.rule_hash == candidate_hash) {
                continue;
            }

            // Generate Lean soundness file
            let (file_name, file_content) = generate_soundness_file(candidate);

            // Verify the soundness proof
            if let Some(ref lean_dir) = self.lean_dir {
                match verify_soundness_proof(lean_dir, &file_name, &file_content) {
                    SoundnessVerdict::Accepted(soundness_hash) => {
                        let theorem_name = theorem_name_for_rule(candidate);
                        let rule = ProvenRule::new(
                            candidate.clone(),
                            soundness_hash,
                            theorem_name,
                            self.epoch,
                        );
                        new_rules.push(rule.clone());
                        self.rule_db.add_rule(rule);
                    }
                    SoundnessVerdict::Rejected(reason) => {
                        // Rule failed Lean verification — discard silently.
                        // This is expected for many candidates.
                        let _ = reason;
                    }
                }
            } else {
                // No Lean directory configured — run in mock/offline mode.
                // In mock mode, we accept rules whose structure is self-evidently sound
                // (for testing purposes only).
                if is_self_evident(candidate) {
                    let soundness_hash = hash::H(file_content.as_bytes());
                    let theorem_name = theorem_name_for_rule(candidate);
                    let rule = ProvenRule::new(
                        candidate.clone(),
                        soundness_hash,
                        theorem_name,
                        self.epoch,
                    );
                    new_rules.push(rule.clone());
                    self.rule_db.add_rule(rule);
                }
            }
        }

        if new_rules.is_empty() {
            SecResult::NoNewRules { candidates_tried: tried }
        } else {
            SecResult::NewRules(new_rules)
        }
    }

    /// Get a reference to the rule database.
    pub fn rule_db(&self) -> &RuleDb {
        &self.rule_db
    }
}

/// Verdict from Lean soundness verification.
enum SoundnessVerdict {
    /// Lean accepted the proof (no sorry). Contains hash of the proof file.
    Accepted(Hash32),
    /// Lean rejected the proof. Contains the reason.
    Rejected(String),
}

/// Verify a soundness proof by writing the file and running `lake build`.
///
/// Implements sorry checking and Lean compilation directly (without depending
/// on kernel-lean crate, since kernel-frc is a dependency of kernel-lean).
fn verify_soundness_proof(lean_dir: &str, file_name: &str, content: &str) -> SoundnessVerdict {
    use std::process::Command;

    let lean_path = Path::new(lean_dir);
    let sec_dir = lean_path.join("KernelVm").join("SEC");

    // Ensure SEC directory exists
    if std::fs::create_dir_all(&sec_dir).is_err() {
        return SoundnessVerdict::Rejected("failed to create SEC directory".to_string());
    }

    // Write the soundness file
    let file_path = sec_dir.join(file_name);
    if std::fs::write(&file_path, content).is_err() {
        return SoundnessVerdict::Rejected("failed to write soundness file".to_string());
    }

    // Check for sorry BEFORE building (defense in depth)
    if check_content_for_sorry(content) {
        let _ = std::fs::remove_file(&file_path);
        return SoundnessVerdict::Rejected("sorry detected in generated file".to_string());
    }

    // Run lake build
    let output = Command::new("lake")
        .arg("build")
        .current_dir(lean_path)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let soundness_hash = hash::H(content.as_bytes());
                SoundnessVerdict::Accepted(soundness_hash)
            } else {
                // Clean up the failed file
                let _ = std::fs::remove_file(&file_path);
                let stderr = String::from_utf8_lossy(&out.stderr);
                SoundnessVerdict::Rejected(format!("lake build failed: {}", stderr))
            }
        }
        Err(e) => {
            let _ = std::fs::remove_file(&file_path);
            SoundnessVerdict::Rejected(format!("failed to run lake: {}", e))
        }
    }
}

/// Check if content contains `sorry` as a standalone tactic word.
fn check_content_for_sorry(content: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();
        // Skip comments
        if trimmed.starts_with("--") || trimmed.starts_with("/-") {
            continue;
        }
        // Check for sorry as a standalone word
        for (i, _) in trimmed.match_indices("sorry") {
            let before_ok = i == 0 || !trimmed.as_bytes()[i - 1].is_ascii_alphanumeric();
            let after_idx = i + 5;
            let after_ok = after_idx >= trimmed.len() || !trimmed.as_bytes()[after_idx].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

/// Check if a rule is self-evidently sound (for testing with `accept_self_evident` only).
///
/// This is NOT used in production. In production, Lean is the ONLY soundness
/// oracle. Without Lean verification, no rules are accepted.
fn is_self_evident(_rule: &RuleSyn) -> bool {
    // In production: Lean is the ONLY soundness oracle.
    // Without a Lean directory, no rules are accepted.
    // This prevents unsound rule application (e.g., applying
    // "ground expression preserved" to non-ground expressions).
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gap() -> GapTarget {
        GapTarget {
            gap_hash: hash::H(b"test_gap"),
            gap_statement: "∀n, I(n) → I(n+1)".to_string(),
            obligation_kind: ObligationKind::Step,
            problem_id: "test".to_string(),
            inv_expr: Some(Expr::Const(1)),
            prop_expr: Some(Expr::Const(1)),
            delta: 1,
        }
    }

    #[test]
    fn sec_engine_new() {
        let engine = SecEngine::new();
        assert!(engine.rule_db.is_empty());
        assert!(engine.lean_dir.is_none());
    }

    #[test]
    fn mine_without_lean_finds_no_rules() {
        // Without a Lean directory, SEC cannot verify soundness,
        // so no rules are accepted. Lean is the ONLY oracle.
        let mut engine = SecEngine::new();
        let gap = make_gap();
        let result = engine.mine_for_gap(&gap);
        match result {
            SecResult::NoNewRules { candidates_tried } => {
                assert!(candidates_tried > 0, "Should try candidates even without Lean");
                assert!(engine.rule_db.is_empty(), "No rules without Lean verification");
            }
            SecResult::NewRules(_) => {
                panic!("Without Lean, no rules should be accepted");
            }
        }
    }

    #[test]
    fn mine_idempotent() {
        let mut engine = SecEngine::new();
        let gap = make_gap();

        // Mine twice — both times should find no rules (no Lean)
        let _r1 = engine.mine_for_gap(&gap);
        let count_after_first = engine.rule_db.len();

        let _r2 = engine.mine_for_gap(&gap);
        let count_after_second = engine.rule_db.len();

        // Both should find nothing
        assert_eq!(count_after_first, 0);
        assert_eq!(count_after_second, 0);
    }

    #[test]
    fn gap_target_construction() {
        let gap = GapTarget {
            gap_hash: hash::H(b"goldbach_step"),
            gap_statement: "∀n≥4, even(n) → ∃p,q prime, n = p + q".to_string(),
            obligation_kind: ObligationKind::Step,
            problem_id: "goldbach".to_string(),
            inv_expr: None,
            prop_expr: None,
            delta: 2,
        };
        assert_eq!(gap.problem_id, "goldbach");
        assert_eq!(gap.delta, 2);
    }

    #[test]
    fn sec_result_new_rules() {
        let rule = ProvenRule::new(
            RuleSyn {
                kind: super::super::rule_syn::RuleKind::StepPreservation,
                arity: 0,
                premises: vec![],
                conclusion: super::super::rule_syn::RuleExpr::MetaVar(0),
                description: "test".to_string(),
                size: 1,
            },
            hash::H(b"proof"),
            "test_theorem".to_string(),
            1,
        );
        let result = SecResult::NewRules(vec![rule]);
        assert!(matches!(result, SecResult::NewRules(r) if r.len() == 1));
    }

    #[test]
    fn is_self_evident_requires_lean() {
        use super::super::rule_syn::{RuleSyn, RuleExpr, RuleKind};
        // Without Lean, no rule is self-evident — Lean is the ONLY oracle
        let rule = RuleSyn {
            kind: RuleKind::StepPreservation,
            arity: 1,
            premises: vec![RuleExpr::MetaVar(0)],
            conclusion: RuleExpr::StepPreserved(Box::new(RuleExpr::MetaVar(0)), 1),
            description: "ground".to_string(),
            size: 1,
        };
        assert!(!is_self_evident(&rule));

        let non_evident = RuleSyn {
            kind: RuleKind::AlgebraicIdentity,
            arity: 1,
            premises: vec![],
            conclusion: RuleExpr::MetaVar(0),
            description: "complex".to_string(),
            size: 3,
        };
        assert!(!is_self_evident(&non_evident));
    }
}
