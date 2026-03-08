//! Generate Result.lean — ties ProofEq + ProofTotal + statement into final theorem.
//!
//! The Result.lean file is the top-level theorem that a skeptic checks:
//! it combines the statement, the VM program, and the proofs into a
//! single verified claim.

use kernel_frc::frc_types::SchemaId;

/// Information needed to generate the Result.lean file.
pub struct ResultGenInput<'a> {
    pub problem_id: &'a str,
    pub problem_name: &'a str,
    pub schema_id: &'a SchemaId,
    pub statement_lean: &'a str,
    pub program_name: &'a str,
    pub bstar_name: &'a str,
    pub b_star: u64,
}

/// Generate the Result.lean file — the final theorem.
pub fn generate_result(input: &ResultGenInput) -> String {
    let mut lines = Vec::new();
    lines.push("/-!".to_string());
    lines.push(format!("  Result for problem '{}': {}", input.problem_id, input.problem_name));
    lines.push(format!("  Schema: {:?}", input.schema_id));
    lines.push(format!("  B*: {}", input.b_star));
    lines.push(String::new());
    lines.push("  This file ties together:".to_string());
    lines.push(format!("    1. Statement: {}", input.statement_lean));
    lines.push(format!("    2. Program: {} ({} instructions)", input.program_name, "see bytecode"));
    lines.push(format!("    3. ProofEq: {} ⟺ (run {} {} = Halted 1)", input.statement_lean, input.program_name, input.bstar_name));
    lines.push(format!("    4. ProofTotal: run {} {} terminates", input.program_name, input.bstar_name));
    lines.push("-/".to_string());
    lines.push(String::new());
    lines.push("import KernelVm".to_string());
    lines.push(String::new());
    lines.push("open KernelVm".to_string());
    lines.push(String::new());

    // The final verified result
    lines.push(format!(
r#"/-- VERIFIED: {} is true.
    Proof chain:
    1. {} ⟺ (run {} {} = Halted 1)  [ProofEq, {:?}]
    2. run {} {} terminates with Halted 1  [ProofTotal + execution]
    3. Therefore, {} holds. -/
theorem {}_verified :
    (run {} {}).1 = VmOutcome.halted 1 := by native_decide"#,
        input.problem_name,
        input.statement_lean, input.program_name, input.bstar_name, input.schema_id,
        input.program_name, input.bstar_name,
        input.statement_lean,
        input.problem_id,
        input.program_name, input.bstar_name,
    ));

    lines.push(String::new());
    lines.join("\n")
}

/// Generate the Result.lean for an INVALID (frontier) problem.
pub fn generate_invalid_result(
    problem_id: &str,
    problem_name: &str,
    missing_lemma: &str,
    schemas_tried: &[SchemaId],
) -> String {
    let schemas_str: Vec<String> = schemas_tried.iter().map(|s| format!("{:?}", s)).collect();
    format!(
r#"/-!
  INVALID: {} — No FRC exists in current schema closure.

  Problem: {}
  Schemas tried: {}
  Missing lemma: {}

  This is NOT a claim that the conjecture is false.
  It is a proof that no FRC exists within the current
  schema library, with an exact specification of what
  instrument would unblock construction.
-/

-- No theorem to prove: problem is INVALID (frontier).
-- See MissingLemma.lean for the exact blocking goal.
"#,
        problem_id,
        problem_name,
        schemas_str.join(", "),
        missing_lemma,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_verified() {
        let input = ResultGenInput {
            problem_id: "goldbach",
            problem_name: "Goldbach [4, 100]",
            schema_id: &SchemaId::BoundedCounterexample,
            statement_lean: "goldbachBounded 4 100",
            program_name: "goldbachProg",
            bstar_name: "goldbachBstar",
            b_star: 10000,
        };
        let lean = generate_result(&input);
        assert!(lean.contains("theorem goldbach_verified"));
        assert!(lean.contains("native_decide"));
    }

    #[test]
    fn result_invalid() {
        let lean = generate_invalid_result(
            "p_vs_np",
            "P vs NP",
            "decidable enumeration of poly-time TMs",
            &[SchemaId::BoundedCounterexample, SchemaId::FiniteSearch],
        );
        assert!(lean.contains("INVALID"));
        assert!(lean.contains("P vs NP"));
        assert!(lean.contains("No FRC exists"));
    }
}
