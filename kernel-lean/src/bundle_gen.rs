//! Generate complete proof bundle: out/<ID>/ directory for each problem.
//!
//! For VERIFIED problems:
//!   out/<ID>/frc.json, C.bytecode, Bstar.txt, ProofEq.lean, ProofTotal.lean,
//!   ExecTrace.bin, Result.lean, Receipt.json
//!
//! For INVALID problems:
//!   out/<ID>/INVALID.json, MissingLemma.lean, DependencyGraph.json, Receipt.json

use std::path::{Path, PathBuf};
use std::fs;

use kernel_types::{Hash32, SerPi, hash};
use kernel_frc::frc_types::*;
use kernel_frc::vm::{Vm, VmOutcome, ExecTrace};

use crate::proof_eq_gen;
use crate::proof_total_gen;
use crate::result_gen::{self, ResultGenInput};
use crate::irc_gen;
use crate::irc_result_gen;

/// Emit a proof bundle for a single VERIFIED problem.
pub fn emit_verified_bundle(
    output_dir: &Path,
    problem_id: &str,
    problem_name: &str,
    frc: &Frc,
    trace: &ExecTrace,
) -> Result<PathBuf, String> {
    let dir = output_dir.join(problem_id);
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {}", e))?;

    // 1. frc.json
    let frc_json = serde_json::to_string_pretty(frc)
        .map_err(|e| format!("frc serialize: {}", e))?;
    fs::write(dir.join("frc.json"), &frc_json)
        .map_err(|e| format!("write frc.json: {}", e))?;

    // 2. C.bytecode — canonical CBOR of program
    let bytecode = frc.program.ser_pi();
    fs::write(dir.join("C.bytecode"), &bytecode)
        .map_err(|e| format!("write C.bytecode: {}", e))?;

    // 3. Bstar.txt
    fs::write(dir.join("Bstar.txt"), frc.b_star.to_string())
        .map_err(|e| format!("write Bstar.txt: {}", e))?;

    // 4. ProofEq.lean
    let program_name = format!("{}Prog", problem_id);
    let bstar_name = format!("{}Bstar", problem_id);
    let proof_eq_lean = proof_eq_gen::generate_proof_eq(
        &frc.proof_eq,
        &frc.schema_id,
        problem_id,
        &program_name,
        &bstar_name,
        &format!("{}Statement", problem_id),
    );
    fs::write(dir.join("ProofEq.lean"), &proof_eq_lean)
        .map_err(|e| format!("write ProofEq.lean: {}", e))?;

    // 5. ProofTotal.lean
    let proof_total_lean = proof_total_gen::generate_proof_total(
        &frc.proof_total,
        problem_id,
        &program_name,
        &bstar_name,
    );
    fs::write(dir.join("ProofTotal.lean"), &proof_total_lean)
        .map_err(|e| format!("write ProofTotal.lean: {}", e))?;

    // 6. ExecTrace.bin — serialized trace
    let trace_bytes = trace.ser_pi();
    fs::write(dir.join("ExecTrace.bin"), &trace_bytes)
        .map_err(|e| format!("write ExecTrace.bin: {}", e))?;

    // 7. Result.lean
    let result_lean = result_gen::generate_result(&ResultGenInput {
        problem_id,
        problem_name,
        schema_id: &frc.schema_id,
        statement_lean: &format!("{}Statement", problem_id),
        program_name: &program_name,
        bstar_name: &bstar_name,
        b_star: frc.b_star,
    });
    fs::write(dir.join("Result.lean"), &result_lean)
        .map_err(|e| format!("write Result.lean: {}", e))?;

    // 8. Receipt.json
    let execution_outcome = match &trace.outcome {
        VmOutcome::Halted(c) => *c,
        _ => 0,
    };
    let receipt = FrcReceipt::new(
        frc.frc_hash,
        execution_outcome,
        trace.trace_head,
        compute_bundle_hash(&dir),
        frc.statement_hash,
        true,
    );
    let receipt_json = serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("receipt serialize: {}", e))?;
    fs::write(dir.join("Receipt.json"), &receipt_json)
        .map_err(|e| format!("write Receipt.json: {}", e))?;

    Ok(dir)
}

