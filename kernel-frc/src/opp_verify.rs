// OPP Verifier — verifies FRC results.
//
// Verification pipeline:
//   1. Verify FRC internal consistency (hashes, proof bindings)
//   2. Verify VM trace consistency and hash chain
//   3. Verify FRC proofs bind S ↔ run(C,B*)=1
//   4. Verify Merkle root matches manifest
//   5. Re-execute VM and compare outcome

use kernel_types::{SerPi, hash};
use kernel_ledger::{Ledger, Event, EventKind};
use crate::frc_types::*;
use crate::vm::{Vm, VmOutcome};

/// Verification result.
#[derive(Debug, Clone)]
pub struct VerificationResult {
    pub frc_internal_ok: bool,
    pub trace_ok: bool,
    pub proof_binding_ok: bool,
    pub merkle_ok: bool,
    pub reexecution_ok: bool,
    pub overall: bool,
    pub details: String,
}

/// The OPP verifier.
pub struct OppVerifier;

impl OppVerifier {
    /// Verify an FRC + receipt against the original OPP.
    pub fn verify(
        opp: &OpenProblemPackage,
        frc: &Frc,
        receipt: &FrcReceipt,
        ledger: &mut Ledger,
    ) -> VerificationResult {
        // 1. Verify FRC internal consistency
        let frc_internal_ok = frc.verify_internal();

        // 2. Re-execute and verify trace
        let trace = Vm::run_traced(&frc.program, frc.b_star);
        let trace_ok = Vm::verify_trace(&frc.program, &trace);

        // 3. Verify proof bindings: S ↔ run(C,B*)=1
        let proof_binding_ok =
            frc.proof_eq.statement_hash == opp.opp_hash
            && frc.proof_eq.program_hash == frc.program.ser_pi_hash()
            && frc.proof_eq.b_star == frc.b_star
            && frc.proof_total.program_hash == frc.program.ser_pi_hash()
            && frc.proof_total.b_star == frc.b_star;

        // 4. Verify Merkle root
        let expected_merkle = hash::merkle_root(&[
            opp.opp_hash,
            frc.frc_hash,
            trace.trace_head,
        ]);
        let merkle_ok = expected_merkle == receipt.merkle_root;

        // 5. Verify re-execution matches receipt
        let reexecution_outcome = match &trace.outcome {
            VmOutcome::Halted(code) => *code,
            _ => 255,
        };
        let reexecution_ok = reexecution_outcome == receipt.execution_outcome
            && trace.trace_head == receipt.trace_head;

        let overall = frc_internal_ok && trace_ok && proof_binding_ok
            && merkle_ok && reexecution_ok;

        let details = if overall {
            format!(
                "VERIFIED: FRC for {} → outcome={}, {} steps",
                kernel_types::hash::hex(&opp.opp_hash)[..16].to_string(),
                receipt.execution_outcome,
                trace.total_steps,
            )
        } else {
            let mut failures = Vec::new();
            if !frc_internal_ok { failures.push("FRC internal"); }
            if !trace_ok { failures.push("trace"); }
            if !proof_binding_ok { failures.push("proof binding"); }
            if !merkle_ok { failures.push("merkle"); }
            if !reexecution_ok { failures.push("reexecution"); }
            format!("FAIL: {}", failures.join(", "))
        };

        ledger.commit(Event::new(
            EventKind::OppVerifyComplete,
            &hash::H(details.as_bytes()),
            vec![ledger.head()],
            1,
            if overall { 1 } else { 0 },
        ));

        VerificationResult {
            frc_internal_ok,
            trace_ok,
            proof_binding_ok,
            merkle_ok,
            reexecution_ok,
            overall,
            details,
        }
    }

