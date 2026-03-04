// Integration tests — end-to-end verification of the truthful FRC engine.
//
// Tests every contract type through the full pipeline:
//   Contract → SearchProblem → Program → VM → FRC → cross-verify with solver

use kernel_types::{hash, SerPi};
use kernel_ledger::Ledger;
use kernel_contracts::compiler::compile_contract;
use kernel_solver::Solver;
use kernel_frc::contract_frc::{build_contract_frc, verify_frc_against_solver};
use kernel_frc::{Vm, VmOutcome, FrcSearch, GapLedger, MotifLibrary};
use kernel_frc::class_c::CoverageReport;
use kernel_goldmaster::suite::GoldMasterSuite;

fn test_ledger() -> Ledger {
    Ledger::new()
}

// --- BoolCnf tests ---

#[test]
fn bool_cnf_sat_end_to_end() {
    // Q0-equivalent: (x1 OR x2) — SAT
    let contract = compile_contract(r#"{
        "type": "bool_cnf",
        "description": "x1 OR x2",
        "num_vars": 2,
        "clauses": [[1, 2]]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());

    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1)); // SAT

    // Cross-verify with solver
    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

#[test]
fn bool_cnf_unsat_end_to_end() {
    // Q1-equivalent: x AND NOT x — UNSAT
    let contract = compile_contract(r#"{
        "type": "bool_cnf",
        "description": "x AND NOT x",
        "num_vars": 1,
        "clauses": [[1], [-1]]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());

    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(0)); // UNSAT

    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

#[test]
fn bool_cnf_forced_end_to_end() {
    // Q2-equivalent: forced SAT (single variable, single positive clause)
    let contract = compile_contract(r#"{
        "type": "bool_cnf",
        "description": "forced x1",
        "num_vars": 1,
        "clauses": [[1]]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1)); // SAT

    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

// --- ArithFind tests ---

#[test]
fn arith_find_unique_end_to_end() {
    // Q4-equivalent: 2x+3=7 → x=2
    let contract = compile_contract(r#"{
        "type": "arith_find",
        "description": "2x+3=7",
        "coefficients": [3, 2],
        "target": 7,
        "answer_range": [0, 100]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());

    let (outcome, state) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1)); // found
    assert_eq!(*state.memory.get(&0).unwrap(), 2); // x = 2

    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

#[test]
fn arith_find_unsat_end_to_end() {
    // Q5-equivalent: x²=-1 — no integer solution
    let contract = compile_contract(r#"{
        "type": "arith_find",
        "description": "x^2=-1",
        "coefficients": [0, 0, 1],
        "target": -1,
        "answer_range": [-100, 100]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(0)); // UNSAT

    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

// --- Table tests ---

#[test]
fn table_sat_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "table",
        "description": "table with SAT entry",
        "entries": [
            {"key": "alpha", "value": "UNSAT"},
            {"key": "beta", "value": "SAT"}
        ]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1)); // found SAT entry

    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

#[test]
fn table_unsat_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "table",
        "description": "all UNSAT table",
        "entries": [
            {"key": "alpha", "value": "UNSAT"},
            {"key": "beta", "value": "UNSAT"}
        ]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(0)); // no SAT entry

    let mut solver = Solver::new();
    let output = solver.solve(&contract);
    assert!(verify_frc_against_solver(&contract, &frc, &output.status));
}

// --- FormalProof tests ---

#[test]
fn formal_proof_invalid_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "formal_proof",
        "description": "P vs NP (inadmissible)",
        "statement": "P = NP",
        "formal_system": "ZFC",
        "required_separator": "Prove or disprove P = NP"
    }"#).unwrap();
    let mut ledger = test_ledger();

    let result = build_contract_frc(&contract, &mut ledger);
    assert!(result.is_err());

    let frontier = result.unwrap_err();
    assert_eq!(frontier.statement_hash, contract.qid);
    assert!(!frontier.gaps.is_empty());
    assert!(frontier.gaps[0].goal_statement.contains("FormalProof"));
}

// --- Cross-verification across all GoldMaster ---

#[test]
fn cross_verify_all_goldmaster_contracts() {
    let gm = GoldMasterSuite::v1();
    let mut ledger = test_ledger();

    for (i, contract) in gm.contracts.iter().enumerate() {
        let mut solver = Solver::new();
        let output = solver.solve(contract);

        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                assert!(frc.verify_internal(),
                    "Q{}: FRC internal verify failed", i);
                assert!(verify_frc_against_solver(contract, &frc, &output.status),
                    "Q{}: FRC-solver cross-verify failed (FRC vs {})", i, output.status);
            }
            Err(_frontier) => {
                // FormalProof contracts are expected to be INVALID
                // Solver should say UNSAT for these
                assert_eq!(output.status, kernel_types::Status::Unsat,
                    "Q{}: INVALID contract but solver didn't say UNSAT", i);
            }
        }
    }
}

