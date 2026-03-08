//! Invoke `lake build` and parse results.
//!
//! This module provides the interface between the Rust proof bundle
//! and the Lean4 type checker. It invokes `lake build` in the lean/
//! directory and reports success/failure.

use std::path::Path;
use std::process::Command;

/// Result of running `lake build`.
#[derive(Debug)]
pub struct LeanBuildResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

/// Run `lake build` in the given Lean4 project directory.
///
/// Returns the build result. If Lean4 is not installed,
/// returns a result with `success = false` and a helpful error message.
pub fn run_lake_build(lean_dir: &Path) -> LeanBuildResult {
    // Check if lake is available
    let lake_check = Command::new("lake")
        .arg("--version")
        .output();

    if lake_check.is_err() {
        return LeanBuildResult {
            success: false,
            stdout: String::new(),
            stderr: "Lean4/lake not found. Install from https://leanprover.github.io/lean4/doc/setup.html".to_string(),
            exit_code: None,
        };
    }

    // Run lake build
    let output = Command::new("lake")
        .arg("build")
        .current_dir(lean_dir)
        .output();

    match output {
        Ok(out) => LeanBuildResult {
            success: out.status.success(),
            stdout: String::from_utf8_lossy(&out.stdout).to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).to_string(),
            exit_code: out.status.code(),
        },
        Err(e) => LeanBuildResult {
            success: false,
            stdout: String::new(),
            stderr: format!("Failed to execute lake build: {}", e),
            exit_code: None,
        },
    }
}

/// Check if any Lean files in a directory contain `sorry` as a tactic
/// (not inside comments or string literals).
pub fn check_no_sorry(lean_dir: &Path) -> Vec<String> {
    let mut sorry_files = Vec::new();

    fn has_sorry_tactic(content: &str) -> bool {
        let mut in_block_comment = false;
        for line in content.lines() {
            let trimmed = line.trim();

            // Track block comments /-! ... -/ and /- ... -/
            if trimmed.contains("/-") {
                in_block_comment = true;
            }
            if trimmed.contains("-/") {
                in_block_comment = false;
                continue;
            }
            if in_block_comment {
                continue;
            }

            // Skip line comments
            let code_part = if let Some(idx) = trimmed.find("--") {
                &trimmed[..idx]
            } else {
                trimmed
            };

            // Skip string literals (simple heuristic: inside quotes)
            let code_no_strings = remove_string_literals(code_part);

            // Check for sorry as a standalone word
            if contains_word(&code_no_strings, "sorry") {
                return true;
            }
        }
        false
    }

    fn remove_string_literals(s: &str) -> String {
        let mut result = String::new();
        let mut in_string = false;
        let mut prev = '\0';
        for c in s.chars() {
            if c == '"' && prev != '\\' {
                in_string = !in_string;
            } else if !in_string {
                result.push(c);
            }
            prev = c;
        }
        result
    }

    fn contains_word(s: &str, word: &str) -> bool {
        for (i, _) in s.match_indices(word) {
            let before_ok = i == 0 || !s.as_bytes()[i - 1].is_ascii_alphanumeric();
            let after_idx = i + word.len();
            let after_ok = after_idx >= s.len() || !s.as_bytes()[after_idx].is_ascii_alphanumeric();
            if before_ok && after_ok {
                return true;
            }
        }
        false
    }

    fn walk_dir(dir: &Path, sorry_files: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk_dir(&path, sorry_files);
                } else if path.extension().map(|e| e == "lean").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if has_sorry_tactic(&content) {
                            sorry_files.push(path.display().to_string());
                        }
                    }
                }
            }
        }
    }

    walk_dir(lean_dir, &mut sorry_files);
    sorry_files
}

/// Full verification pipeline: build + sorry check.
pub fn verify_lean_proofs(lean_dir: &Path) -> LeanVerifyResult {
    let sorry_files = check_no_sorry(lean_dir);
    let build = run_lake_build(lean_dir);
    let no_sorry = sorry_files.is_empty();
    let pass = build.success && no_sorry;

    LeanVerifyResult {
        build_success: build.success,
        no_sorry,
        sorry_files,
        build_stdout: build.stdout,
        build_stderr: build.stderr,
        pass,
    }
}

/// Result of full Lean verification.
#[derive(Debug)]
pub struct LeanVerifyResult {
    pub build_success: bool,
    pub no_sorry: bool,
    pub sorry_files: Vec<String>,
    pub build_stdout: String,
    pub build_stderr: String,
    pub pass: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorry_check_no_files() {
        let dir = std::env::temp_dir().join("test_no_sorry");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("test.lean"), "theorem t : True := trivial").unwrap();
        let result = check_no_sorry(&dir);
        assert!(result.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sorry_check_found() {
        let dir = std::env::temp_dir().join("test_has_sorry");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("bad.lean"), "theorem t : False := sorry").unwrap();
        let result = check_no_sorry(&dir);
        assert_eq!(result.len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