/// Emit a proof bundle for a single INVALID (frontier) problem.
pub fn emit_invalid_bundle(
    output_dir: &Path,
    problem_id: &str,
    problem_name: &str,
    frontier: &FrontierWitness,
) -> Result<PathBuf, String> {
    let dir = output_dir.join(problem_id);
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {}", e))?;

    // 1. INVALID.json
    let invalid_json = serde_json::to_string_pretty(frontier)
        .map_err(|e| format!("frontier serialize: {}", e))?;
    fs::write(dir.join("INVALID.json"), &invalid_json)
        .map_err(|e| format!("write INVALID.json: {}", e))?;

    // 2. MissingLemma.lean — reference to the frontier file
    let missing = frontier.minimal_missing_lemma.as_ref()
        .map(|ml| ml.lemma_statement.as_str())
        .unwrap_or("No specific missing lemma identified");
    let _schemas_tried: Vec<String> = frontier.schemas_tried.iter()
        .map(|s| format!("{:?}", s))
        .collect();
    let result_lean = result_gen::generate_invalid_result(
        problem_id, problem_name, missing, &frontier.schemas_tried,
    );
    fs::write(dir.join("MissingLemma.lean"), &result_lean)
        .map_err(|e| format!("write MissingLemma.lean: {}", e))?;

    // 3. DependencyGraph.json
    let dep_graph = serde_json::json!({
        "nodes": [{
            "id": problem_id,
            "status": "INVALID",
            "missing": frontier.gaps.iter().map(|g| &g.goal_statement).collect::<Vec<_>>(),
        }],
        "edges": frontier.gaps.iter().map(|g| serde_json::json!({
            "from": g.goal_statement,
            "blocks": problem_id,
        })).collect::<Vec<_>>(),
    });
    fs::write(dir.join("DependencyGraph.json"),
        serde_json::to_string_pretty(&dep_graph).unwrap_or_default())
        .map_err(|e| format!("write DependencyGraph.json: {}", e))?;

    // 4. Receipt.json
    let receipt = FrcReceipt::new(
        frontier.frontier_hash,
        0,
        [0u8; 32],
        compute_bundle_hash(&dir),
        frontier.statement_hash,
        false,
    );
    let receipt_json = serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("receipt serialize: {}", e))?;
    fs::write(dir.join("Receipt.json"), &receipt_json)
        .map_err(|e| format!("write Receipt.json: {}", e))?;

    Ok(dir)
}

/// Emit IRC bundle files for a problem (adds to existing bundle directory).
pub fn emit_irc_bundle(
    output_dir: &Path,
    problem_id: &str,
    result: &kernel_frc::frc_types::IrcResult,
) -> Result<PathBuf, String> {
    let dir = output_dir.join(problem_id);
    fs::create_dir_all(&dir).map_err(|e| format!("mkdir: {}", e))?;

    match result {
        kernel_frc::frc_types::IrcResult::Proved(irc) => {
            // Invariant.lean — generated IRC invariant + obligations
            let inv_lean = irc_gen::generate_irc_lean(irc, problem_id);
            fs::write(dir.join("Invariant.lean"), &inv_lean)
                .map_err(|e| format!("write Invariant.lean: {}", e))?;

            // IrcResult.lean — final unbounded theorem
            let result_lean = irc_result_gen::generate_irc_result_proved(irc, problem_id);
            fs::write(dir.join("IrcResult.lean"), &result_lean)
                .map_err(|e| format!("write IrcResult.lean: {}", e))?;

            // irc.json — serialized IRC certificate
            let irc_json = serde_json::to_string_pretty(irc)
                .map_err(|e| format!("irc serialize: {}", e))?;
            fs::write(dir.join("irc.json"), &irc_json)
                .map_err(|e| format!("write irc.json: {}", e))?;
        }
        kernel_frc::frc_types::IrcResult::Frontier(frontier) => {
            // Invariant.lean — best candidate IRC with gaps documented
            if let Some(ref best) = frontier.best_candidate {
                let inv_lean = irc_gen::generate_irc_lean(best, problem_id);
                fs::write(dir.join("Invariant.lean"), &inv_lean)
                    .map_err(|e| format!("write Invariant.lean: {}", e))?;
            }

            // IrcResult.lean — frontier report
            let result_lean = irc_result_gen::generate_irc_result_frontier(frontier, problem_id);
            fs::write(dir.join("IrcResult.lean"), &result_lean)
                .map_err(|e| format!("write IrcResult.lean: {}", e))?;

            // irc_frontier.json — serialized frontier
            let frontier_json = serde_json::to_string_pretty(frontier)
                .map_err(|e| format!("frontier serialize: {}", e))?;
            fs::write(dir.join("irc_frontier.json"), &frontier_json)
                .map_err(|e| format!("write irc_frontier.json: {}", e))?;
        }
    }

    Ok(dir)
}