// --- Determinism ---

#[test]
fn frc_deterministic() {
    let contract = compile_contract(r#"{
        "type": "arith_find",
        "description": "3x+1=10",
        "coefficients": [1, 3],
        "target": 10,
        "answer_range": [0, 50]
    }"#).unwrap();

    let mut l1 = test_ledger();
    let mut l2 = test_ledger();

    let frc1 = build_contract_frc(&contract, &mut l1).unwrap();
    let frc2 = build_contract_frc(&contract, &mut l2).unwrap();

    assert_eq!(frc1.frc_hash, frc2.frc_hash);
    assert_eq!(frc1.program.ser_pi_hash(), frc2.program.ser_pi_hash());
    assert_eq!(frc1.b_star, frc2.b_star);
}

// --- Coverage metrics ---

#[test]
fn coverage_rate_100_for_solvable() {
    let gap_ledger = GapLedger::new();
    let mut motif_lib = MotifLibrary::new();

    // Add 5 fake motifs
    for i in 0..5 {
        let h = hash::H(format!("m{}", i).as_bytes());
        let contract = compile_contract(&format!(r#"{{
            "type": "arith_find",
            "description": "test{}",
            "coefficients": [0, 1],
            "target": {},
            "answer_range": [0, 100]
        }}"#, i, i)).unwrap();
        let mut ledger = test_ledger();
        let frc = build_contract_frc(&contract, &mut ledger).unwrap();
        motif_lib.add_motif(h, format!("m{}", i), frc);
    }

    let report = CoverageReport::compute(5, 0, &gap_ledger, &motif_lib, 6, None);
    assert_eq!(report.coverage_rate_milli, 1000); // 100%
    assert_eq!(report.frc_found, 5);
}

// --- Gap closure produces motifs ---

#[test]
fn gap_closure_produces_motif() {
    let mut engine = FrcSearch::new();
    let mut ledger = test_ledger();

    // Search for something that produces a gap
    let stmt_hash = hash::H(b"forall x: P(x)");
    let stmt = kernel_frc::schema::StatementDesc {
        kind: kernel_frc::schema::StatementKind::UniversalInfinite,
        text: "forall x: P(x)".to_string(),
        variables: vec![],
        predicate: "P(x)".to_string(),
        params: vec![],
    };
    let ctx = kernel_frc::schema::ReductionContext::default_context();

    let result = engine.search(stmt_hash, &stmt, &ctx, &mut ledger);
    assert!(matches!(result, kernel_frc::FrcResult::Invalid(_)));
    assert!(engine.gap_ledger.active_count() > 0);

    // Try to close with program builder
    let resolved = engine.try_close_gaps_with_programs(&mut ledger);
    // Some gaps may be closable, some may not — just verify it doesn't panic
    // and motif library grows if gaps are resolved
    if resolved > 0 {
        assert!(engine.motif_library.len() > 0);
    }
}

// --- Contract QID appears in FRC ---

#[test]
fn contract_qid_in_frc() {
    let contract = compile_contract(r#"{
        "type": "bool_cnf",
        "description": "qid test",
        "num_vars": 2,
        "clauses": [[1, 2]]
    }"#).unwrap();
    let mut ledger = test_ledger();

    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert_eq!(frc.statement_hash, contract.qid);
}

// --- Open problem finite fragments ---

#[test]
fn goldbach_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Goldbach [4, 1000]",
        "problem_id": "goldbach",
        "parameter_n": 1000
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn collatz_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Collatz [1, 5000]",
        "problem_id": "collatz",
        "parameter_n": 5000,
        "parameter_aux": 1000
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn twin_primes_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Twin primes [2, 10000]",
        "problem_id": "twin_primes",
        "parameter_n": 10000
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, state) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
    // Should find quickly (3, 5 are twin primes)
    assert!(state.steps_taken < 200);
}

#[test]
fn flt_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "FLT [3,7] [1,40]",
        "problem_id": "flt",
        "parameter_n": 7,
        "parameter_aux": 40
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn odd_perfect_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Odd perfect [1, 5000]",
        "problem_id": "odd_perfect",
        "parameter_n": 5000
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn mersenne_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Mersenne [2, 31]",
        "problem_id": "mersenne",
        "parameter_n": 31
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, state) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
    assert!(state.steps_taken < 200);
}

