// Millennium FRC Builder — constructs genuine FRCs for finite fragments of open problems.
//
// Each open problem that maps to integer arithmetic gets a real VM program
// that performs the finite verification. The FRC proves a real mathematical
// fact (e.g., "Goldbach holds for all even n in [4, 1000]").

use kernel_types::{SerPi, hash};
use kernel_ledger::{Ledger, Event, EventKind};
use kernel_contracts::contract::{Contract, EvalSpec};

use crate::frc_types::*;
use crate::open_problems;
use crate::vm::{Vm, VmOutcome};

/// Build a genuine FRC for a MillenniumFinite contract.
///
/// Flow:
/// 1. Match problem_id → call appropriate build_* from open_problems
/// 2. Get (Program, b_star, description)
/// 3. Construct ProofEq with honest reduction chain
/// 4. Construct ProofTotal from loop bound analysis
/// 5. Vm::run(program, b_star) → verify execution
/// 6. Return Frc
pub fn build_millennium_frc(
    contract: &Contract,
    ledger: &mut Ledger,
) -> Result<Frc, FrontierWitness> {
    let (problem_id, parameter_n, parameter_aux) = match &contract.eval {
        EvalSpec::MillenniumFinite { problem_id, parameter_n, parameter_aux } => {
            (problem_id.as_str(), *parameter_n, *parameter_aux)
        }
        _ => {
            return Err(FrontierWitness::new(
                contract.qid,
                vec![],
                vec![Gap {
                    goal_hash: contract.qid,
                    goal_statement: "Not a MillenniumFinite contract".to_string(),
                    schema_id: SchemaId::FiniteSearch,
                    dependency_hashes: vec![],
                    unresolved_bound: None,
                }],
                None,
            ));
        }
    };

    // Build the appropriate program
    let (program, b_star, description) = match problem_id {
        "goldbach" => open_problems::build_goldbach(parameter_n),
        "collatz" => {
            let max_iter = parameter_aux.unwrap_or(1000);
            open_problems::build_collatz(parameter_n, max_iter)
        }
        "twin_primes" => open_problems::build_twin_prime_search(parameter_n),
        "flt" => {
            let max_base = parameter_aux.unwrap_or(40);
            open_problems::build_flt(parameter_n, max_base)
        }
        "odd_perfect" => open_problems::build_odd_perfect(parameter_n),
        "mersenne" => open_problems::build_mersenne(parameter_n),
        "zfc_zero_ne_one" => open_problems::build_zero_ne_one(),
        "mertens" => open_problems::build_mertens(parameter_n),
        "legendre" => open_problems::build_legendre(parameter_n),
        "erdos_straus" => open_problems::build_erdos_straus(parameter_n),
        "bsd_ec_count" => {
            let curve_id = parameter_aux.unwrap_or(0);
            open_problems::build_bsd_ec_count(parameter_n, curve_id)
        }
        "weak_goldbach" => open_problems::build_weak_goldbach(parameter_n),
        "bertrand" => open_problems::build_bertrand(parameter_n),
        "lagrange_four_squares" => open_problems::build_lagrange_four_squares(parameter_n),
        _ => {
            return Err(FrontierWitness::new(
                contract.qid,
                vec![],
                vec![Gap {
                    goal_hash: contract.qid,
                    goal_statement: format!("Unknown millennium finite problem: {}", problem_id),
                    schema_id: SchemaId::FiniteSearch,
                    dependency_hashes: vec![],
                    unresolved_bound: None,
                }],
                None,
            ));
        }
    };

    let prog_hash = program.ser_pi_hash();

    // Construct reduction chain describing the honest mathematical reduction
    let reduction_justification = format!(
        "{} (problem_id='{}', n={}{}) reduced to VM program via {}",
        contract.description,
        problem_id,
        parameter_n,
        parameter_aux.map(|a| format!(", aux={}", a)).unwrap_or_default(),
        description,
    );

    let reduction_step = ReductionStep {
        from_hash: contract.qid,
        to_hash: prog_hash,
        justification: reduction_justification,
        step_hash: hash::H(&[contract.qid.as_slice(), prog_hash.as_slice()].concat()),
    };

    let proof_eq = ProofEq {
        statement_hash: contract.qid,
        program_hash: prog_hash,
        b_star,
        reduction_chain: vec![reduction_step],
        proof_hash: ProofEq::compute_hash(&contract.qid, &prog_hash, b_star, &[]),
    };

    let halting_argument = format!(
        "Program has {} instructions. {} B*={} derived from parameter bounds and loop structure.",
        program.len(), description, b_star
    );
    let proof_total = ProofTotal {
        program_hash: prog_hash,
        b_star,
        halting_argument: halting_argument.clone(),
        proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, &halting_argument),
    };

    // Execute VM to verify
    let (outcome, _state) = Vm::run(&program, b_star);
    match &outcome {
        VmOutcome::Halted(_) => {}
        other => {
            return Err(FrontierWitness::new(
                contract.qid,
                vec![SchemaId::FiniteSearch],
                vec![Gap {
                    goal_hash: contract.qid,
                    goal_statement: format!(
                        "VM execution for '{}' did not halt: {:?} (B*={})",
                        problem_id, other, b_star
                    ),
                    schema_id: SchemaId::FiniteSearch,
                    dependency_hashes: vec![],
                    unresolved_bound: Some(format!("B* = {}", b_star)),
                }],
                None,
            ));
        }
    }

    let frc = Frc::new(program, b_star, proof_eq, proof_total, SchemaId::FiniteSearch, contract.qid);

    // Emit event
    ledger.commit(Event::new(
        EventKind::FrcComplete,
        &frc.ser_pi(),
        vec![ledger.head()],
        b_star,
        1,
    ));

    Ok(frc)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_contracts::compiler::compile_contract;

    fn test_ledger() -> Ledger {
        Ledger::new()
    }

    #[test]
    fn goldbach_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Goldbach [4, 100]",
            "problem_id": "goldbach",
            "parameter_n": 100
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn collatz_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Collatz [1, 100]",
            "problem_id": "collatz",
            "parameter_n": 100,
            "parameter_aux": 500
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn twin_primes_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Twin primes [2, 10000]",
            "problem_id": "twin_primes",
            "parameter_n": 10000
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn zfc_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "0 ≠ 1",
            "problem_id": "zfc_zero_ne_one",
            "parameter_n": 0
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn mertens_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Mertens [1, 100]",
            "problem_id": "mertens",
            "parameter_n": 100
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn legendre_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Legendre [1, 50]",
            "problem_id": "legendre",
            "parameter_n": 50
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn erdos_straus_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Erdos-Straus [2, 100]",
            "problem_id": "erdos_straus",
            "parameter_n": 100
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn bsd_ec_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "BSD EC F_97",
            "problem_id": "bsd_ec_count",
            "parameter_n": 97,
            "parameter_aux": 0
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn weak_goldbach_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Weak Goldbach [7, 101]",
            "problem_id": "weak_goldbach",
            "parameter_n": 101
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn bertrand_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Bertrand [1, 100]",
            "problem_id": "bertrand",
            "parameter_n": 100
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn lagrange_frc() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Lagrange [1, 100]",
            "problem_id": "lagrange_four_squares",
            "parameter_n": 100
        }"#).unwrap();
        let mut ledger = test_ledger();
        let frc = build_millennium_frc(&contract, &mut ledger).unwrap();
        assert!(frc.verify_internal());
        let (outcome, _) = Vm::run(&frc.program, frc.b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    #[test]
    fn unknown_problem_returns_error() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "unknown",
            "problem_id": "nonexistent",
            "parameter_n": 100
        }"#).unwrap();
        let mut ledger = test_ledger();
        let result = build_millennium_frc(&contract, &mut ledger);
        assert!(result.is_err());
    }

    #[test]
    fn frc_hash_deterministic() {
        let contract = compile_contract(r#"{
            "type": "millennium_finite",
            "description": "Goldbach determinism test",
            "problem_id": "goldbach",
            "parameter_n": 50
        }"#).unwrap();
        let mut l1 = test_ledger();
        let mut l2 = test_ledger();
        let frc1 = build_millennium_frc(&contract, &mut l1).unwrap();
        let frc2 = build_millennium_frc(&contract, &mut l2).unwrap();
        assert_eq!(frc1.frc_hash, frc2.frc_hash);
    }
}
