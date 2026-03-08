//! Universal solver — connects witness enumerator to Lean checker.
//!
//! For a statement S (Lean Prop), the solver:
//!   1. Enumerates ALL finite byte strings via WitnessEnumerator
//!   2. Interprets each as a candidate proof script (UTF-8)
//!   3. Writes: `theorem proof : S := by <candidate>`
//!   4. Runs `lake build` (Lean type-checker)
//!   5. First PASS = PROVED(S, π). Otherwise continue.
//!
//! This is the ENGINE. The accelerator (IRC/UCert) is tried first as a fast path.
//! If the accelerator fails, this solver runs. It is complete: if a proof exists
//! as a finite byte string, the enumerator reaches it.

use std::path::Path;
use std::process::Command;
use kernel_types::{Hash32, hash};

use super::statement::ProofStatement;
use super::witness::WitnessEnumerator;

/// Result of solving a single statement.
#[derive(Debug, Clone)]
pub enum SolveResult {
    /// Proof found and verified by Lean.
    Proved {
        /// The statement that was proved.
        statement_id: String,
        /// The raw witness (byte string) that constitutes the proof.
        witness: Vec<u8>,
        /// The witness interpreted as UTF-8 (the tactic script).
        proof_script: String,
        /// Rank at which the witness was found.
        rank: u64,
        /// Hash of the proof file.
        proof_hash: Hash32,
    },
    /// Budget exhausted — no proof found within the given number of witnesses.
    Frontier {
        /// The statement that remains unproved.
        statement_id: String,
        /// How many witnesses were checked.
        witnesses_checked: u64,
        /// Maximum byte-string length reached.
        max_length_reached: usize,
    },
}

/// Solve a statement by universal witness enumeration.
///
/// Enumerates all byte strings, interprets each as a Lean tactic script,
/// checks via `lake build`. Returns on first acceptance or budget exhaustion.
pub fn solve_by_enumeration(
    lean_dir: &str,
    statement: &ProofStatement,
    max_witnesses: u64,
) -> SolveResult {
    let enumerator = WitnessEnumerator::new();
    let mut checked = 0u64;
    let mut max_len = 0usize;

    for (rank, witness) in enumerator {
        if checked >= max_witnesses {
            break;
        }

        max_len = max_len.max(witness.len());

        // Interpret witness as UTF-8. Non-UTF-8 → skip (not a valid Lean string).
        let script = match String::from_utf8(witness.clone()) {
            Ok(s) => s,
            Err(_) => {
                checked += 1;
                continue;
            }
        };

        // Skip empty scripts
        if script.trim().is_empty() {
            checked += 1;
            continue;
        }

        // Check via Lean
        match check_witness_lean(lean_dir, statement, &script, rank) {
            LeanVerdict::Accepted(proof_hash) => {
                return SolveResult::Proved {
                    statement_id: statement.id.clone(),
                    witness,
                    proof_script: script,
                    rank,
                    proof_hash,
                };
            }
            LeanVerdict::Rejected | LeanVerdict::Timeout => {
                checked += 1;
                continue;
            }
        }
    }

    SolveResult::Frontier {
        statement_id: statement.id.clone(),
        witnesses_checked: checked,
        max_length_reached: max_len,
    }
}

/// Lean checker verdict.
enum LeanVerdict {
    Accepted(Hash32),
    Rejected,
    Timeout,
}