/// Compute the blake3 hash of all files in a directory (sorted by name).
fn compute_bundle_hash(dir: &Path) -> Hash32 {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map(|rd| rd.filter_map(|e| e.ok()).collect())
        .unwrap_or_default();
    entries.sort_by_key(|e| e.file_name());

    let mut hasher_input = Vec::new();
    for entry in &entries {
        if let Ok(content) = fs::read(entry.path()) {
            hasher_input.extend_from_slice(entry.file_name().to_string_lossy().as_bytes());
            hasher_input.extend_from_slice(&content);
        }
    }
    hash::H(&hasher_input)
}

/// Verify a proof bundle directory.
pub fn verify_bundle(bundle_dir: &Path) -> BundleVerifyResult {
    let mut result = BundleVerifyResult::default();

    // Check Receipt.json exists
    let receipt_path = bundle_dir.join("Receipt.json");
    if !receipt_path.exists() {
        result.errors.push("Missing Receipt.json".to_string());
        return result;
    }

    // Check if VERIFIED or INVALID
    let is_invalid = bundle_dir.join("INVALID.json").exists();
    result.is_invalid = is_invalid;

    if is_invalid {
        // Verify INVALID bundle
        if !bundle_dir.join("MissingLemma.lean").exists() {
            result.errors.push("Missing MissingLemma.lean".to_string());
        }
        if !bundle_dir.join("DependencyGraph.json").exists() {
            result.errors.push("Missing DependencyGraph.json".to_string());
        }
    } else {
        // Verify VERIFIED bundle
        for file in &["frc.json", "C.bytecode", "Bstar.txt", "ProofEq.lean",
                      "ProofTotal.lean", "ExecTrace.bin", "Result.lean"] {
            if !bundle_dir.join(file).exists() {
                result.errors.push(format!("Missing {}", file));
            }
        }

        // Verify FRC internal consistency
        if let Ok(frc_content) = fs::read_to_string(bundle_dir.join("frc.json")) {
            if let Ok(frc) = serde_json::from_str::<Frc>(&frc_content) {
                result.frc_verified = frc.verify_internal();
                if !result.frc_verified {
                    result.errors.push("FRC internal verification failed".to_string());
                }

                // Re-execute VM and check outcome
                let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                result.vm_halted_1 = matches!(outcome, VmOutcome::Halted(1));
                if !result.vm_halted_1 {
                    result.errors.push(format!("VM did not halt with code 1: {:?}", outcome));
                }
            } else {
                result.errors.push("Failed to parse frc.json".to_string());
            }
        }

        // Check for sorry in Lean files
        for lean_file in &["ProofEq.lean", "ProofTotal.lean", "Result.lean"] {
            if let Ok(content) = fs::read_to_string(bundle_dir.join(lean_file)) {
                if content.contains("sorry") {
                    result.has_sorry = true;
                    result.errors.push(format!("{} contains 'sorry'", lean_file));
                }
            }
        }
    }

    // Recompute bundle hash
    let recomputed = compute_bundle_hash(bundle_dir);
    result.bundle_hash = recomputed;
    result.pass = result.errors.is_empty();

    result
}

/// Result of verifying a proof bundle.
#[derive(Debug, Default)]
pub struct BundleVerifyResult {
    pub pass: bool,
    pub is_invalid: bool,
    pub frc_verified: bool,
    pub vm_halted_1: bool,
    pub has_sorry: bool,
    pub lean_checked: bool,
    pub bundle_hash: Hash32,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_bundle_hash_deterministic() {
        let dir = std::env::temp_dir().join("test_bundle_hash");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("a.txt"), b"hello").unwrap();
        fs::write(dir.join("b.txt"), b"world").unwrap();
        let h1 = compute_bundle_hash(&dir);
        let h2 = compute_bundle_hash(&dir);
        assert_eq!(h1, h2);
        let _ = fs::remove_dir_all(&dir);
    }
}
