//! Generate ProofTotal.lean — Lean4 proof that run(C, B*) terminates.
//!
//! For each program, generates a Lean4 proof that `run prog b_star` terminates.
//! Since `run` is defined with fuel (structurally decreasing on Nat),
//! termination is guaranteed by construction. The proof uses `native_decide`
//! to verify the specific outcome.

use kernel_frc::frc_types::ProofTotal;

/// Generate a Lean4 proof of totality for the given program.
///
/// Returns the content of ProofTotal.lean.
pub fn generate_proof_total(
    proof_total: &ProofTotal,
    problem_id: &str,
    program_name: &str,
    bstar_name: &str,
) -> String {
    let mut lines = Vec::new();
    lines.push("import KernelVm".to_string());
    lines.push(format!("import Generated.{}.Program", problem_id));
    lines.push(format!("import Generated.{}.Bstar", problem_id));
    lines.push(String::new());
    lines.push("/-!".to_string());
    lines.push(format!("  ProofTotal for problem '{}': run {} {} terminates.", problem_id, program_name, bstar_name));
    lines.push(format!("  B*: {}", proof_total.b_star));
    lines.push(format!("  Halting argument: {}", proof_total.halting_argument));
    lines.push("-/".to_string());
    lines.push(String::new());
    lines.push("open KernelVm".to_string());
    lines.push(String::new());

    // Structural totality proof
    lines.push(format!(
r#"/-- ProofTotal ({problem_id}):
    `run` is total by construction: it uses `runLoop` which is
    structurally recursive on `fuel : Nat`. Lean's type checker
    verifies termination — no `partial` annotation, all cases covered.

    Additionally, the program halts with a specific exit code
    within B* steps, verified computationally. -/
theorem {problem_id}_total :
    ∃ c, (run {program_name} {bstar_name}).1 = VmOutcome.halted c := by
  exact ⟨1, by native_decide⟩

/-- The program completes in at most B* steps. -/
theorem {problem_id}_within_budget :
    (run {program_name} {bstar_name}).1 ≠ VmOutcome.budgetExhausted := by
  native_decide"#
    ));

    lines.push(String::new());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::hash;

    #[test]
    fn totality_proof_generation() {
        let pt = ProofTotal {
            program_hash: hash::H(b"prog"),
            b_star: 5000,
            halting_argument: "Loop runs N iterations, N=100".to_string(),
            proof_hash: hash::H(b"total"),
            lean_proof: None,
        };
        let lean = generate_proof_total(&pt, "goldbach", "goldbachProg", "goldbachBstar");
        assert!(lean.contains("theorem goldbach_total"));
        assert!(lean.contains("native_decide"));
        assert!(lean.contains("VmOutcome.halted c"));
        assert!(lean.contains("theorem goldbach_within_budget"));
    }
}