/// Check a single witness (proof script) against a statement via Lean.
///
/// 1. Generate .lean file: `theorem proof : <prop> := by\n  <script>`
/// 2. Check for sorry (defense in depth)
/// 3. Write to lean/ProofEnum/<namespace>/
/// 4. Run `lake build`
/// 5. Return verdict
/// 6. Clean up on failure
fn check_witness_lean(
    lean_dir: &str,
    statement: &ProofStatement,
    script: &str,
    rank: u64,
) -> LeanVerdict {
    // Generate .lean file content
    let content = format!(
        "-- Universal Proof Enumerator witness (rank {})\n\
         {imports}\n\
         namespace {ns}\n\
         \n\
         theorem proof_{rank} : {prop} := by\n  \
         {script}\n\
         \n\
         end {ns}\n",
        rank,
        imports = if statement.lean_imports.is_empty() {
            String::new()
        } else {
            statement.lean_imports.iter()
                .map(|i| format!("import {}", i))
                .collect::<Vec<_>>()
                .join("\n") + "\n"
        },
        ns = statement.namespace,
        rank = rank,
        prop = statement.lean_prop,
        script = script,
    );

    // Defense in depth: reject if sorry is present
    if contains_sorry(&content) {
        return LeanVerdict::Rejected;
    }

    let lean_path = Path::new(lean_dir);
    let ns_path = statement.namespace.replace('.', "/");
    let proof_dir = lean_path.join(&ns_path);

    if std::fs::create_dir_all(&proof_dir).is_err() {
        return LeanVerdict::Rejected;
    }

    let file_name = format!("Witness_{}.lean", rank);
    let file_path = proof_dir.join(&file_name);
    if std::fs::write(&file_path, &content).is_err() {
        return LeanVerdict::Rejected;
    }

    // Run lake build
    let output = Command::new("lake")
        .arg("build")
        .current_dir(lean_path)
        .output();

    match output {
        Ok(out) => {
            if out.status.success() {
                let proof_hash = hash::H(content.as_bytes());
                LeanVerdict::Accepted(proof_hash)
            } else {
                let _ = std::fs::remove_file(&file_path);
                LeanVerdict::Rejected
            }
        }
        Err(_) => {
            let _ = std::fs::remove_file(&file_path);
            LeanVerdict::Rejected
        }
    }
}

/// Public wrapper: check if a proof script is accepted by Lean for a statement.
/// Used by the normalizer (engine.rs Phase 1.5) to try mined rules.
pub fn check_witness_lean_pub(
    lean_dir: &str,
    statement: &ProofStatement,
    script: &str,
    rank: u64,
) -> bool {
    matches!(check_witness_lean(lean_dir, statement, script, rank), LeanVerdict::Accepted(_))
}

/// Check if content contains `sorry` as a standalone word.
fn contains_sorry(content: &str) -> bool {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("--") || trimmed.starts_with("/-") {
            continue;
        }
        for (i, _) in trimmed.match_indices("sorry") {
            let before_ok = i == 0 || !trimmed.as_bytes()[i - 1].is_ascii_alphanumeric();
            let after_idx = i + 5;
            let after_ok =
                after_idx >= trimmed.len() || !trimmed.as_bytes()[after_idx].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proof_enum::statement::get_statement;

    #[test]
    fn sorry_detected() {
        assert!(contains_sorry("  sorry"));
        assert!(contains_sorry("theorem x := sorry"));
        assert!(!contains_sorry("-- sorry"));
        assert!(!contains_sorry("notsorry"));
    }

    #[test]
    fn solve_with_zero_budget_is_frontier() {
        let stmt = get_statement("goldbach");
        let result = solve_by_enumeration("/nonexistent", &stmt, 0);
        match result {
            SolveResult::Frontier { witnesses_checked, .. } => {
                assert_eq!(witnesses_checked, 0);
            }
            SolveResult::Proved { .. } => panic!("should be frontier with zero budget"),
        }
    }

    #[test]
    fn solve_with_small_budget_is_frontier() {
        // With a small budget and no Lean, all witnesses get rejected
        let stmt = get_statement("goldbach");
        let result = solve_by_enumeration("/nonexistent", &stmt, 100);
        match result {
            SolveResult::Frontier { witnesses_checked, .. } => {
                assert!(witnesses_checked <= 100);
            }
            SolveResult::Proved { .. } => panic!("should be frontier without Lean"),
        }
    }
}