    /// Verify a frontier witness (INVALID result).
    pub fn verify_frontier(
        opp: &OpenProblemPackage,
        frontier: &FrontierWitness,
    ) -> bool {
        // Frontier must reference the correct statement
        if frontier.statement_hash != opp.opp_hash {
            return false;
        }

        // Must have tried at least one schema
        if frontier.schemas_tried.is_empty() {
            return false;
        }

        // Frontier hash must be internally consistent
        let recomputed = FrontierWitness::new(
            frontier.statement_hash,
            frontier.schemas_tried.clone(),
            frontier.gaps.clone(),
            frontier.minimal_missing_lemma.clone(),
        );
        recomputed.frontier_hash == frontier.frontier_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::opp::OppRunner;

    fn make_opp_and_solve() -> (OpenProblemPackage, Frc, FrcReceipt) {
        let opp = OpenProblemPackage::new(
            "exists x in [0,10]: x > 0".to_string(),
            "".to_string(),
            TargetClass {
                allowed_schemas: vec![SchemaId::FiniteSearch],
                grammar_description: "first-order".to_string(),
            },
            AllowedPrimitives {
                max_vm_steps: 100_000,
                max_memory_slots: 256,
                cost_model: "unit".to_string(),
            },
            ExpectedOutput::Either,
        );

        let mut runner = OppRunner::new();
        let mut ledger = Ledger::new();
        match runner.solve(&opp, &mut ledger) {
            crate::opp::OppResult::Proof { frc, receipt } => (opp, frc, receipt),
            other => panic!("Expected Proof, got {:?}", other),
        }
    }

    #[test]
    fn verify_valid_frc() {
        let (opp, frc, receipt) = make_opp_and_solve();
        let mut ledger = Ledger::new();

        let result = OppVerifier::verify(&opp, &frc, &receipt, &mut ledger);
        assert!(result.overall, "Verification failed: {}", result.details);
        assert!(result.frc_internal_ok);
        assert!(result.trace_ok);
        assert!(result.proof_binding_ok);
        assert!(result.merkle_ok);
        assert!(result.reexecution_ok);
    }

    #[test]
    fn verify_rejects_tampered_receipt() {
        let (opp, frc, mut receipt) = make_opp_and_solve();
        receipt.execution_outcome = 99; // tamper
        let mut ledger = Ledger::new();

        let result = OppVerifier::verify(&opp, &frc, &receipt, &mut ledger);
        assert!(!result.overall);
        assert!(!result.reexecution_ok);
    }

    #[test]
    fn verify_rejects_wrong_merkle() {
        let (opp, frc, mut receipt) = make_opp_and_solve();
        receipt.merkle_root = hash::H(b"wrong");
        let mut ledger = Ledger::new();

        let result = OppVerifier::verify(&opp, &frc, &receipt, &mut ledger);
        assert!(!result.overall);
        assert!(!result.merkle_ok);
    }

    #[test]
    fn verify_frontier() {
        let opp = OpenProblemPackage::new(
            "forall x: undecidable".to_string(),
            "".to_string(),
            TargetClass {
                allowed_schemas: vec![],
                grammar_description: "".to_string(),
            },
            AllowedPrimitives {
                max_vm_steps: 100_000,
                max_memory_slots: 256,
                cost_model: "unit".to_string(),
            },
            ExpectedOutput::Either,
        );

        let frontier = FrontierWitness::new(
            opp.opp_hash,
            vec![SchemaId::FiniteSearch],
            vec![],
            None,
        );

        assert!(OppVerifier::verify_frontier(&opp, &frontier));
    }

    #[test]
    fn verify_frontier_rejects_wrong_hash() {
        let opp = OpenProblemPackage::new(
            "test".to_string(),
            "".to_string(),
            TargetClass { allowed_schemas: vec![], grammar_description: "".to_string() },
            AllowedPrimitives { max_vm_steps: 1000, max_memory_slots: 64, cost_model: "unit".to_string() },
            ExpectedOutput::Either,
        );

        let frontier = FrontierWitness::new(
            hash::H(b"wrong_statement"), // wrong hash
            vec![SchemaId::FiniteSearch],
            vec![],
            None,
        );

        assert!(!OppVerifier::verify_frontier(&opp, &frontier));
    }

    #[test]
    fn verify_emits_ledger_event() {
        let (opp, frc, receipt) = make_opp_and_solve();
        let mut ledger = Ledger::new();

        OppVerifier::verify(&opp, &frc, &receipt, &mut ledger);
        assert!(ledger.len() >= 1);
    }
}