#[test]
fn zfc_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "0 != 1",
        "problem_id": "zfc_zero_ne_one",
        "parameter_n": 0
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, state) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
    assert!(state.steps_taken <= 6);
}

// --- New open problem finite fragments (Mertens, Legendre, Erdős-Straus, BSD, Weak Goldbach, Bertrand, Lagrange) ---

#[test]
fn mertens_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Mertens |M(n)| <= sqrt(n) for n in [1, 100]",
        "problem_id": "mertens",
        "parameter_n": 100
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn legendre_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Legendre: prime in (n^2, (n+1)^2) for n in [1, 50]",
        "problem_id": "legendre",
        "parameter_n": 50
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn erdos_straus_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Erdos-Straus 4/n=1/x+1/y+1/z for n in [2, 100]",
        "problem_id": "erdos_straus",
        "parameter_n": 100
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn bsd_ec_count_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "BSD EC y^2=x^3-x over F_97, Hasse bound",
        "problem_id": "bsd_ec_count",
        "parameter_n": 97,
        "parameter_aux": 0
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn weak_goldbach_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Weak Goldbach: odd n in [7, 101] is sum of 3 primes",
        "problem_id": "weak_goldbach",
        "parameter_n": 101
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn bertrand_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Bertrand: prime in (n, 2n) for n in [1, 100]",
        "problem_id": "bertrand",
        "parameter_n": 100
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn lagrange_four_squares_frc_end_to_end() {
    let contract = compile_contract(r#"{
        "type": "millennium_finite",
        "description": "Lagrange: n=a^2+b^2+c^2+d^2 for n in [1, 100]",
        "problem_id": "lagrange_four_squares",
        "parameter_n": 100
    }"#).unwrap();
    let mut ledger = test_ledger();
    let frc = build_contract_frc(&contract, &mut ledger).unwrap();
    assert!(frc.verify_internal());
    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
    assert_eq!(outcome, VmOutcome::Halted(1));
}

#[test]
fn all_finite_fragments_deterministic() {
    use kernel_types::SerPi;

    let specs = vec![
        r#"{"type":"millennium_finite","description":"g","problem_id":"goldbach","parameter_n":100}"#,
        r#"{"type":"millennium_finite","description":"c","problem_id":"collatz","parameter_n":100,"parameter_aux":500}"#,
        r#"{"type":"millennium_finite","description":"t","problem_id":"twin_primes","parameter_n":100}"#,
        r#"{"type":"millennium_finite","description":"z","problem_id":"zfc_zero_ne_one","parameter_n":0}"#,
        r#"{"type":"millennium_finite","description":"m","problem_id":"mertens","parameter_n":50}"#,
        r#"{"type":"millennium_finite","description":"l","problem_id":"legendre","parameter_n":20}"#,
        r#"{"type":"millennium_finite","description":"e","problem_id":"erdos_straus","parameter_n":50}"#,
        r#"{"type":"millennium_finite","description":"b","problem_id":"bsd_ec_count","parameter_n":7,"parameter_aux":0}"#,
        r#"{"type":"millennium_finite","description":"w","problem_id":"weak_goldbach","parameter_n":51}"#,
        r#"{"type":"millennium_finite","description":"br","problem_id":"bertrand","parameter_n":50}"#,
        r#"{"type":"millennium_finite","description":"lq","problem_id":"lagrange_four_squares","parameter_n":50}"#,
    ];

    for spec in specs {
        let contract = compile_contract(spec).unwrap();
        let mut l1 = test_ledger();
        let mut l2 = test_ledger();
        let frc1 = build_contract_frc(&contract, &mut l1).unwrap();
        let frc2 = build_contract_frc(&contract, &mut l2).unwrap();
        assert_eq!(frc1.frc_hash, frc2.frc_hash,
            "Non-deterministic FRC for: {}", spec);
        assert_eq!(frc1.program.ser_pi_hash(), frc2.program.ser_pi_hash());
    }
}

// --- B* is sufficient for all GoldMaster contracts ---

#[test]
fn b_star_sufficient_all_goldmaster() {
    let gm = GoldMasterSuite::v1();
    let mut ledger = test_ledger();

    for (i, contract) in gm.contracts.iter().enumerate() {
        if let Ok(frc) = build_contract_frc(contract, &mut ledger) {
            let (outcome, state) = Vm::run(&frc.program, frc.b_star);
            assert!(matches!(outcome, VmOutcome::Halted(_)),
                "Q{}: B* insufficient — {:?} after {} steps (B*={})",
                i, outcome, state.steps_taken, frc.b_star);
        }
    }
}
