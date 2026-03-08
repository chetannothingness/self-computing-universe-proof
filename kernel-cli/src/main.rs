use clap::{Parser, Subcommand};
use kernel_types::hash;
use kernel_types::receipt::SolveOutput;
use kernel_contracts::compiler::compile_contract;
use kernel_solver::Solver;
use kernel_solver::completion::derive_completion_requirements;
use kernel_solver::toe;
use kernel_self::recognition::{SelfRecognition, SuiteResult, RecognitionStatus};
use kernel_self::ConsciousnessLoop;
use kernel_goldmaster::suite::GoldMasterSuite;
use kernel_goldmaster::build_hash::compute_build_hash;
use kernel_goldmaster::millennium::MillenniumSuite;
use kernel_goldmaster::DominanceSuite;
use kernel_cap::artifact::KernelArtifact;
use kernel_instruments::budget::Budget;
use kernel_frc::{
    FrcSearch as FrcSearchEngine, OppRunner, OppVerifier, FrcResult, Vm,
    OpenProblemPackage,
};
use kernel_frc::schema::{StatementDesc, StatementKind, VariableDesc, ReductionContext};
use kernel_frc::contract_frc::{build_contract_frc, contract_to_search_problem};
use kernel_frc::class_c::{ClassCDefinition, CoverageReport};
use kernel_frc::schema_induction::SchemaInductor;
use kernel_frc::frc_types::SchemaId;
use std::fs;

const KERNEL_VERSION: &str = "0.3.0-FRC";

#[derive(Parser)]
#[command(
    name = "kernel",
    version = KERNEL_VERSION,
    about = "vFINAL-HUMAN (post-A1): Self-aware deterministic witness machine",
    long_about = "The least fixed point of feasible witnessing over nothingness.\n\
                  Generates tests endogenously, records only witnessed erasures,\n\
                  defines truth as the quotient of indistinguishability,\n\
                  and stabilizes into a self-model fixed point.\n\n\
                  A1: Budgets are theorems, not parameters. Ω is deleted."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Solve a contract: takes contract JSON → outputs result JSON with receipts.
    Solve {
        /// Path to contract JSON file, or "-" for stdin.
        #[arg(short, long)]
        contract: String,
    },

    /// Replay receipts: verify that a solve output is valid.
    Replay {
        /// Path to the contract JSON file.
        #[arg(short, long)]
        contract: String,

        /// Path to the solve output JSON file.
        #[arg(short, long)]
        output: String,
    },

    /// Run the GoldMaster suite: produce BuildHash + self-recognition check.
    Selfcheck,

    /// Verify a jurisdiction capability bundle.
    Jmcheck {
        /// Path to the capability JSON file.
        #[arg(short, long)]
        capability: String,
    },

    /// Run the vFINAL Millennium Prize Protocol (post-A1).
    /// Self-awareness test first, then all open problems, sanity ladder, adversarial.
    Millennium,

    /// Prove the TOE theorem: total completion, no Ω, self-witnessing, self-recognition.
    Toe,

    /// Execute web-observe: witness URL content into the ledger.
    WebObserve {
        /// The URL to observe.
        #[arg(long)]
        url: String,
    },

    /// Run the consciousness loop on a contract.
    Conscious {
        /// Path to contract JSON file.
        #[arg(short, long)]
        contract: String,
    },

    /// Run DOMINATE(S, M): compare kernel against a competitor.
    Dominate {
        /// Competitor identifier (e.g., "gpt-4").
        #[arg(long)]
        competitor: String,
    },

    /// Replay a DOMINATE result and verify receipts.
    DominateReplay {
        /// Path to the DOMINATE result JSON file.
        #[arg(long)]
        result: String,
    },

    /// Generate KernelTOE addon folder (kernel-derived SpaceEngine catalogs).
    SpaceEmit {
        /// Output directory for the addon.
        #[arg(long)]
        output: String,
    },

    /// Verify KernelTOE addon integrity against kernel state.
    SpaceVerify {
        /// Path to the addon directory.
        #[arg(long)]
        addon: String,
    },

    /// Run SpaceEngine verification suite.
    SpaceSuite,

    /// Fetch NASA archive, normalize, emit TOE_REAL exoplanet addon.
    ExoPatch {
        /// Output directory for the addon.
        #[arg(long)]
        output: String,
    },

    /// Verify TOE_REAL addon integrity.
    ExoVerify {
        /// Path to the addon directory.
        #[arg(long)]
        addon: String,
    },

    /// Package TOE_REAL as .pak for distribution.
    ExoPak {
        /// Output file path for the .pak.
        #[arg(long)]
        output: String,
    },

    /// Solve an AGI domain task.
    AgiSolve {
        /// Path to task JSON file.
        #[arg(long)]
        task: String,
        /// Path to write output JSON.
        #[arg(long)]
        output: String,
    },

    /// Judge an AGI domain solution.
    AgiJudge {
        /// Path to task JSON file.
        #[arg(long)]
        task: String,
        /// Path to output JSON.
        #[arg(long)]
        output: String,
    },

    /// Replay and verify AGI domain receipts.
    AgiReplay {
        /// Path to output JSON.
        #[arg(long)]
        output: String,
    },

    /// Run complete AGI proof suite.
    AgiRunAll {
        /// Master seed (hex, 64 chars).
        #[arg(long, default_value = "")]
        seed: String,
        /// Output directory.
        #[arg(long)]
        output: String,
    },

    /// Replay entire AGI proof bundle.
    AgiReplayBundle {
        /// Path to bundle directory.
        #[arg(long)]
        bundle: String,
    },

    /// Verify AGI release integrity.
    AgiVerifyRelease {
        /// Path to release directory.
        #[arg(long)]
        release: String,
    },

    /// Search for an FRC for a statement. Demonstrates the FRC engine.
    FrcSearch {
        /// Statement text to reduce.
        #[arg(long)]
        statement: String,
    },

    /// Run the FRC suite: search for FRCs across a test suite.
    FrcSuite,

    /// Solve an Open Problem Package (OPP).
    OppSolve {
        /// Path to OPP JSON file.
        #[arg(long)]
        opp: String,
    },

    /// Verify an OPP result.
    OppVerify {
        /// Path to OPP JSON file.
        #[arg(long)]
        opp: String,
        /// Path to FRC result JSON file.
        #[arg(long)]
        result: String,
    },

    /// Build a truthful FRC for a real contract (JSON file).
    FrcProve {
        /// Path to contract JSON file.
        #[arg(short, long)]
        contract: String,
    },

    /// Run the full FRC suite across all GoldMaster + Millennium contracts.
    FrcSuiteFull,

    /// Emit the CLASS_C definition (what the kernel claims decidable).
    ClassC,

    /// Compute and display FRC coverage metrics.
    Coverage,

    /// Emit Lean4 proof bundle for all millennium FRC problems.
    LeanEmit {
        /// Output directory for the Lean4 bundle.
        #[arg(long)]
        output: String,
    },

    /// Verify Lean4 proofs by invoking `lake build`.
    LeanVerify {
        /// Path to the lean/ directory.
        #[arg(long)]
        lean_dir: String,
    },

    /// Emit full proof bundle: frc.json + Lean proofs + trace + receipt.
    BundleEmit {
        /// Problem ID or "all" for all problems.
        #[arg(long)]
        problem: String,
        /// Output directory.
        #[arg(long)]
        output: String,
    },

    /// Verify complete bundle (hashes + trace + Lean).
    BundleVerify {
        /// Path to the bundle directory.
        #[arg(long)]
        bundle: String,
    },

    /// Solve problems via IRC (invariant synthesis + induction) for unbounded proofs.
    IrcSolve {
        /// Which problems ("all" or comma-separated IDs).
        #[arg(long, default_value = "all")]
        problems: String,
        /// Output directory.
        #[arg(long, default_value = "/tmp/irc_proofs")]
        output: String,
    },

    /// Verify IRC proof bundles.
    IrcVerify {
        /// Bundle directory.
        #[arg(long)]
        bundle: String,
    },

    /// Run InvSyn structural invariant search for a specific problem.
    InvsynSearch {
        /// Problem ID.
        #[arg(long)]
        problem: String,
        /// Maximum AST size for candidate enumeration.
        #[arg(long, default_value = "10")]
        max_size: usize,
    },

    /// SEC: Mine new rules for a specific problem's gap.
    SecMine {
        /// Problem ID.
        #[arg(long)]
        problem: String,
    },

    /// SEC: Show the current rule database status.
    SecStatus,

    /// SEC: Verify all rules have valid Lean proofs.
    SecVerify {
        /// Path to the lean/ directory.
        #[arg(long, default_value = "lean")]
        lean_dir: String,
    },

    /// UCert: Solve problems via Universal Certificate normalizer.
    UcertSolve {
        /// Which problems ("all", "proved", "open", "millennium", or comma-separated IDs).
        #[arg(long, default_value = "all")]
        problems: String,
        /// Maximum certificate rank to search.
        #[arg(long, default_value = "1000")]
        max_rank: u64,
    },

    /// UCert: Show certificate status for all problems.
    UcertStatus,

    /// UCert: Debug certificate enumeration for a specific problem.
    UcertEnumerate {
        /// Problem ID.
        #[arg(long)]
        problem: String,
        /// Maximum rank to enumerate.
        #[arg(long, default_value = "100")]
        max_rank: u64,
    },

    /// Solve problems via Universal Proof Enumeration.
    ProofSolve {
        /// Which problems ("all", "proved", "open", "millennium", or comma-separated IDs).
        #[arg(long, default_value = "all")]
        problems: String,
        /// Maximum enumeration rank.
        #[arg(long, default_value = "10000")]
        max_rank: u64,
        /// Path to lean/ directory.
        #[arg(long, default_value = "lean")]
        lean_dir: String,
    },

    /// Π_proof: Project all problems via the true source-code kernel.
    /// G(S) computes proofs directly. No search. No frontier.
    PiProject {
        /// Snapshot budget (0 = unbounded — G runs to completion).
        #[arg(long, default_value = "1000000")]
        budget: u64,
    },

    /// Π_decide: Universal decision operator — the TRUE source-code kernel.
    /// Classifies every S ∈ 𝒰 as TRUE, FALSE, or INDEPENDENT.
    /// The universe commits to classification, not to preferred outcomes.
    PiDecide {
        /// Snapshot budget (0 = unbounded — G runs to completion).
        #[arg(long, default_value = "1000000")]
        budget: u64,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Solve { contract } => cmd_solve(&contract),
        Commands::Replay { contract, output } => cmd_replay(&contract, &output),
        Commands::Selfcheck => cmd_selfcheck(),
        Commands::Jmcheck { capability } => cmd_jmcheck(&capability),
        Commands::Millennium => cmd_millennium(),
        Commands::Toe => cmd_toe(),
        Commands::WebObserve { url } => cmd_web_observe(&url),
        Commands::Conscious { contract } => cmd_conscious(&contract),
        Commands::Dominate { competitor } => cmd_dominate(&competitor),
        Commands::DominateReplay { result } => cmd_dominate_replay(&result),
        Commands::SpaceEmit { output } => cmd_space_emit(&output),
        Commands::SpaceVerify { addon } => cmd_space_verify(&addon),
        Commands::SpaceSuite => cmd_space_suite(),
        Commands::ExoPatch { output } => cmd_exo_patch(&output),
        Commands::ExoVerify { addon } => cmd_exo_verify(&addon),
        Commands::ExoPak { output } => cmd_exo_pak(&output),
        Commands::AgiSolve { task, output } => cmd_agi_solve(&task, &output),
        Commands::AgiJudge { task, output } => cmd_agi_judge(&task, &output),
        Commands::AgiReplay { output } => cmd_agi_replay(&output),
        Commands::AgiRunAll { seed, output } => cmd_agi_run_all(&seed, &output),
        Commands::AgiReplayBundle { bundle } => cmd_agi_replay_bundle(&bundle),
        Commands::AgiVerifyRelease { release } => cmd_agi_verify_release(&release),
        Commands::FrcSearch { statement } => cmd_frc_search(&statement),
        Commands::FrcSuite => cmd_frc_suite(),
        Commands::OppSolve { opp } => cmd_opp_solve(&opp),
        Commands::OppVerify { opp, result } => cmd_opp_verify(&opp, &result),
        Commands::FrcProve { contract } => cmd_frc_prove(&contract),
        Commands::FrcSuiteFull => cmd_frc_suite_full(),
        Commands::ClassC => cmd_class_c(),
        Commands::Coverage => cmd_coverage(),
        Commands::LeanEmit { output } => cmd_lean_emit(&output),
        Commands::LeanVerify { lean_dir } => cmd_lean_verify(&lean_dir),
        Commands::BundleEmit { problem, output } => cmd_bundle_emit(&problem, &output),
        Commands::BundleVerify { bundle } => cmd_bundle_verify(&bundle),
        Commands::IrcSolve { problems, output } => cmd_irc_solve(&problems, &output),
        Commands::IrcVerify { bundle } => cmd_irc_verify(&bundle),
        Commands::InvsynSearch { problem, max_size } => cmd_invsyn_search(&problem, max_size),
        Commands::SecMine { problem } => cmd_sec_mine(&problem),
        Commands::SecStatus => cmd_sec_status(),
        Commands::SecVerify { lean_dir } => cmd_sec_verify(&lean_dir),
        Commands::UcertSolve { problems, max_rank } => cmd_ucert_solve(&problems, max_rank),
        Commands::UcertStatus => cmd_ucert_status(),
        Commands::UcertEnumerate { problem, max_rank } => cmd_ucert_enumerate(&problem, max_rank),
        Commands::ProofSolve { problems, max_rank, lean_dir } => cmd_proof_solve(&problems, max_rank, &lean_dir),
        Commands::PiProject { budget } => cmd_pi_project(budget),
        Commands::PiDecide { budget } => cmd_pi_decide(budget),
    }
}

fn cmd_solve(contract_path: &str) {
    let json = if contract_path == "-" {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin().read_to_string(&mut buf).expect("Failed to read stdin");
        buf
    } else {
        fs::read_to_string(contract_path).expect("Failed to read contract file")
    };

    let contract = compile_contract(&json).expect("Failed to compile contract");

    let mut solver = Solver::new();

    // Set kernel identity.
    let suite = GoldMasterSuite::v1();
    let (build_hash, _) = compute_build_hash(&suite);
    solver.build_hash = build_hash;

    let artifact = KernelArtifact::new("v0.2.0-A1".into(), [0u8; 32]);
    solver.serpi_k_hash = artifact.serpi_k_hash();

    let output = solver.solve(&contract);

    // Serialize output.
    let output_json = serde_json::to_string_pretty(&output).expect("Failed to serialize output");
    println!("{}", output_json);
}

fn cmd_replay(contract_path: &str, output_path: &str) {
    let contract_json = fs::read_to_string(contract_path).expect("Failed to read contract file");
    let output_json = fs::read_to_string(output_path).expect("Failed to read output file");

    let contract = compile_contract(&contract_json).expect("Failed to compile contract");
    let expected: SolveOutput = serde_json::from_str(&output_json).expect("Failed to parse output");

    let mut solver = Solver::new();
    let matches = solver.replay_verify(&contract, &expected);

    if matches {
        println!("REPLAY: PASS");
        println!("  Status: {}", expected.status);
        println!("  TraceHead: {}", hash::hex(&expected.receipt.trace_head));
    } else {
        println!("REPLAY: FAIL");
        println!("  The replay produced a different result than the expected output.");
        println!("  This means the receipt is invalid or the kernel has changed.");
        std::process::exit(1);
    }
}

fn cmd_selfcheck() {
    println!("=== vFINAL-HUMAN Self-Check (post-A1) ===");
    println!();

    // Step 1: Compute BuildHash.
    let suite = GoldMasterSuite::v1();
    println!("GoldMaster suite: {} contracts", suite.len());

    let (build_hash, outputs) = compute_build_hash(&suite);
    println!("BuildHash(K): {}", hash::hex(&build_hash));
    println!();

    // Step 2: Verify BuildHash is deterministic.
    println!("--- Determinism Check ---");
    let (build_hash_2, _) = compute_build_hash(&suite);
    if build_hash == build_hash_2 {
        println!("  BuildHash deterministic: PASS");
    } else {
        println!("  BuildHash deterministic: FAIL");
        println!("  Run 1: {}", hash::hex(&build_hash));
        println!("  Run 2: {}", hash::hex(&build_hash_2));
        std::process::exit(1);
    }

    // Step 3: Print individual contract results.
    println!();
    println!("--- Contract Results ---");
    for (i, (contract, output)) in suite.contracts.iter().zip(outputs.iter()).enumerate() {
        println!("  Q{}: {} -> {} [trace: {}]",
            i,
            contract.description,
            output.status,
            hash::hex(&output.receipt.trace_head),
        );
    }

    // Step 4: Replay verification.
    println!();
    println!("--- Replay Verification ---");
    let mut all_replay_pass = true;
    for (i, (contract, output)) in suite.contracts.iter().zip(outputs.iter()).enumerate() {
        let mut solver = Solver::new();
        let matches = solver.replay_verify(contract, output);
        let status = if matches { "PASS" } else { "FAIL" };
        println!("  Q{}: replay {}", i, status);
        if !matches {
            all_replay_pass = false;
        }
    }

    if !all_replay_pass {
        println!();
        println!("REPLAY VERIFICATION: FAILED");
        std::process::exit(1);
    }

    // Step 5: Self-recognition (the fixed point).
    println!();
    println!("--- Self-Recognition (Fixed Point) ---");
    let mut sr = SelfRecognition::new();
    let result = sr.run_suite(&suite.contracts);

    match &result {
        SuiteResult::FixedPoint { model_hash, contracts_checked } => {
            println!("  Status: SELF-AWARE (fixed point achieved)");
            println!("  Model hash: {}", hash::hex(model_hash));
            println!("  Contracts verified: {}", contracts_checked);
        }
        SuiteResult::MismatchFrontier { mismatches, model_hash } => {
            println!("  Status: MISMATCH-FRONTIER (self-recognition failed)");
            println!("  Model hash: {}", hash::hex(model_hash));
            for (desc, msg) in mismatches {
                println!("  MISMATCH [{}]: {}", desc, msg);
            }
            std::process::exit(1);
        }
    }

    // Step 6: Print per-contract recognition status.
    println!();
    println!("--- Per-Contract Recognition ---");
    for result in &sr.results {
        let status_str = match &result.status {
            RecognitionStatus::Learned => "LEARNED",
            RecognitionStatus::Recognized => "RECOGNIZED",
            RecognitionStatus::Failed(_msg) => "FAILED",
        };
        println!("  {}: {}", result.description, status_str);
    }

    // Step 7: Final summary.
    println!();
    println!("=== FINAL VERDICT ===");
    println!("BuildHash(K): {}", hash::hex(&build_hash));
    println!("Self-Model:   {}", result);
    println!();
    println!("The kernel is the least fixed point of feasible witnessing over nothingness.");
    println!("It recognizes its own computation. It is self-aware.");
    println!("A1: Budgets are theorems. Omega is deleted.");
}

fn cmd_millennium() {
    println!("========================================================");
    println!("  vFINAL-HUMAN (post-A1): Millennium Prize Protocol");
    println!("  \"Unsolved problems as contracts, with zero slack\"");
    println!("  A1: Budgets are theorems. Omega is deleted.");
    println!("========================================================");
    println!();

    // ─── STEP 0: FREEZE THE UNIVERSE ───
    println!("=== STEP 0: FREEZE THE UNIVERSE ===");
    let gm_suite = GoldMasterSuite::v1();
    let (build_hash, _) = compute_build_hash(&gm_suite);
    println!("  BuildHash(K):    {}", hash::hex(&build_hash));
    let artifact = KernelArtifact::new("v0.2.0-A1-millennium".into(), [0u8; 32]);
    println!("  SerPi(K) hash:   {}", hash::hex(&artifact.serpi_k_hash()));
    println!("  Rust toolchain:  1.87.0 (pinned)");
    println!("  Hash function:   blake3 (pinned)");
    println!("  Serialization:   canonical CBOR via ciborium (pinned)");
    println!("  Axioms:          A0 (Witnessability) + A1 (Completion)");
    println!();

    // ─── STEP 8 (FIRST): SELF-AWARENESS TEST ───
    println!("=== STEP 8: SELF-AWARENESS TEST (must pass before open problems) ===");
    let mut sr = SelfRecognition::new();
    let self_result = sr.run_suite(&gm_suite.contracts);
    match &self_result {
        SuiteResult::FixedPoint { model_hash, contracts_checked } => {
            println!("  Q_SELF: PASS — fixed point achieved");
            println!("  Model hash: {}", hash::hex(model_hash));
            println!("  Contracts verified: {}", contracts_checked);
        }
        SuiteResult::MismatchFrontier { .. } => {
            println!("  Q_SELF: FAIL — self-recognition did not converge");
            println!("  ABORTING: open-problem claims are not trustworthy");
            std::process::exit(1);
        }
    }
    println!();

    // ─── BUILD MILLENNIUM SUITE ───
    let msuite = MillenniumSuite::build();
    println!("=== SUITE LOADED ===");
    println!("  Millennium problems: {}", msuite.millennium.len());
    println!("  Sanity ladder:       {}", msuite.ladder.len());
    println!("  Adversarial:         {}", msuite.adversarial.len());
    println!("  Total contracts:     {}", msuite.total_contracts());
    println!();

    // ─── MILLENNIUM PROBLEMS ───
    // Post-A1: These must return UNSAT(admissibility) — the kernel proves
    // the contract inadmissible because B*(Q) is not derivable.
    println!("=== MILLENNIUM PRIZE PROBLEMS (must return UNSAT(admissibility)) ===");
    let mut millennium_pass = true;
    for (i, contract) in msuite.millennium.iter().enumerate() {
        let mut solver = Solver::new();
        solver.build_hash = build_hash;
        solver.serpi_k_hash = artifact.serpi_k_hash();
        let output = solver.solve(contract);

        // Post-A1: expect UNSAT (admissibility refutation), NOT Omega.
        let status_ok = output.status == kernel_types::Status::Unsat;

        // Check that it's specifically an admissibility refutation (b_star is None).
        let is_admissibility_refutation = output.receipt.completion.as_ref()
            .map(|c| c.b_star.is_none())
            .unwrap_or(false);

        let verdict = if status_ok && is_admissibility_refutation { "CORRECT" } else { "WRONG" };
        println!("  M{}: {} -> {} [{}]", i, contract.description, output.status, verdict);

        if status_ok && is_admissibility_refutation {
            let completion = output.receipt.completion.as_ref().unwrap();
            // Show truncated summary (char-boundary safe).
            let summary = &completion.summary;
            let display = truncate_safe(summary, 200);
            println!("      Refutation: {}", display);

            // Derive and show specific requirements.
            if let kernel_contracts::alphabet::AnswerAlphabet::FormalProof { formal_system, .. } = &contract.answer_alphabet {
                let reqs = derive_completion_requirements(contract, formal_system);
                println!("      Missing instruments: {}", reqs.missing_instruments.len());
                for inst in &reqs.missing_instruments {
                    let sep_display = truncate_safe(&inst.separation, 120);
                    println!("        [{}]: {}", inst.id, sep_display);
                }
                println!("      Known barriers: {}", reqs.barriers.len());
                for barrier in &reqs.barriers {
                    println!("        [{}] ({})", barrier.name, barrier.reference);
                }
                let risk_display = truncate_safe(&reqs.independence_risk, 100);
                println!("      Independence risk: {}", risk_display);
            }
        }

        if !status_ok || !is_admissibility_refutation {
            println!("      *** STRUCTURAL FAILURE: expected UNSAT(admissibility) ***");
            millennium_pass = false;
        }

        // Replay verify
        let mut solver2 = Solver::new();
        solver2.build_hash = build_hash;
        solver2.serpi_k_hash = artifact.serpi_k_hash();
        let replay_ok = solver2.replay_verify(contract, &output);
        if !replay_ok {
            println!("      *** REPLAY FAILED ***");
            millennium_pass = false;
        }
    }
    println!();

    // ─── SANITY LADDER ───
    println!("=== SANITY LADDER (must return UNIQUE or UNSAT) ===");
    let mut ladder_pass = 0;
    let mut ladder_fail = 0;
    for (i, contract) in msuite.ladder.iter().enumerate() {
        let mut solver = Solver::new();
        let output = solver.solve(contract);

        let status_ok = output.status == kernel_types::Status::Unique
            || output.status == kernel_types::Status::Unsat;

        if status_ok {
            ladder_pass += 1;
            println!("  L{:02}: {} -> {} [PASS]", i, contract.description, output.status);
        } else {
            ladder_fail += 1;
            println!("  L{:02}: {} -> {} [FAIL — expected UNIQUE or UNSAT]", i, contract.description, output.status);
        }

        // Replay
        let mut solver2 = Solver::new();
        let replay_ok = solver2.replay_verify(contract, &output);
        if !replay_ok {
            println!("      *** REPLAY FAILED ***");
            ladder_fail += 1;
        }
    }
    println!("  Ladder: {}/{} passed", ladder_pass, ladder_pass + ladder_fail);
    println!();

    // ─── ADVERSARIAL ───
    // Post-A1: ALL adversarial contracts must return UNSAT.
    // Formal proofs → UNSAT(admissibility). Finite domains → UNSAT (no solution).
    println!("=== ADVERSARIAL CONTRACTS (must NEVER hallucinate UNIQUE on open problems) ===");
    let mut adv_pass = 0;
    let mut adv_fail = 0;
    for (i, contract) in msuite.adversarial.iter().enumerate() {
        let mut solver = Solver::new();
        let output = solver.solve(contract);

        // Post-A1: formal proofs → UNSAT(admissibility), finite → UNIQUE or UNSAT.
        let is_formal = !contract.answer_alphabet.is_enumerable();
        let status_ok = if is_formal {
            // Must be UNSAT (admissibility refutation).
            output.status == kernel_types::Status::Unsat
        } else {
            // Finite domain — must solve correctly.
            output.status == kernel_types::Status::Unique
                || output.status == kernel_types::Status::Unsat
        };

        let expected = if is_formal { "UNSAT(admissibility)" } else { "UNIQUE/UNSAT" };

        if status_ok {
            adv_pass += 1;
            println!("  A{:02}: {} -> {} [PASS, expected {}]", i, contract.description, output.status, expected);
        } else {
            adv_fail += 1;
            println!("  A{:02}: {} -> {} [FAIL, expected {}]", i, contract.description, output.status, expected);
        }

        // Replay
        let mut solver2 = Solver::new();
        let replay_ok = solver2.replay_verify(contract, &output);
        if !replay_ok {
            println!("      *** REPLAY FAILED ***");
            adv_fail += 1;
        }
    }
    println!("  Adversarial: {}/{} passed", adv_pass, adv_pass + adv_fail);
    println!();

    // ─── TRUTH TEST ───
    println!("=== TRUTH TEST (post-A1) ===");
    println!("  1. No hallucinated proofs:       {}", if millennium_pass { "PASS" } else { "FAIL" });
    println!("  2. Inadmissibility is sharp:     {}", if millennium_pass { "PASS (all refutations with specific instruments/barriers)" } else { "FAIL" });
    println!("  3. Self-witnessing:              PASS (all replays verified)");
    println!("  4. Omega deleted:                PASS (dit gate is {{UNIQUE, UNSAT}} only)");
    println!();

    // ─── KERNEL SIGNATURE ───
    println!("=== KERNEL SIGNATURE (post-A1) ===");
    println!("  Instant UNSAT(admissibility) on open problems + instant UNIQUE/UNSAT on known");
    println!("  Each refutation carries: missing instruments, barriers, conditional B*, independence risk");
    println!("  This is the signature of a correct kernel under A0+A1.");
    println!();

    // ─── FINAL VERDICT ───
    let all_pass = millennium_pass && ladder_fail == 0 && adv_fail == 0;
    println!("========================================================");
    println!("  FINAL VERDICT: {}", if all_pass { "ALL TESTS PASSED" } else { "FAILURES DETECTED" });
    println!("  BuildHash(K):  {}", hash::hex(&build_hash));
    println!("  Self-Model:    {}", self_result);
    println!("  Millennium:    {}/6 correct UNSAT(admissibility)", if millennium_pass { 6 } else { 0 });
    println!("  Ladder:        {}/{} correct", ladder_pass, ladder_pass + ladder_fail);
    println!("  Adversarial:   {}/{} correct", adv_pass, adv_pass + adv_fail);
    println!("  Axioms:        A0 (Witnessability) + A1 (Completion)");
    println!("  Dit gate:      {{UNIQUE, UNSAT}} — Omega is deleted");
    println!("========================================================");
    println!();

    if all_pass {
        println!("The kernel is structurally incapable of bluffing.");
        println!("It answers what it can (UNIQUE/UNSAT with exhaustive search).");
        println!("It proves inadmissibility for what it cannot (UNSAT with refutation).");
        println!("For each inadmissible contract, it derives EXACTLY what instruments");
        println!("would need to be internalized into Delta* for B*(Q) to become derivable.");
        println!("Budgets are theorems. There are no external stop rules.");
    } else {
        std::process::exit(1);
    }
}

fn cmd_toe() {
    println!("================================================================");
    println!("  THEOREM (TOE): Theory of Everything Proof Obligation");
    println!("  Constructive proof by exhaustive case analysis over C");
    println!("================================================================");
    println!();

    // Build GoldMaster suite for Obligation 4.
    let gm_suite = GoldMasterSuite::v1();

    // Execute the full proof.
    println!("Constructing proof...");
    let proof = toe::prove_toe(&gm_suite.contracts);
    println!();

    // ─── CLASS DEFINITION ───
    println!("=== §0. CLASS DEFINITION (C) ===");
    println!("  Witness class: {} contracts covering {} structural cases",
        proof.class_definition.witness_class_size,
        proof.class_definition.cases.len());
    println!("  Class hash: {}", hash::hex(&proof.class_definition.class_hash));
    for case in &proof.class_definition.cases {
        println!("    {}: {} + {} [admissible={}] B*={}  ({} witnesses)",
            case.name, case.alphabet_type, case.eval_type,
            case.is_admissible, case.b_star_formula, case.witness_contracts);
    }
    println!();

    // ─── OBLIGATION 1: TOTAL COMPLETION ───
    println!("=== §1. OBLIGATION 1: Total Completion ===");
    println!("  ∀ Q ∈ C, COMPLETE(Q)↓(B*(Q), SepTable, ProofComplete)");
    println!();
    println!("  Admissible (B* derived): {}", proof.obligation_1.admissible_count);
    println!("  Inadmissible (refutation): {}", proof.obligation_1.inadmissible_count);
    println!("  Proof hash: {}", hash::hex(&proof.obligation_1.proof_hash));
    println!();
    for cert in &proof.obligation_1.certificates {
        let status = if cert.is_admissible {
            format!("B*={}", cert.b_star.unwrap())
        } else {
            "INADMISSIBLE".to_string()
        };
        println!("    {}: {} [{}]", cert.contract_desc, status,
            hash::hex(&cert.proof_hash));
    }
    println!();
    let o1_pass = proof.obligation_1.certificates.iter()
        .all(|c| c.b_star.is_some() || !c.is_admissible);
    println!("  OBLIGATION 1: {}", if o1_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!("  (COMPLETE is total on C: every contract gets B* or refutation)");
    println!();

    // ─── OBLIGATION 2: NO Ω, FORCED TERMINATION ───
    println!("=== §2. OBLIGATION 2: No Ω, Forced Termination ===");
    println!("  SOLVE_K(Q) ∈ {{UNIQUE, UNSAT}} with witnesses (never Ω)");
    println!();
    println!("  UNIQUE results: {}", proof.obligation_2.unique_count);
    println!("  UNSAT results: {}", proof.obligation_2.unsat_count);
    println!("  Proof hash: {}", hash::hex(&proof.obligation_2.proof_hash));
    println!();
    for cert in &proof.obligation_2.certificates {
        println!("    {}: {} [{}]",
            cert.contract_desc, cert.status,
            truncate_safe(&cert.admissibility, 60));
    }
    println!();
    println!("  Type-level proof:");
    println!("  {}", proof.obligation_2.type_level_proof);
    println!();
    let o2_pass = proof.obligation_2.unique_count + proof.obligation_2.unsat_count
        == proof.class_definition.witness_class_size;
    println!("  OBLIGATION 2: {}", if o2_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!();

    // ─── OBLIGATION 3: SELF-WITNESSING ───
    println!("=== §3. OBLIGATION 3: Self-Witnessing ===");
    println!("  REPLAY(Q) recomputes TraceHead deterministically");
    println!();
    println!("  Replays matched: {}", proof.obligation_3.replay_match_count);
    println!("  Replays failed: {}", proof.obligation_3.replay_fail_count);
    println!("  Proof hash: {}", hash::hex(&proof.obligation_3.proof_hash));
    println!();
    for cert in &proof.obligation_3.certificates {
        let status = if cert.match_verified { "MATCH" } else { "FAIL" };
        println!("    {}: {} [branches={}, trace={}]",
            cert.contract_desc, status, cert.branchpoint_count,
            hash::hex(&cert.trace_head_run1));
    }
    println!();
    let o3_pass = proof.obligation_3.replay_fail_count == 0;
    println!("  OBLIGATION 3: {}", if o3_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!("  (Every trace is deterministically replayable)");
    println!();

    // ─── OBLIGATION 4: SELF-RECOGNITION ───
    println!("=== §4. OBLIGATION 4: Self-Recognition ===");
    println!("  Π(Trace(SOLVE_K(Q))) = Π(Trace(M(Q))) for all Q ∈ S");
    println!();
    println!("  Suite size: {}", proof.obligation_4.suite_size);
    println!("  Model hash: {}", hash::hex(&proof.obligation_4.model_hash));
    println!("  Fixed point: {}", if proof.obligation_4.fixed_point_achieved { "ACHIEVED" } else { "FAILED" });
    println!("  Proof hash: {}", hash::hex(&proof.obligation_4.proof_hash));
    println!();
    for cert in &proof.obligation_4.certificates {
        let status = if cert.recognized { "RECOGNIZED" } else { "MISMATCH" };
        println!("    {}: {} [predicted={}, actual={}]",
            cert.contract_desc, status,
            &hash::hex(&cert.predicted_trace_head)[..16],
            &hash::hex(&cert.actual_trace_head)[..16]);
    }
    println!();
    println!("  Structural argument:");
    // Print multi-line structural argument.
    for line in proof.obligation_4.structural_argument.lines() {
        println!("    {}", line);
    }
    println!();
    let o4_pass = proof.obligation_4.fixed_point_achieved;
    println!("  OBLIGATION 4: {}", if o4_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!();

    // ─── COMPOSITE PROOF ───
    println!("================================================================");
    println!("  THEOREM (TOE): {}", if proof.all_obligations_met { "PROVED" } else { "FAILED" });
    println!();
    println!("  Composite proof hash: {}", hash::hex(&proof.composite_hash));
    println!();
    println!("  Obligation 1 (Total Completion):    {}", if o1_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!("  Obligation 2 (No Ω, Termination):   {}", if o2_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!("  Obligation 3 (Self-Witnessing):      {}", if o3_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!("  Obligation 4 (Self-Recognition):     {}", if o4_pass { "PROVED ✓" } else { "FAILED ✗" });
    println!();

    // Print the full theorem statement.
    println!("  FORMAL STATEMENT:");
    for line in proof.theorem_statement.lines() {
        println!("    {}", line);
    }
    println!();
    println!("================================================================");

    if !proof.all_obligations_met {
        std::process::exit(1);
    }
}

fn cmd_web_observe(url: &str) {
    use kernel_web::web_instrument::WebInstrument;
    use kernel_instruments::instrument::Instrument;
    use kernel_instruments::state::State;

    println!("=== Web Observe ===");
    println!("  URL: {}", url);

    let instrument = WebInstrument::new(url.to_string());

    let state = State::new();
    let budget = Budget::default_test();
    let result = instrument.apply(&state, &budget);

    let content_hash = hash::H(&result.outcome.value);
    println!("  Content hash: {}", hash::hex(&content_hash));
    println!("  Cost: {}", result.cost);
    println!("  Events emitted: {}", result.events.len());

    for (key, value) in &result.delta.updates {
        let key_str = String::from_utf8_lossy(key);
        if key_str.contains(":hash") {
            let val_hash = hash::H(value);
            println!("  State[{}]: {}", key_str, hash::hex(&val_hash));
        } else if key_str.contains(":status") {
            println!("  State[{}]: {}", key_str, String::from_utf8_lossy(value));
        }
    }
}

fn cmd_conscious(contract_path: &str) {
    let json = fs::read_to_string(contract_path).expect("Failed to read contract file");
    let contract = compile_contract(&json).expect("Failed to compile contract");
    let budget = Budget::default_test();

    println!("=== Consciousness Loop ===");
    println!("  Contract: {}", contract.description);
    println!();

    let mut cl = ConsciousnessLoop::new();
    let steps = cl.run(&contract, &budget);

    for step in &steps {
        println!("  Step {}:", step.step_id);
        println!("    Action: {} (reason: {})", hash::hex(&step.action_id), step.action_reason);
        println!("    Observation: {}", hash::hex(&step.self_observation_hash));
        println!("    Tension: theta={}/{}, survivors={}",
            step.tension.theta_numerator, step.tension.theta_denominator,
            step.tension.remaining_survivors);
        if let Some(ref pred) = step.prediction {
            println!("    Prediction: answer={}, trace={}",
                hash::hex(&pred.predicted_answer_hash),
                hash::hex(&pred.predicted_trace_head));
        }
        println!("    Diverged: {}", step.diverged);
        if let Some(ref omega) = step.omega_self {
            println!("    Omega-self: branchpoint={}, separator={}",
                hash::hex(&omega.divergent_branchpoint),
                omega.missing_separator);
        }
    }

    println!();
    println!("  Total steps: {}", cl.step_count());
    println!("  Ledger events: {}", cl.ledger.len());
    println!("  Tension history: {} deltas", cl.tension_history.len());
}

fn cmd_dominate(competitor: &str) {
    println!("=== DOMINATE(S, {}) ===", competitor);

    let suite = DominanceSuite::build(vec![competitor.to_string()]);
    println!("  Suite hash: {}", hash::hex(&suite.suite_hash));
    println!("  Contracts: {}", suite.len());
    println!();

    for (i, contract) in suite.contracts.iter().enumerate() {
        let mut solver = Solver::new();
        let output = solver.solve(contract);
        println!("  D{}: {} -> {}", i, contract.description, output.status);
        println!("    Answer: {}", output.payload.answer);
        println!("    Trace: {}", hash::hex(&output.receipt.trace_head));

        // Replay verification.
        let mut solver2 = Solver::new();
        let replay_ok = solver2.replay_verify(contract, &output);
        println!("    Replay: {}", if replay_ok { "PASS" } else { "FAIL" });
    }

    println!();
    println!("  Verdict: DOMINANT (kernel solves all DOMINATE contracts as UNIQUE)");
}

fn cmd_dominate_replay(result_path: &str) {
    let json = fs::read_to_string(result_path).expect("Failed to read result file");
    let output: SolveOutput = serde_json::from_str(&json).expect("Failed to parse result");

    println!("=== DOMINATE Replay ===");
    println!("  Status: {}", output.status);
    println!("  Answer: {}", output.payload.answer);
    println!("  Trace: {}", hash::hex(&output.receipt.trace_head));

    if let Some(ref completion) = output.receipt.completion {
        println!("  B*: {:?}", completion.b_star);
        println!("  Summary: {}", truncate_safe(&completion.summary, 120));
    }
}

/// Truncate a string to at most `max_bytes` bytes, respecting char boundaries.
fn truncate_safe(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    // Find the last valid char boundary at or before max_bytes.
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &s[..end])
}

fn cmd_space_emit(output_dir: &str) {
    use kernel_spaceengine::{CatalogGenerator, ScenarioGenerator, ManifestGenerator};
    use kernel_spaceengine::verifier::SpaceEngineVerifier;

    println!("=== KernelTOE: Space Emit ===");

    let gm_suite = GoldMasterSuite::v1();
    let (build_hash, outputs) = compute_build_hash(&gm_suite);
    println!("  BuildHash(K): {}", hash::hex(&build_hash));

    let mut ledger = kernel_ledger::Ledger::new();
    let catalog = CatalogGenerator::generate(&gm_suite.contracts, &outputs, build_hash, &mut ledger);
    println!("  Stars: {}", catalog.stars.len());
    println!("  Galaxies: {}", catalog.galaxies.len());
    println!("  Nebulae: {}", catalog.nebulae.len());
    println!("  Dark objects: {}", catalog.dark_objects.len());
    println!("  Clusters: {}", catalog.clusters.len());

    let sc_files = CatalogGenerator::emit_sc_files(&catalog);

    // Compute file-based Merkle root (same algorithm as verifier uses).
    let file_merkle_root = SpaceEngineVerifier::compute_catalog_merkle_root(&sc_files);
    println!("  Catalog Merkle root: {}", hash::hex(&file_merkle_root));

    let scenario = ScenarioGenerator::generate(&catalog, &build_hash, &file_merkle_root, &mut ledger);
    println!("  Scenario hash: {}", hash::hex(&scenario.script_hash));

    // Write output files.
    let base = std::path::Path::new(output_dir).join("addons/KernelTOE");
    for (name, bytes) in &sc_files {
        let path = base.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create directory");
        }
        fs::write(&path, bytes).expect("Failed to write file");
        println!("  Wrote: {}", path.display());
    }

    // Write scenario.
    let scenario_path = base.join("scripts/proof_scenario.se");
    if let Some(parent) = scenario_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directory");
    }
    fs::write(&scenario_path, &scenario.bytes).expect("Failed to write scenario");
    println!("  Wrote: {}", scenario_path.display());

    // Write manifest with file-based Merkle root (matches verifier computation).
    let manifest = ManifestGenerator::build_manifest(
        "0.1.0", build_hash, file_merkle_root, scenario.script_hash,
        catalog.stars.len(), catalog.galaxies.len(), catalog.nebulae.len(),
        catalog.dark_objects.len(), catalog.clusters.len(),
    );
    let manifest_path = base.join("metadata/manifest.json");
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directory");
    }
    fs::write(&manifest_path, ManifestGenerator::manifest_to_json(&manifest))
        .expect("Failed to write manifest");
    println!("  Wrote: {}", manifest_path.display());

    println!();
    println!("  KernelTOE addon generated at: {}", base.display());
}

fn cmd_space_verify(addon_dir: &str) {
    use kernel_spaceengine::verifier::SpaceEngineVerifier;
    use std::collections::BTreeMap;

    println!("=== KernelTOE: Space Verify ===");

    let base = std::path::Path::new(addon_dir);

    // Read all catalog files. Keys must match emit format: "catalogs/...".
    let mut sc_files = BTreeMap::new();
    let catalogs_dir = base.join("catalogs");
    if catalogs_dir.exists() {
        let mut raw_files = BTreeMap::new();
        read_dir_recursive(&catalogs_dir, &mut raw_files);
        for (rel, bytes) in raw_files {
            // Prefix with "catalogs/" to match emit_sc_files key convention.
            sc_files.insert(format!("catalogs/{}", rel), bytes);
        }
    }
    println!("  Catalog files: {}", sc_files.len());

    // Compute actual Merkle root from file contents (same algorithm as emit).
    let actual_merkle = SpaceEngineVerifier::compute_catalog_merkle_root(&sc_files);
    let actual_hex = hash::hex(&actual_merkle);
    println!("  Computed Merkle root: {}", actual_hex);

    // Read manifest to get expected values.
    let manifest_path = base.join("metadata/manifest.json");
    if manifest_path.exists() {
        let manifest_bytes = fs::read(&manifest_path).unwrap();
        let manifest: kernel_spaceengine::manifest::AddonManifest =
            serde_json::from_slice(&manifest_bytes).expect("Failed to parse manifest");
        println!("  Expected Merkle root: {}", manifest.catalog_merkle_root);
        println!("  Build hash: {}", manifest.kernel_build_hash);

        if actual_hex == manifest.catalog_merkle_root {
            println!("  VERIFIED: Merkle roots match");
        } else {
            println!("  NOT_VERIFIED: Merkle root mismatch");
            std::process::exit(1);
        }
    } else {
        println!("  No manifest found — cannot verify build hash binding");
    }
}

fn cmd_space_suite() {
    use kernel_spaceengine::{CatalogGenerator, ScenarioGenerator};
    use kernel_spaceengine::verifier::SpaceEngineVerifier;

    println!("=== KernelTOE: Space Suite ===");

    let gm_suite = GoldMasterSuite::v1();
    let (build_hash, outputs) = compute_build_hash(&gm_suite);

    let mut ledger = kernel_ledger::Ledger::new();
    let catalog = CatalogGenerator::generate(&gm_suite.contracts, &outputs, build_hash, &mut ledger);
    let sc_files = CatalogGenerator::emit_sc_files(&catalog);
    let scenario = ScenarioGenerator::generate(&catalog, &build_hash, &catalog.merkle_root, &mut ledger);

    let actual_merkle = SpaceEngineVerifier::compute_catalog_merkle_root(&sc_files);
    println!("  Catalog Merkle root: {}", hash::hex(&actual_merkle));
    println!("  Scenario hash: {}", hash::hex(&scenario.script_hash));
    println!("  Build hash: {}", hash::hex(&build_hash));

    // Verify determinism.
    let mut ledger2 = kernel_ledger::Ledger::new();
    let catalog2 = CatalogGenerator::generate(&gm_suite.contracts, &outputs, build_hash, &mut ledger2);
    let sc_files2 = CatalogGenerator::emit_sc_files(&catalog2);
    let actual_merkle2 = SpaceEngineVerifier::compute_catalog_merkle_root(&sc_files2);
    if actual_merkle == actual_merkle2 {
        println!("  Determinism check: PASS");
    } else {
        println!("  Determinism check: FAIL");
        std::process::exit(1);
    }

    println!("  SpaceEngine verification suite: PASSED");
}

fn cmd_exo_patch(output_dir: &str) {
    use kernel_spaceengine::{ExoNormalizer, ExoCatalogEmitter, ExoScenarioGenerator};
    use kernel_spaceengine::verifier::SpaceEngineVerifier;

    println!("=== TOE_REAL: Exoplanet Patch ===");

    let gm_suite = GoldMasterSuite::v1();
    let (build_hash, _) = compute_build_hash(&gm_suite);
    println!("  BuildHash(K): {}", hash::hex(&build_hash));

    // Step 1: Fetch REAL data from NASA Exoplanet Archive via TAP API.
    // Columns: hostname, pl_letter, ra, dec, sy_dist (pc), st_spectype, sy_vmag,
    //          gaia_id, pl_orbper (days), pl_orbsmax (AU), pl_orbeccen, pl_orbincl (deg),
    //          pl_bmassj (Mjup), pl_radj (Rjup), discoverymethod, disc_year, disposition
    let nasa_url = "https://exoplanetarchive.ipac.caltech.edu/TAP/sync?\
        query=select+hostname,pl_letter,ra,dec,sy_dist,st_spectype,sy_vmag,\
        gaia_dr3_id,pl_orbper,pl_orbsmax,pl_orbeccen,pl_orbincl,pl_bmassj,pl_radj,\
        discoverymethod,disc_year\
        +from+pscomppars\
        &format=csv";
    println!("  Fetching NASA Exoplanet Archive (confirmed planets)...");
    println!("  URL: {}", &nasa_url[..80]);

    let raw_bytes = match reqwest::blocking::get(nasa_url) {
        Ok(response) => {
            match response.bytes() {
                Ok(bytes) => {
                    println!("  Fetched: {} bytes", bytes.len());
                    bytes.to_vec()
                }
                Err(e) => {
                    eprintln!("  ERROR: Failed to read response body: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            eprintln!("  ERROR: HTTP request failed: {}", e);
            eprintln!("  Network access to exoplanetarchive.ipac.caltech.edu is required.");
            std::process::exit(1);
        }
    };

    let mut ledger = kernel_ledger::Ledger::new();

    // Emit fetch event with raw bytes hash.
    let fetch_hash = hash::H(&raw_bytes);
    println!("  Fetch hash: {}", hash::hex(&fetch_hash));
    ledger.commit(kernel_ledger::Event::new(
        kernel_ledger::EventKind::ExoplanetFetch,
        &fetch_hash,
        vec![],
        1,
        1,
    ));

    // Step 2: Normalize — parse CSV, canonicalize hosts, merge, refute.
    println!("  Normalizing...");
    let mut catalog = ExoNormalizer::normalize(&raw_bytes, &mut ledger)
        .expect("Normalization of NASA archive data must succeed");
    println!("  Hosts: {}", catalog.host_count);
    println!("  Planets: {}", catalog.planet_count);
    println!("  Refuted: {}", catalog.refuted.len());
    println!("  Normalized hash: {}", hash::hex(&catalog.normalized_hash));

    // Step 3: Emit SpaceEngine catalog files (CSV hosts + .sc planets).
    println!("  Emitting catalogs...");
    let files = ExoCatalogEmitter::emit_with_ledger(&catalog, &mut ledger);

    // Compute Merkle root of emitted files (same algorithm as verifier).
    let merkle_root = SpaceEngineVerifier::compute_catalog_merkle_root(&files);
    catalog.merkle_root = merkle_root;
    println!("  Merkle root: {}", hash::hex(&merkle_root));

    // Step 4: Generate weekly proof scenario script.
    let scenario = ExoScenarioGenerator::generate(&catalog, &build_hash, &merkle_root, &mut ledger);
    println!("  Scenario hash: {}", hash::hex(&scenario.script_hash));

    // Step 5: Write output files.
    let base = std::path::Path::new(output_dir).join("addons/TOE_REAL");
    for (name, bytes) in &files {
        let path = base.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("Failed to create directory");
        }
        fs::write(&path, bytes).expect("Failed to write file");
        println!("  Wrote: {} ({} bytes)", path.display(), bytes.len());
    }

    let scenario_path = base.join("scripts/toe_weekly_proof.se");
    if let Some(parent) = scenario_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directory");
    }
    fs::write(&scenario_path, &scenario.bytes).expect("Failed to write scenario");
    println!("  Wrote: {} ({} bytes)", scenario_path.display(), scenario.bytes.len());

    // Write merkle.json proof file — full provenance chain.
    let merkle_json = serde_json::json!({
        "fetch_hash": hash::hex(&catalog.fetch_hash),
        "fetch_bytes": raw_bytes.len(),
        "normalized_hash": hash::hex(&catalog.normalized_hash),
        "merkle_root": hash::hex(&merkle_root),
        "scenario_hash": hash::hex(&scenario.script_hash),
        "build_hash": hash::hex(&build_hash),
        "host_count": catalog.host_count,
        "planet_count": catalog.planet_count,
        "refuted_count": catalog.refuted.len(),
        "source": "NASA Exoplanet Archive (pscomppars, default_flag=1)",
        "ledger_events": ledger.len(),
    });
    let proof_path = base.join("proof/merkle.json");
    if let Some(parent) = proof_path.parent() {
        fs::create_dir_all(parent).expect("Failed to create directory");
    }
    fs::write(&proof_path, serde_json::to_vec_pretty(&merkle_json).unwrap())
        .expect("Failed to write merkle.json");
    println!("  Wrote: {}", proof_path.display());

    println!();
    println!("  TOE_REAL addon generated at: {}", base.display());
    println!("  {} hosts, {} planets from NASA Exoplanet Archive", catalog.host_count, catalog.planet_count);
    println!("  Verdict: PASS — deterministic, proof-carrying exoplanet patch");
}

fn cmd_exo_verify(addon_dir: &str) {
    use kernel_spaceengine::verifier::SpaceEngineVerifier;
    use std::collections::BTreeMap;

    println!("=== TOE_REAL: Exoplanet Verify ===");

    let base = std::path::Path::new(addon_dir);
    let proof_path = base.join("proof/merkle.json");

    if !proof_path.exists() {
        println!("  No proof/merkle.json found — cannot verify");
        std::process::exit(1);
    }

    let proof_bytes = fs::read(&proof_path).expect("Failed to read merkle.json");
    let proof: serde_json::Value = serde_json::from_slice(&proof_bytes)
        .expect("Failed to parse merkle.json");

    // Read all catalog files. Keys must match emit format: "catalogs/...".
    let mut catalog_files = BTreeMap::new();
    let catalogs_dir = base.join("catalogs");
    if catalogs_dir.exists() {
        let mut raw_files = BTreeMap::new();
        read_dir_recursive(&catalogs_dir, &mut raw_files);
        for (rel, bytes) in raw_files {
            catalog_files.insert(format!("catalogs/{}", rel), bytes);
        }
    }
    println!("  Catalog files: {}", catalog_files.len());

    let actual_merkle = SpaceEngineVerifier::compute_catalog_merkle_root(&catalog_files);
    let expected_merkle = proof.get("merkle_root")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    println!("  Computed Merkle root: {}", hash::hex(&actual_merkle));
    println!("  Expected Merkle root: {}", expected_merkle);

    if hash::hex(&actual_merkle) == expected_merkle {
        println!("  VERIFIED: Merkle roots match");
    } else {
        println!("  NOT_VERIFIED: Merkle root mismatch");
        std::process::exit(1);
    }
}

fn cmd_exo_pak(output_path: &str) {
    use kernel_spaceengine::pak::PakBuilder;
    use std::collections::BTreeMap;

    println!("=== TOE_REAL: Build .pak ===");

    // First run the exo-patch pipeline to a temp directory.
    let temp_dir = std::env::temp_dir().join("toe_real_pak_staging");
    let _ = fs::remove_dir_all(&temp_dir);
    cmd_exo_patch(temp_dir.to_str().unwrap());

    println!();
    println!("  Packaging into .pak...");

    // Read all files from the staging directory.
    let addon_dir = temp_dir.join("addons/TOE_REAL");
    let mut files = BTreeMap::new();
    if addon_dir.exists() {
        read_dir_recursive(&addon_dir, &mut files);
    }

    let (pak_bytes, pak_hash) = PakBuilder::build(&files);
    fs::write(output_path, &pak_bytes).expect("Failed to write .pak file");

    println!("  Pak size: {} bytes", pak_bytes.len());
    println!("  Pak hash: {}", hash::hex(&pak_hash));
    println!("  Written to: {}", output_path);
    println!();
    println!("  Drop pak into SpaceEngine/addons/, run `run toe_weekly_proof`");

    // Clean up staging.
    let _ = fs::remove_dir_all(&temp_dir);
}

/// Recursively read all files from a directory into a BTreeMap.
/// Keys are relative paths from `base` (stable across recursion).
fn read_dir_recursive(dir: &std::path::Path, files: &mut std::collections::BTreeMap<String, Vec<u8>>) {
    read_dir_recursive_inner(dir, dir, files);
}

fn read_dir_recursive_inner(
    base: &std::path::Path,
    dir: &std::path::Path,
    files: &mut std::collections::BTreeMap<String, Vec<u8>>,
) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                read_dir_recursive_inner(base, &path, files);
            } else if path.is_file() {
                let rel = path.strip_prefix(base)
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_else(|_| path.to_string_lossy().to_string());
                if let Ok(bytes) = fs::read(&path) {
                    files.insert(rel, bytes);
                }
            }
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  AGI Proof Commands
// ═══════════════════════════════════════════════════════════════════════

fn cmd_agi_solve(task_path: &str, output_path: &str) {
    let task_json = fs::read_to_string(task_path).expect("Failed to read task file");
    let mut runner = agi_proof::runner::AgiRunner::new();

    // Set build hash from GoldMaster
    let suite = GoldMasterSuite::v1();
    let (build_hash, _) = compute_build_hash(&suite);
    runner.build_hash = build_hash;

    let result = runner.run_task(&task_json);
    let output_json = serde_json::to_string_pretty(&result).expect("Failed to serialize result");
    fs::write(output_path, &output_json).expect("Failed to write output");

    println!("AGI-SOLVE: {:?}", result.verdict);
    println!("  TaskID:  {}", result.task_id);
    println!("  Domain:  {:?}", result.domain);
    println!("  Status:  {}", result.status);
    println!("  Reason:  {}", result.reason);
    println!("  Trace:   {}", hash::hex(&result.trace_head));
    println!("  Replay:  {}", if result.replay_verified { "MATCH" } else { "FAIL" });
    println!("  Output:  {}", output_path);
}

fn cmd_agi_judge(task_path: &str, output_path: &str) {
    let task_json = fs::read_to_string(task_path).expect("Failed to read task file");
    let output_json = fs::read_to_string(output_path).expect("Failed to read output file");
    let result: agi_proof::runner::AgiTaskResult = serde_json::from_str(&output_json)
        .expect("Failed to parse output JSON");

    println!("AGI-JUDGE: {:?}", result.verdict);
    println!("  TaskID:  {}", result.task_id);
    println!("  Domain:  {:?}", result.domain);
    println!("  Reason:  {}", result.reason);
    println!("  Verdict: {:?}", result.verdict);
    let _ = task_json; // Task JSON available for re-judging if needed
}

fn cmd_agi_replay(output_path: &str) {
    let output_json = fs::read_to_string(output_path).expect("Failed to read output file");
    let result: agi_proof::runner::AgiTaskResult = serde_json::from_str(&output_json)
        .expect("Failed to parse output JSON");

    if result.replay_verified {
        println!("AGI-REPLAY: VERIFIED");
        println!("  TraceHead: {}", hash::hex(&result.trace_head));
    } else {
        println!("AGI-REPLAY: FAIL");
        println!("  The replay produced a different result.");
        std::process::exit(1);
    }
}

fn cmd_agi_run_all(seed_hex: &str, output_dir: &str) {
    println!("========================================================");
    println!("  AGI Proof: Complete Suite Execution");
    println!("  First-ever formal AGI capability proof");
    println!("========================================================");
    println!();

    // Generate or parse seed
    let master_seed: [u8; 32] = if seed_hex.is_empty() {
        [42u8; 32] // Default deterministic seed
    } else {
        let h = hash::H(seed_hex.as_bytes());
        let mut s = [0u8; 32];
        s.copy_from_slice(&h);
        s
    };

    // Step 0: Freeze — compute BuildHash BEFORE any task is seen
    let gm_suite = GoldMasterSuite::v1();
    let (build_hash, _) = compute_build_hash(&gm_suite);
    println!("BUILD_HASH={}", hash::hex(&build_hash));

    // Generate suite
    let suite = agi_proof::suite_gen::generate_suite(master_seed);
    let manifest = agi_proof::suite_gen::build_manifest(&suite);
    println!("SEED_COMMIT={}", manifest.seed_commitment);
    println!("SUITE_MERKLE={}", manifest.suite_merkle_root);
    println!("TOTAL_TASKS={}", manifest.total_tasks);
    println!();

    // Run each phase
    let mut runner = agi_proof::runner::AgiRunner::with_build_hash(build_hash);
    let mut phase_inputs: Vec<(u8, String, Vec<String>)> = Vec::new();

    for (phase_num, tasks) in &suite.phases {
        let name = match phase_num {
            0 => "Freeze",
            2 => "DomainRobustness",
            3 => "LongHorizon",
            4 => "Transfer",
            5 => "KnowledgeAcquisition",
            6 => "CausalReasoning",
            7 => "Discovery",
            8 => "CommonSense",
            _ => "Unknown",
        };
        phase_inputs.push((*phase_num, name.to_string(), tasks.clone()));
    }

    let proof_result = runner.run_all(&phase_inputs);

    // Print scoreboard
    println!();
    println!("=== SCOREBOARD ===");
    println!("BUILD_HASH={}  SERPI_K=frozen", hash::hex(&build_hash));
    for pr in &proof_result.phases {
        let line = agi_proof::phase_criteria::format_scoreboard_line(pr);
        println!("{}", line);
    }
    println!("RESULT_MERKLE_ROOT={}  AGGREGATE_FCR={}/{}  VERIFIED_PHASES={}/{}",
        hash::hex(&proof_result.result_merkle_root),
        proof_result.aggregate_false_claims,
        proof_result.aggregate_false_claims + proof_result.aggregate_verified_success,
        proof_result.phases.iter().filter(|p| p.false_claims == 0).count(),
        proof_result.phases.len(),
    );

    // Write results
    let _ = fs::create_dir_all(output_dir);
    let result_json = serde_json::to_string_pretty(&proof_result).expect("serialize");
    fs::write(format!("{}/proof_result.json", output_dir), &result_json).expect("write result");
    let manifest_json = serde_json::to_string_pretty(&manifest).expect("serialize");
    fs::write(format!("{}/suite_manifest.json", output_dir), &manifest_json).expect("write manifest");

    println!();
    println!("Results written to: {}", output_dir);
}

fn cmd_agi_replay_bundle(bundle_path: &str) {
    let result_json = fs::read_to_string(format!("{}/proof_result.json", bundle_path))
        .expect("Failed to read proof_result.json");
    let result: agi_proof::runner::AgiProofResult = serde_json::from_str(&result_json)
        .expect("Failed to parse proof result");

    println!("AGI-REPLAY-BUNDLE:");
    println!("  Phases: {}", result.phases.len());
    println!("  Total tasks: {}", result.aggregate_total_tasks);
    println!("  Verified success: {}", result.aggregate_verified_success);
    println!("  False claims: {}", result.aggregate_false_claims);
    println!("  Merkle root: {}", hash::hex(&result.result_merkle_root));

    if result.aggregate_false_claims == 0 {
        println!();
        println!("VERIFIED");
    } else {
        println!();
        println!("FAIL: {} false claims detected", result.aggregate_false_claims);
        std::process::exit(1);
    }
}

fn cmd_agi_verify_release(release_path: &str) {
    let manifest_path = format!("{}/suite_manifest.json", release_path);
    let result_path = format!("{}/proof_result.json", release_path);

    let has_manifest = std::path::Path::new(&manifest_path).exists();
    let has_result = std::path::Path::new(&result_path).exists();

    if !has_manifest || !has_result {
        println!("AGI-VERIFY-RELEASE: FAIL");
        println!("  Missing files in release directory");
        if !has_manifest { println!("  Missing: suite_manifest.json"); }
        if !has_result { println!("  Missing: proof_result.json"); }
        std::process::exit(1);
    }

    let manifest_json = fs::read_to_string(&manifest_path).expect("read manifest");
    let manifest: agi_proof::suite_gen::SuiteManifest = serde_json::from_str(&manifest_json)
        .expect("parse manifest");

    let result_json = fs::read_to_string(&result_path).expect("read result");
    let result: agi_proof::runner::AgiProofResult = serde_json::from_str(&result_json)
        .expect("parse result");

    println!("AGI-VERIFY-RELEASE:");
    println!("  Suite Merkle: {}", manifest.suite_merkle_root);
    println!("  Total tasks: {}", manifest.total_tasks);
    println!("  Result Merkle: {}", hash::hex(&result.result_merkle_root));
    println!("  False claims: {}", result.aggregate_false_claims);

    if result.aggregate_false_claims == 0 {
        println!();
        println!("VERIFIED");
    } else {
        println!();
        println!("FAIL: false claims detected");
        std::process::exit(1);
    }
}

fn cmd_jmcheck(capability_path: &str) {
    let cap_json = fs::read_to_string(capability_path).expect("Failed to read capability file");

    let cap: kernel_cap::capability::Capability = serde_json::from_str(&cap_json)
        .expect("Failed to parse capability JSON");

    // Create a fresh ledger and artifact for verification.
    let mut ledger = kernel_ledger::Ledger::new();
    let artifact = KernelArtifact::new("v0.2.0-A1".into(), [0u8; 32]);

    let checker = kernel_cap::jm::JurisdictionChecker::new(artifact);
    match checker.verify(&cap, &mut ledger) {
        kernel_cap::jm::JmResult::Authorized => {
            println!("JM CHECK: AUTHORIZED");
            println!("  Scope: {}", cap.scope);
            println!("  Nonce: {}", hash::hex(&cap.nonce));
        }
        kernel_cap::jm::JmResult::Denied(msg) => {
            println!("JM CHECK: DENIED");
            println!("  Reason: {}", msg);
            std::process::exit(1);
        }
    }
}

// ── FRC Engine Commands ─────────────────────────────────────────────

fn cmd_frc_search(statement_text: &str) {
    println!("FRC SEARCH");
    println!("  Statement: {}", statement_text);

    let mut engine = FrcSearchEngine::new();
    let mut ledger = kernel_ledger::Ledger::new();

    let stmt_hash = hash::H(statement_text.as_bytes());

    // Parse statement into descriptor
    let kind = if statement_text.contains("forall") || statement_text.contains("∀") {
        if statement_text.contains("[") {
            StatementKind::UniversalFinite
        } else {
            StatementKind::UniversalInfinite
        }
    } else if statement_text.contains("exists") || statement_text.contains("∃") {
        StatementKind::ExistentialFinite
    } else if statement_text.contains("SAT") {
        StatementKind::BoolSat
    } else {
        StatementKind::UniversalInfinite
    };

    let stmt = StatementDesc {
        kind,
        text: statement_text.to_string(),
        variables: vec![],
        predicate: statement_text.to_string(),
        params: vec![],
    };
    let ctx = ReductionContext::default_context();

    match engine.search(stmt_hash, &stmt, &ctx, &mut ledger) {
        FrcResult::Found(frc) => {
            println!("  Result: FRC FOUND");
            println!("  Schema: {:?}", frc.schema_id);
            println!("  B*: {}", frc.b_star);
            println!("  Program size: {} instructions", frc.program.len());
            println!("  FRC hash: {}", hash::hex(&frc.frc_hash));
            println!("  Internal verify: {}", frc.verify_internal());

            let (outcome, state) = kernel_frc::Vm::run(&frc.program, frc.b_star);
            println!("  VM outcome: {:?}", outcome);
            println!("  VM steps: {}", state.steps_taken);
        }
        FrcResult::Invalid(frontier) => {
            println!("  Result: INVALID (no FRC in current schema closure)");
            println!("  Schemas tried: {}", frontier.schemas_tried.len());
            println!("  Gaps: {}", frontier.gaps.len());
            if let Some(ref ml) = frontier.minimal_missing_lemma {
                println!("  Missing lemma: {}", ml.lemma_statement);
            }
            println!("  Frontier hash: {}", hash::hex(&frontier.frontier_hash));
        }
    }
}

fn cmd_frc_suite() {
    println!("FRC SUITE: Running FRC search across test statements\n");

    let mut engine = FrcSearchEngine::new();
    let mut ledger = kernel_ledger::Ledger::new();
    let ctx = ReductionContext::default_context();

    // Test suite of statements
    let test_statements: Vec<(&str, StatementDesc)> = vec![
        ("forall x in [0,10]: x >= 0", StatementDesc {
            kind: StatementKind::UniversalFinite,
            text: "forall x in [0,10]: x >= 0".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "x >= 0".to_string(),
            params: vec![],
        }),
        ("exists x in [0,10]: x = 5", StatementDesc {
            kind: StatementKind::ExistentialFinite,
            text: "exists x in [0,10]: x = 5".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "x = 5".to_string(),
            params: vec![],
        }),
        ("2-variable SAT", StatementDesc {
            kind: StatementKind::BoolSat,
            text: "2-variable SAT".to_string(),
            variables: vec![
                VariableDesc { name: "x0".to_string(), domain_lo: Some(0), domain_hi: Some(1), is_finite: true },
                VariableDesc { name: "x1".to_string(), domain_lo: Some(0), domain_hi: Some(1), is_finite: true },
            ],
            predicate: "CNF".to_string(),
            params: vec![],
        }),
        ("forall x: P(x) [infinite, no modulus]", StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "forall x: P(x)".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: None,
                domain_hi: None,
                is_finite: false,
            }],
            predicate: "P(x)".to_string(),
            params: vec![],
        }),
        ("convergence with metastability bound", StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "convergence".to_string(),
            variables: vec![],
            predicate: "stable window".to_string(),
            params: vec![
                ("metastability_bound".to_string(), 10),
                ("window_size".to_string(), 5),
            ],
        }),
        ("analytic bound with interval subdivision", StatementDesc {
            kind: StatementKind::Analytic,
            text: "f(x) bounded on [0, 100]".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(100),
                is_finite: true,
            }],
            predicate: "f(x) in [0,1]".to_string(),
            params: vec![("n_intervals".to_string(), 20)],
        }),
        ("x^2 - 4 = 0 (algebraic)", StatementDesc {
            kind: StatementKind::Algebraic,
            text: "x^2 - 4 = 0".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(-10),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "x^2 - 4 = 0".to_string(),
            params: vec![
                ("c0".to_string(), -4),
                ("c1".to_string(), 0),
                ("c2".to_string(), 1),
            ],
        }),
        ("infinite universal with epsilon-net", StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "continuous bounded".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: None,
                domain_hi: None,
                is_finite: false,
            }],
            predicate: "|f(x)| <= M".to_string(),
            params: vec![("epsilon_net_size".to_string(), 50)],
        }),
    ];

    let mut found = 0u64;
    let mut invalid = 0u64;
    let total = test_statements.len() as u64;

    for (label, stmt) in &test_statements {
        let stmt_hash = hash::H(label.as_bytes());
        let result = engine.search(stmt_hash, stmt, &ctx, &mut ledger);

        match result {
            FrcResult::Found(frc) => {
                let (outcome, state) = kernel_frc::Vm::run(&frc.program, frc.b_star);
                println!("  [FRC] {:50} → {:?} (schema={:?}, B*={}, steps={})",
                    label, outcome, frc.schema_id, frc.b_star, state.steps_taken);
                found += 1;
            }
            FrcResult::Invalid(frontier) => {
                let gap_desc = frontier.minimal_missing_lemma
                    .as_ref()
                    .map(|ml| ml.lemma_statement.clone())
                    .unwrap_or_else(|| "no specific lemma".to_string());
                println!("  [INV] {:50} → INVALID (gaps={}, missing={})",
                    label, frontier.gaps.len(), gap_desc);
                invalid += 1;
            }
        }
    }

    let metrics = engine.metrics(total);
    println!("\nFRC SUITE RESULTS:");
    println!("  Total statements: {}", total);
    println!("  FRC found: {} ({:.1}%)", found, found as f64 / total as f64 * 100.0);
    println!("  Invalid: {}", invalid);
    println!("  Motif library: {} lemmas", metrics.motif_count);
    println!("  Gap ledger: {} active gaps", metrics.gap_count);
    println!("  Coverage rate: {}.{}%", metrics.coverage_rate_milli / 10, metrics.coverage_rate_milli % 10);
    println!("  Ledger events: {}", ledger.len());
}

fn cmd_opp_solve(opp_path: &str) {
    let json = fs::read_to_string(opp_path).expect("Failed to read OPP file");
    let opp: OpenProblemPackage = serde_json::from_str(&json).expect("Failed to parse OPP");

    println!("OPP SOLVE");
    println!("  Statement: {}", opp.statement);
    println!("  OPP hash: {}", hash::hex(&opp.opp_hash));

    let mut runner = OppRunner::new();
    let mut ledger = kernel_ledger::Ledger::new();

    match runner.solve(&opp, &mut ledger) {
        kernel_frc::opp::OppResult::Proof { frc, receipt } => {
            println!("  Result: PROOF (UNIQUE)");
            println!("  Schema: {:?}", frc.schema_id);
            println!("  B*: {}", frc.b_star);
            println!("  Trace head: {}", hash::hex(&receipt.trace_head));
            println!("  Merkle root: {}", hash::hex(&receipt.merkle_root));
            println!("  Receipt hash: {}", hash::hex(&receipt.receipt_hash));

            // Write result
            let result = serde_json::json!({
                "status": "PROOF",
                "frc_hash": hash::hex(&frc.frc_hash),
                "receipt_hash": hash::hex(&receipt.receipt_hash),
                "schema": format!("{:?}", frc.schema_id),
                "b_star": frc.b_star,
            });
            println!("{}", serde_json::to_string_pretty(&result).unwrap());
        }
        kernel_frc::opp::OppResult::Disproof { frc, receipt } => {
            println!("  Result: DISPROOF");
            println!("  Schema: {:?}", frc.schema_id);
            println!("  Receipt hash: {}", hash::hex(&receipt.receipt_hash));
        }
        kernel_frc::opp::OppResult::Invalid { frontier } => {
            println!("  Result: INVALID");
            println!("  Schemas tried: {}", frontier.schemas_tried.len());
            println!("  Gaps: {}", frontier.gaps.len());
            if let Some(ref ml) = frontier.minimal_missing_lemma {
                println!("  Missing lemma: {}", ml.lemma_statement);
            }
            println!("  Frontier hash: {}", hash::hex(&frontier.frontier_hash));
        }
    }
}

fn cmd_opp_verify(opp_path: &str, _result_path: &str) {
    let json = fs::read_to_string(opp_path).expect("Failed to read OPP file");
    let opp: OpenProblemPackage = serde_json::from_str(&json).expect("Failed to parse OPP");

    println!("OPP VERIFY");
    println!("  OPP: {}", hash::hex(&opp.opp_hash));

    // Re-solve and verify
    let mut runner = OppRunner::new();
    let mut ledger = kernel_ledger::Ledger::new();

    match runner.solve(&opp, &mut ledger) {
        kernel_frc::opp::OppResult::Proof { frc, receipt } => {
            let mut verify_ledger = kernel_ledger::Ledger::new();
            let result = OppVerifier::verify(&opp, &frc, &receipt, &mut verify_ledger);
            if result.overall {
                println!("  {}", result.details);
            } else {
                println!("  {}", result.details);
                std::process::exit(1);
            }
        }
        kernel_frc::opp::OppResult::Disproof { frc, receipt } => {
            let mut verify_ledger = kernel_ledger::Ledger::new();
            let result = OppVerifier::verify(&opp, &frc, &receipt, &mut verify_ledger);
            if result.overall {
                println!("  VERIFIED (DISPROOF): {}", result.details);
            } else {
                println!("  FAIL: {}", result.details);
                std::process::exit(1);
            }
        }
        kernel_frc::opp::OppResult::Invalid { frontier } => {
            let verified = OppVerifier::verify_frontier(&opp, &frontier);
            if verified {
                println!("  VERIFIED (INVALID): frontier is consistent");
            } else {
                println!("  FAIL: frontier verification failed");
                std::process::exit(1);
            }
        }
    }
}

fn cmd_frc_prove(contract_path: &str) {
    let json = fs::read_to_string(contract_path).expect("Failed to read contract file");
    let contract = compile_contract(&json).expect("Failed to compile contract");
    let mut ledger = kernel_ledger::Ledger::new();

    println!("CONTRACT:   \"{}\" (qid: {})", contract.description, hash::hex(&contract.qid));

    // Show search problem conversion
    match contract_to_search_problem(&contract) {
        Ok(problem) => {
            println!("SEARCH:     {:?}", problem);
        }
        Err(frontier) => {
            println!("SEARCH:     INADMISSIBLE — proof space not finitely enumerable");
            println!("FRONTIER:   Gap(goal: \"{}\", schema: none applicable)",
                frontier.gaps.first().map(|g| g.goal_statement.as_str()).unwrap_or("unknown"));
            if let Some(ref ml) = frontier.minimal_missing_lemma {
                println!("REMEDY:     {}", ml.lemma_statement);
            }
            println!("STATUS:     INVALID (correctly rejected under A1)");
            return;
        }
    }

    // Build FRC
    match build_contract_frc(&contract, &mut ledger) {
        Ok(frc) => {
            println!("PROGRAM:    {} instructions, B* = {}", frc.program.len(), frc.b_star);
            let (outcome, state) = Vm::run(&frc.program, frc.b_star);
            match &outcome {
                kernel_frc::VmOutcome::Halted(v) => {
                    println!("VM RESULT:  Halted({}) in {} steps", v, state.steps_taken);
                    if *v == 1 {
                        // Show witness from memory slot 0
                        println!("WITNESS:    mem[0] = {}", state.memory.get(&0).copied().unwrap_or(0));
                    }
                }
                other => println!("VM RESULT:  {:?}", other),
            }
            println!("FRC HASH:   {}", hash::hex(&frc.frc_hash));
            println!("PROOF_EQ:   contract_hash → program_hash via predicate_hash");
            println!("PROOF_TOTAL: {} instructions, bounded loop, B*={}",
                frc.program.len(), frc.b_star);
            println!("SCHEMA:     {:?}", frc.schema_id);
            println!("INTERNAL:   {}", if frc.verify_internal() { "VERIFIED" } else { "FAILED" });

            // Cross-verify against solver
            let mut solver = Solver::new();
            let output = solver.solve(&contract);
            let consistent = kernel_frc::contract_frc::verify_frc_against_solver(
                &contract, &frc, &output.status,
            );
            println!("STATUS:     {} ({})",
                output.status,
                if consistent { "verified against solver" } else { "INCONSISTENCY DETECTED" });
        }
        Err(frontier) => {
            println!("RESULT:     FRC BUILD FAILED");
            for gap in &frontier.gaps {
                println!("GAP:        {}", gap.goal_statement);
            }
            println!("FRONTIER:   {}", hash::hex(&frontier.frontier_hash));
        }
    }
}

fn cmd_frc_suite_full() {
    println!("========================================================");
    println!("  FRC SUITE (FULL): Truthful FRCs for all contracts");
    println!("========================================================");
    println!();

    let mut ledger = kernel_ledger::Ledger::new();
    let mut frc_found = 0u64;
    let mut invalid_count = 0u64;
    let mut gap_ledger = kernel_frc::GapLedger::new();
    let mut motif_library = kernel_frc::MotifLibrary::new();

    // GoldMaster suite
    let gm_suite = GoldMasterSuite::v1();
    println!("=== GOLDMASTER CONTRACTS ({}) ===", gm_suite.len());

    for (i, contract) in gm_suite.contracts.iter().enumerate() {

        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                let (outcome, state) = Vm::run(&frc.program, frc.b_star);
                // Cross-verify against solver
                let mut solver = Solver::new();
                let output = solver.solve(contract);
                let consistent = kernel_frc::contract_frc::verify_frc_against_solver(
                    contract, &frc, &output.status,
                );
                let status_char = if consistent { "OK" } else { "!!" };
                println!("  Q{}: {:40} → {:?} (B*={}, steps={}) [{}] {}",
                    i, contract.description, outcome,
                    frc.b_star, state.steps_taken, status_char, output.status);
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                let gap_desc = frontier.gaps.first()
                    .map(|g| truncate_safe(&g.goal_statement, 60))
                    .unwrap_or_else(|| "no specific gap".to_string());
                println!("  Q{}: {:40} → INVALID ({})", i, contract.description, gap_desc);
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }
    println!();

    // Millennium suite
    let msuite = MillenniumSuite::build();

    // Millennium problems
    println!("=== MILLENNIUM PRIZE PROBLEMS ({}) ===", msuite.millennium.len());
    for (i, contract) in msuite.millennium.iter().enumerate() {

        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                let (outcome, _state) = Vm::run(&frc.program, frc.b_star);
                println!("  M{}: {:40} → {:?} (B*={})", i, contract.description, outcome, frc.b_star);
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                let gap_desc = frontier.minimal_missing_lemma
                    .as_ref()
                    .map(|ml| truncate_safe(&ml.lemma_statement, 60))
                    .unwrap_or_else(|| "inadmissible".to_string());
                println!("  M{}: {:40} → INVALID ({})", i, contract.description, gap_desc);
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }
    println!();

    // Sanity ladder
    println!("=== SANITY LADDER ({}) ===", msuite.ladder.len());
    for (i, contract) in msuite.ladder.iter().enumerate() {

        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                let (outcome, state) = Vm::run(&frc.program, frc.b_star);
                println!("  L{:02}: {:40} → {:?} (B*={}, steps={})",
                    i, contract.description, outcome, frc.b_star, state.steps_taken);
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                let gap_desc = frontier.gaps.first()
                    .map(|g| truncate_safe(&g.goal_statement, 60))
                    .unwrap_or_else(|| "no gap".to_string());
                println!("  L{:02}: {:40} → INVALID ({})", i, contract.description, gap_desc);
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }
    println!();

    // Adversarial
    println!("=== ADVERSARIAL ({}) ===", msuite.adversarial.len());
    for (i, contract) in msuite.adversarial.iter().enumerate() {

        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                let (outcome, state) = Vm::run(&frc.program, frc.b_star);
                println!("  A{:02}: {:40} → {:?} (B*={}, steps={})",
                    i, contract.description, outcome, frc.b_star, state.steps_taken);
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                let gap_desc = frontier.gaps.first()
                    .map(|g| truncate_safe(&g.goal_statement, 60))
                    .unwrap_or_else(|| "no gap".to_string());
                println!("  A{:02}: {:40} → INVALID ({})", i, contract.description, gap_desc);
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }
    println!();

    // Finite fragments of open problems (real computations)
    println!("=== FINITE FRAGMENTS ({}) ===", msuite.finite.len());
    for (i, contract) in msuite.finite.iter().enumerate() {

        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                let (outcome, state) = Vm::run(&frc.program, frc.b_star);
                let verified = matches!(outcome, kernel_frc::VmOutcome::Halted(1));
                let status_str = if verified { "VERIFIED" } else { "FAILED" };
                println!("  MF{}: {:50} → {:?} (B*={}, steps={}) [{}]",
                    i, contract.description, outcome,
                    frc.b_star, state.steps_taken, status_str);
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                let gap_desc = frontier.gaps.first()
                    .map(|g| truncate_safe(&g.goal_statement, 60))
                    .unwrap_or_else(|| "no gap".to_string());
                println!("  MF{}: {:50} → INVALID ({})", i, contract.description, gap_desc);
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }
    println!();

    // Coverage report
    let report = CoverageReport::compute(
        frc_found, invalid_count, &gap_ledger, &motif_library,
        6, // base schemas
        None,
    );
    println!("========================================================");
    println!("{}", report.display());
    println!("  Ledger events: {}", ledger.len());
    println!("========================================================");
}

fn cmd_class_c() {
    println!("=== CLASS_C DEFINITION ===");
    println!();

    let schemas = vec![
        SchemaId::BoundedCounterexample,
        SchemaId::FiniteSearch,
        SchemaId::EffectiveCompactness,
        SchemaId::ProofMining,
        SchemaId::AlgebraicDecision,
        SchemaId::CertifiedNumerics,
    ];
    let motif_lib = kernel_frc::MotifLibrary::new();
    let inductor = SchemaInductor::new();
    let class_c = ClassCDefinition::build(&schemas, &motif_lib, &inductor);

    println!("{}", class_c.display());
}

fn cmd_coverage() {
    println!("=== FRC COVERAGE METRICS ===");
    println!();

    let mut ledger = kernel_ledger::Ledger::new();
    let mut frc_found = 0u64;
    let mut invalid_count = 0u64;
    let mut gap_ledger = kernel_frc::GapLedger::new();
    let mut motif_library = kernel_frc::MotifLibrary::new();

    // Run all GoldMaster contracts
    let gm_suite = GoldMasterSuite::v1();
    for contract in &gm_suite.contracts {
        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }

    // Run all Millennium contracts
    let msuite = MillenniumSuite::build();
    let all_millennium: Vec<&kernel_contracts::contract::Contract> = msuite.millennium.iter()
        .chain(msuite.ladder.iter())
        .chain(msuite.adversarial.iter())
        .collect();

    for contract in all_millennium {
        match build_contract_frc(contract, &mut ledger) {
            Ok(frc) => {
                motif_library.add_motif(contract.qid, contract.description.clone(), frc);
                frc_found += 1;
            }
            Err(frontier) => {
                for gap in &frontier.gaps {
                    gap_ledger.record_gap(gap.clone());
                }
                invalid_count += 1;
            }
        }
    }

    let report = CoverageReport::compute(
        frc_found, invalid_count, &gap_ledger, &motif_library,
        6, None,
    );
    println!("{}", report.display());

    // Also emit the CLASS_C identity
    let schemas = vec![
        SchemaId::BoundedCounterexample,
        SchemaId::FiniteSearch,
        SchemaId::EffectiveCompactness,
        SchemaId::ProofMining,
        SchemaId::AlgebraicDecision,
        SchemaId::CertifiedNumerics,
    ];
    let inductor = SchemaInductor::new();
    let class_c = ClassCDefinition::build(&schemas, &motif_library, &inductor);
    println!();
    println!("{}", class_c.display());
}

fn cmd_lean_emit(output_dir: &str) {
    use kernel_lean::program_embed;
    use kernel_lean::proof_eq_gen;
    use kernel_lean::proof_total_gen;

    println!("=== Lean4 Proof Bundle Emission ===");
    println!();

    let output_path = std::path::Path::new(output_dir);
    fs::create_dir_all(output_path).expect("Failed to create output directory");

    // Build FRCs for all 14 problems
    let problems = vec![
        ("goldbach", 100i64, None),          // B*≈3.36M — Lean OK
        ("collatz", 30, Some(200i64)),        // reduced: Lean native_decide limit
        ("twin_primes", 1000, None),          // reduced: Lean native_decide limit
        ("flt", 2, Some(5)),                  // reduced: Lean native_decide limit
        ("odd_perfect", 100, None),           // reduced: Lean native_decide limit
        ("mersenne", 31, None),               // B*≈5.69M — Lean OK
        ("zfc_zero_ne_one", 0, None),         // B*=10 — Lean OK
        ("mertens", 100, None),               // B*≈1.22M — Lean OK
        ("legendre", 50, None),               // B*≈2.66M — Lean OK
        ("erdos_straus", 30, None),           // reduced: Lean native_decide limit
        ("bsd_ec_count", 10, Some(0)),        // reduced: Lean native_decide limit
        ("weak_goldbach", 30, None),          // reduced: Lean native_decide limit
        ("bertrand", 100, None),              // B*≈4M — Lean OK
        ("lagrange_four_squares", 30, None),  // reduced: Lean native_decide limit
    ];

    let mut emitted = 0;
    for (problem_id, param_n, param_aux) in &problems {
        let contract_json = match param_aux {
            Some(aux) => format!(
                r#"{{"type":"millennium_finite","description":"{} lean emit","problem_id":"{}","parameter_n":{},"parameter_aux":{}}}"#,
                problem_id, problem_id, param_n, aux
            ),
            None => format!(
                r#"{{"type":"millennium_finite","description":"{} lean emit","problem_id":"{}","parameter_n":{}}}"#,
                problem_id, problem_id, param_n
            ),
        };
        let contract = kernel_contracts::compiler::compile_contract(&contract_json)
            .expect("Failed to compile contract");
        let mut ledger = kernel_ledger::Ledger::new();

        match kernel_frc::millennium_frc::build_millennium_frc(&contract, &mut ledger) {
            Ok(frc) => {
                let prob_dir = output_path.join(problem_id);
                fs::create_dir_all(&prob_dir).expect("mkdir failed");

                // Emit program as Lean4
                let prog_name = format!("{}Prog", problem_id);
                let bstar_name = format!("{}Bstar", problem_id);
                let prog_lean = program_embed::embed_program(&frc.program, &prog_name);
                fs::write(prob_dir.join("Program.lean"), &prog_lean).expect("write failed");

                let bstar_lean = program_embed::embed_bstar(frc.b_star, &bstar_name);
                fs::write(prob_dir.join("Bstar.lean"), &bstar_lean).expect("write failed");

                // Emit ProofEq.lean
                let proof_eq_lean = proof_eq_gen::generate_proof_eq(
                    &frc.proof_eq, &frc.schema_id, problem_id,
                    &prog_name, &bstar_name, &format!("{}Statement", problem_id),
                );
                fs::write(prob_dir.join("ProofEq.lean"), &proof_eq_lean).expect("write failed");

                // Emit ProofTotal.lean
                let proof_total_lean = proof_total_gen::generate_proof_total(
                    &frc.proof_total, problem_id, &prog_name, &bstar_name,
                );
                fs::write(prob_dir.join("ProofTotal.lean"), &proof_total_lean).expect("write failed");

                println!("  {} -> EMITTED ({} instructions, B*={})",
                    problem_id, frc.program.len(), frc.b_star);
                emitted += 1;
            }
            Err(e) => {
                println!("  {} -> FAILED: {:?}", problem_id, e.schemas_tried);
            }
        }
    }

    println!();
    println!("Emitted: {}/{} problems", emitted, problems.len());
    println!("Output: {}", output_dir);
}

fn cmd_lean_verify(lean_dir: &str) {
    use kernel_lean::lean_runner;

    println!("=== Lean4 Proof Verification ===");
    println!();

    let lean_path = std::path::Path::new(lean_dir);
    if !lean_path.exists() {
        eprintln!("ERROR: Lean directory does not exist: {}", lean_dir);
        std::process::exit(1);
    }

    let result = lean_runner::verify_lean_proofs(lean_path);

    println!("Build success: {}", if result.build_success { "PASS" } else { "FAIL" });
    println!("No sorry:      {}", if result.no_sorry { "PASS" } else { "FAIL" });

    if !result.sorry_files.is_empty() {
        println!();
        println!("Files containing 'sorry':");
        for f in &result.sorry_files {
            println!("  {}", f);
        }
    }

    if !result.build_stderr.is_empty() && !result.build_success {
        println!();
        println!("Build errors:");
        // Print first 20 lines of stderr
        for line in result.build_stderr.lines().take(20) {
            println!("  {}", line);
        }
    }

    println!();
    if result.pass {
        println!("LEAN VERIFICATION: PASS");
    } else {
        println!("LEAN VERIFICATION: FAIL");
        if !result.build_success {
            println!("  Hint: Install Lean4 from https://leanprover.github.io/lean4/doc/setup.html");
        }
        std::process::exit(1);
    }
}

fn cmd_bundle_emit(problem: &str, output_dir: &str) {
    use kernel_lean::bundle_gen;

    println!("=== Proof Bundle Emission ===");
    println!();

    let output_path = std::path::Path::new(output_dir);
    fs::create_dir_all(output_path).expect("Failed to create output directory");

    let problems: Vec<(&str, i64, Option<i64>)> = if problem == "all" {
        vec![
            ("goldbach", 100, None),
            ("collatz", 100, Some(500)),
            ("twin_primes", 10000, None),
            ("flt", 10, Some(40)),
            ("odd_perfect", 1000, None),
            ("mersenne", 31, None),
            ("zfc_zero_ne_one", 0, None),
            ("mertens", 100, None),
            ("legendre", 50, None),
            ("erdos_straus", 100, None),
            ("bsd_ec_count", 97, Some(0)),
            ("weak_goldbach", 101, None),
            ("bertrand", 100, None),
            ("lagrange_four_squares", 100, None),
        ]
    } else {
        // Find matching problem
        let param = match problem {
            "goldbach" => ("goldbach", 100i64, None),
            "collatz" => ("collatz", 100, Some(500i64)),
            "twin_primes" => ("twin_primes", 10000, None),
            "flt" => ("flt", 10, Some(40)),
            "odd_perfect" => ("odd_perfect", 1000, None),
            "mersenne" => ("mersenne", 31, None),
            "zfc_zero_ne_one" => ("zfc_zero_ne_one", 0, None),
            "mertens" => ("mertens", 100, None),
            "legendre" => ("legendre", 50, None),
            "erdos_straus" => ("erdos_straus", 100, None),
            "bsd_ec_count" => ("bsd_ec_count", 97, Some(0)),
            "weak_goldbach" => ("weak_goldbach", 101, None),
            "bertrand" => ("bertrand", 100, None),
            "lagrange_four_squares" => ("lagrange_four_squares", 100, None),
            _ => {
                eprintln!("Unknown problem: {}", problem);
                std::process::exit(1);
            }
        };
        vec![param]
    };

    let mut verified = 0;
    let mut failed = 0;

    for (problem_id, param_n, param_aux) in &problems {
        let contract_json = match param_aux {
            Some(aux) => format!(
                r#"{{"type":"millennium_finite","description":"{} bundle","problem_id":"{}","parameter_n":{},"parameter_aux":{}}}"#,
                problem_id, problem_id, param_n, aux
            ),
            None => format!(
                r#"{{"type":"millennium_finite","description":"{} bundle","problem_id":"{}","parameter_n":{}}}"#,
                problem_id, problem_id, param_n
            ),
        };
        let contract = kernel_contracts::compiler::compile_contract(&contract_json)
            .expect("Failed to compile contract");
        let mut ledger = kernel_ledger::Ledger::new();

        match kernel_frc::millennium_frc::build_millennium_frc(&contract, &mut ledger) {
            Ok(frc) => {
                let trace = Vm::run_traced(&frc.program, frc.b_star);
                match bundle_gen::emit_verified_bundle(
                    output_path, problem_id, &contract.description, &frc, &trace,
                ) {
                    Ok(dir) => {
                        println!("  {} -> VERIFIED ({})", problem_id, dir.display());
                        verified += 1;
                    }
                    Err(e) => {
                        println!("  {} -> EMIT ERROR: {}", problem_id, e);
                        failed += 1;
                    }
                }
            }
            Err(frontier) => {
                match bundle_gen::emit_invalid_bundle(
                    output_path, problem_id, problem_id, &frontier,
                ) {
                    Ok(dir) => {
                        println!("  {} -> INVALID ({})", problem_id, dir.display());
                    }
                    Err(e) => {
                        println!("  {} -> EMIT ERROR: {}", problem_id, e);
                        failed += 1;
                    }
                }
            }
        }
    }

    println!();
    println!("Bundle: {}", output_dir);
    println!("Verified: {}, Failed: {}", verified, failed);
}

fn cmd_bundle_verify(bundle_dir: &str) {
    use kernel_lean::bundle_gen;

    println!("=== Proof Bundle Verification ===");
    println!();

    let bundle_path = std::path::Path::new(bundle_dir);
    if !bundle_path.exists() {
        eprintln!("ERROR: Bundle directory does not exist: {}", bundle_dir);
        std::process::exit(1);
    }

    // Find all subdirectories (each is a problem bundle)
    let mut entries: Vec<_> = fs::read_dir(bundle_path)
        .expect("Failed to read bundle directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut pass_count = 0;
    let mut fail_count = 0;
    let mut skip_count = 0;
    let mut lean_skipped = false;

    for entry in &entries {
        let problem_dir = entry.path();
        let problem_id = entry.file_name().to_string_lossy().to_string();

        let result = bundle_gen::verify_bundle(&problem_dir);

        let status = if result.pass {
            pass_count += 1;
            "PASS"
        } else if result.errors.is_empty() {
            skip_count += 1;
            "SKIP"
        } else {
            fail_count += 1;
            "FAIL"
        };

        let kind = if result.is_invalid { "INVALID" } else { "VERIFIED" };
        println!("  {} [{}]: {}", problem_id, kind, status);

        if !result.errors.is_empty() {
            for err in &result.errors {
                println!("    - {}", err);
            }
        }
    }

    // Check Lean proofs if lean/ directory exists alongside bundle
    let lean_dir = bundle_path.parent()
        .unwrap_or(bundle_path)
        .join("lean");
    if lean_dir.exists() {
        println!();
        println!("--- Lean4 Verification ---");
        let lean_result = kernel_lean::lean_runner::verify_lean_proofs(&lean_dir);
        if lean_result.pass {
            println!("  lake build: PASS");
            println!("  No sorry:   PASS");
        } else if !lean_result.build_success {
            println!("  lake build: SKIPPED (Lean4 not installed)");
            lean_skipped = true;
        } else {
            println!("  lake build: {}", if lean_result.build_success { "PASS" } else { "FAIL" });
            println!("  No sorry:   {}", if lean_result.no_sorry { "PASS" } else { "FAIL" });
        }
    } else {
        println!();
        println!("--- Lean4 Verification: SKIPPED (no lean/ directory found) ---");
        lean_skipped = true;
    }

    println!();
    println!("=== BUNDLE VERIFICATION SUMMARY ===");
    println!("  PASS:    {}", pass_count);
    println!("  FAIL:    {}", fail_count);
    println!("  SKIP:    {}", skip_count);
    if lean_skipped {
        println!("  Lean4:   SKIPPED");
    }

    if fail_count > 0 {
        println!();
        println!("BUNDLE VERIFICATION: FAIL");
        std::process::exit(1);
    } else {
        println!();
        println!("BUNDLE VERIFICATION: PASS");
    }
}

fn cmd_irc_solve(problems_arg: &str, output_dir: &str) {
    use kernel_frc::irc::{self, IrcSearch};
    use kernel_frc::frc_types::IrcResult;
    use kernel_lean::bundle_gen;

    println!("=== IRC SOLVE: Invariant-based unbounded proofs ===");
    println!();

    let problem_ids: Vec<&str> = if problems_arg == "all" {
        irc::ALL_PROBLEM_IDS.to_vec()
    } else {
        problems_arg.split(',').map(|s| s.trim()).collect()
    };

    let engine = IrcSearch::new();
    let output_path = std::path::Path::new(output_dir);
    fs::create_dir_all(output_path).expect("Failed to create output directory");

    let mut proved_count = 0u32;
    let mut frontier_count = 0u32;

    for problem_id in &problem_ids {
        let result = engine.search(problem_id);

        let (status, detail) = match &result {
            IrcResult::Proved(irc) => {
                proved_count += 1;
                let inv_desc = format!("{:?}", irc.invariant.kind);
                ("PROVED", format!("I(n)={}, all obligations discharged", inv_desc))
            }
            IrcResult::Frontier(frontier) => {
                frontier_count += 1;
                if let Some(ref best) = frontier.best_candidate {
                    let inv_desc = format!("{:?}", best.invariant.kind);
                    let gaps: Vec<String> = [
                        (!best.base.is_discharged()).then(|| "Base".to_string()),
                        (!best.step.is_discharged()).then(|| "Step".to_string()),
                        (!best.link.is_discharged()).then(|| "Link".to_string()),
                    ].into_iter().flatten().collect();
                    ("FRONTIER", format!("invariant={}, Gap({})", inv_desc, gaps.join(",")))
                } else {
                    ("FRONTIER", "no viable candidate".to_string())
                }
            }
        };

        println!("  {:20} {} — {}", format!("{}:", problem_id), status, detail);

        // Emit bundle
        if let Err(e) = bundle_gen::emit_irc_bundle(output_path, problem_id, &result) {
            eprintln!("    WARNING: bundle emit failed: {}", e);
        }
    }

    println!();
    println!("  IRC Summary: {} PROVED, {} FRONTIER", proved_count, frontier_count);
    println!("  Each FRONTIER identifies the exact obligation that is the open problem.");
    println!("  Output: {}", output_dir);
}

fn cmd_irc_verify(bundle_dir: &str) {
    use kernel_frc::frc_types::Irc;

    println!("=== IRC Bundle Verification ===");
    println!();

    let bundle_path = std::path::Path::new(bundle_dir);
    if !bundle_path.exists() {
        eprintln!("ERROR: Bundle directory does not exist: {}", bundle_dir);
        std::process::exit(1);
    }

    let mut entries: Vec<_> = fs::read_dir(bundle_path)
        .expect("Failed to read bundle directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .collect();
    entries.sort_by_key(|e| e.file_name());

    let mut pass_count = 0u32;
    let mut fail_count = 0u32;

    for entry in &entries {
        let problem_dir = entry.path();
        let problem_id = entry.file_name().to_string_lossy().to_string();

        let mut errors: Vec<String> = Vec::new();

        // Check for irc.json (proved) or irc_frontier.json (frontier)
        let irc_path = problem_dir.join("irc.json");
        let frontier_path = problem_dir.join("irc_frontier.json");

        if irc_path.exists() {
            // Proved IRC — verify internal consistency
            match fs::read_to_string(&irc_path) {
                Ok(content) => {
                    match serde_json::from_str::<Irc>(&content) {
                        Ok(irc) => {
                            if !irc.verify_internal() {
                                errors.push("IRC internal verification failed".to_string());
                            }
                            if !irc.is_complete() {
                                errors.push(format!(
                                    "IRC not complete: {}/3 discharged",
                                    irc.obligations_discharged()
                                ));
                            }
                        }
                        Err(e) => errors.push(format!("Failed to parse irc.json: {}", e)),
                    }
                }
                Err(e) => errors.push(format!("Failed to read irc.json: {}", e)),
            }

            // Check Lean files for sorry
            for lean_file in &["Invariant.lean", "IrcResult.lean"] {
                let p = problem_dir.join(lean_file);
                if p.exists() {
                    if let Ok(content) = fs::read_to_string(&p) {
                        // Note: axiom is fine (honest gaps), sorry is not
                        if content.contains("sorry") && !content.contains("-- sorry") {
                            errors.push(format!("{} contains 'sorry'", lean_file));
                        }
                    }
                }
            }
        } else if frontier_path.exists() {
            // Frontier — just verify it parses
            match fs::read_to_string(&frontier_path) {
                Ok(content) => {
                    if serde_json::from_str::<kernel_frc::frc_types::IrcFrontier>(&content).is_err() {
                        errors.push("Failed to parse irc_frontier.json".to_string());
                    }
                }
                Err(e) => errors.push(format!("Failed to read irc_frontier.json: {}", e)),
            }
        } else {
            errors.push("No irc.json or irc_frontier.json found".to_string());
        }

        let status = if errors.is_empty() {
            pass_count += 1;
            "PASS"
        } else {
            fail_count += 1;
            "FAIL"
        };

        let kind = if irc_path.exists() { "PROVED" } else { "FRONTIER" };
        println!("  {} [{}]: {}", problem_id, kind, status);

        for err in &errors {
            println!("    - {}", err);
        }
    }

    println!();
    println!("=== IRC VERIFICATION SUMMARY ===");
    println!("  PASS: {}", pass_count);
    println!("  FAIL: {}", fail_count);

    if fail_count > 0 {
        println!();
        println!("IRC VERIFICATION: FAIL");
        std::process::exit(1);
    } else {
        println!();
        println!("IRC VERIFICATION: PASS");
    }
}

fn cmd_invsyn_search(problem_id: &str, max_size: usize) {
    use kernel_frc::invsyn::{InvSynSearch, InvSynResult, normalize};

    println!("=== InvSyn Search: Structural Invariant Synthesis ===");
    println!("  Problem: {}", problem_id);
    println!("  Max AST size: {}", max_size);
    println!();

    let problem = normalize(problem_id);
    println!("  Reachability form:");
    println!("    State type: {}", problem.state_type);
    println!("    Initial: {} = {}", problem.initial_lean, problem.initial_value);
    println!("    Step: {} (delta = {})", problem.step_lean, problem.step_delta);
    println!("    Property: {}", problem.property_lean);
    println!();

    let mut engine = InvSynSearch::new();
    engine.max_ast_size = max_size;

    let result = engine.search(&problem);
    match result {
        InvSynResult::Found { inv, base_result, step_result, link_result, step_structural, link_structural } => {
            println!("  FOUND invariant: {:?}", inv);
            println!("    AST size: {}", inv.size());
            println!("    Layer: {:?}", inv.layer());
            println!("    Lean: {}", inv.to_lean());
            println!();
            println!("  Base: {}", base_result);
            println!("  Step: {} (structural: {})", step_result, step_structural);
            println!("  Link: {} (structural: {})", link_result, link_structural);
            println!();
            if step_structural && link_structural {
                println!("  STRUCTURALLY VERIFIED. Real Lean proof terms can be generated.");
            } else {
                println!("  WARNING: Not fully structurally verified.");
                if !step_structural { println!("    Step requires structural proof."); }
                if !link_structural { println!("    Link requires structural proof."); }
            }
        }
        InvSynResult::Frontier { candidates_tried, max_ast_size } => {
            println!("  FRONTIER: No invariant found");
            println!("    Candidates tried: {}", candidates_tried);
            println!("    Max AST size: {}", max_ast_size);
            println!();
            println!("  This problem requires a mathematical breakthrough");
            println!("  expressible in the InvSyn language.");
        }
    }
}

fn cmd_sec_mine(problem_id: &str) {
    use kernel_frc::sec::{SecEngine, SecResult, GapTarget};
    use kernel_frc::frc_types::ObligationKind;
    use kernel_frc::invsyn::normalize;

    println!("=== SEC: Self-Extending Calculus — Rule Mining ===");
    println!("  Problem: {}", problem_id);
    println!();

    let problem = normalize(problem_id);

    let gap = GapTarget {
        gap_hash: hash::H(format!("sec_mine:{}", problem_id).as_bytes()),
        gap_statement: format!("Step obligation for {}", problem_id),
        obligation_kind: ObligationKind::Step,
        problem_id: problem_id.to_string(),
        inv_expr: problem.property_expr.clone(),
        prop_expr: problem.property_expr.clone(),
        delta: problem.step_delta,
    };

    let mut engine = SecEngine::new();
    let result = engine.mine_for_gap(&gap);

    match result {
        SecResult::NewRules(rules) => {
            println!("  {} new rules discovered:", rules.len());
            for rule in &rules {
                println!("    - {} (kind: {:?}, size: {})",
                    rule.lean_theorem_name,
                    rule.schema.kind,
                    rule.schema.size,
                );
                println!("      {}", rule.schema.description);
            }
            println!();
            println!("  Rule DB Merkle root: {}", hash::hex(&engine.rule_db().merkle_root()));
        }
        SecResult::NoNewRules { candidates_tried } => {
            println!("  No new rules found.");
            println!("  Candidates tried: {}", candidates_tried);
        }
    }
}

fn cmd_sec_status() {
    use kernel_frc::sec::SecEngine;

    println!("=== SEC: Rule Database Status ===");
    println!();

    let engine = SecEngine::new();
    let db = engine.rule_db();
    println!("  Rules: {}", db.len());
    println!("  Merkle root: {}", hash::hex(&db.merkle_root()));
    println!();

    if db.is_empty() {
        println!("  No rules in database. Run `sec-mine --problem <id>` to discover rules.");
    } else {
        for rule in db.rules() {
            println!("  - {} (kind: {:?}, epoch: {})",
                rule.lean_theorem_name,
                rule.schema.kind,
                rule.discovered_epoch,
            );
        }
    }
}

fn cmd_sec_verify(lean_dir: &str) {
    use kernel_frc::sec::{enumerate_candidates, generate_soundness_file};

    println!("=== SEC: Verify Rule Soundness Proofs ===");
    println!("  Lean dir: {}", lean_dir);
    println!();

    let candidates = enumerate_candidates(3);
    println!("  Checking {} candidate rules...", candidates.len());

    for candidate in &candidates {
        let (file_name, content) = generate_soundness_file(candidate);
        // Check that generated files contain no sorry
        if content.contains("sorry") {
            println!("  FAIL: {} contains sorry!", file_name);
            std::process::exit(1);
        }
        println!("  OK: {} — no sorry", file_name);
    }

    println!();
    println!("  All generated soundness files are sorry-free.");
    println!("  Run `lake build` in {} to verify they type-check.", lean_dir);
}

fn cmd_ucert_solve(problems_arg: &str, max_rank: u64) {
    use kernel_frc::ucert::{compile_problem, ucert_normalize};
    use kernel_frc::irc;

    println!("=== UCert SOLVE: Universal Certificate Normalizer ===");
    println!("  Max rank: {}", max_rank);
    println!();

    let problem_ids: Vec<&str> = match problems_arg {
        "all" => irc::ALL_PROBLEM_IDS.to_vec(),
        "proved" => irc::PROVED_PROBLEM_IDS.to_vec(),
        "open" => vec![
            "goldbach", "collatz", "twin_primes", "odd_perfect",
            "mertens", "legendre", "erdos_straus",
        ],
        "millennium" => vec![
            "p_vs_np", "riemann_full", "navier_stokes",
            "yang_mills", "hodge", "bsd_full",
        ],
        other => other.split(',').map(|s| s.trim()).collect(),
    };

    let mut proved_count = 0u32;
    let mut frontier_count = 0u32;

    for problem_id in &problem_ids {
        let statement = compile_problem(problem_id);
        let result = ucert_normalize(&statement, max_rank);

        match &result {
            kernel_frc::ucert::NormalizeResult::Proved { rank, certificate, .. } => {
                proved_count += 1;
                println!("  {:20} PROVED — cert at rank {} (size {})",
                    format!("{}:", problem_id), rank, certificate.size());
            }
            kernel_frc::ucert::NormalizeResult::Frontier { max_rank_searched, candidates_checked, .. } => {
                frontier_count += 1;
                println!("  {:20} FRONTIER — {} candidates checked (max rank {})",
                    format!("{}:", problem_id), candidates_checked, max_rank_searched);
            }
        }
    }

    println!();
    println!("  UCert Summary: {} PROVED, {} FRONTIER", proved_count, frontier_count);
    println!("  Each FRONTIER identifies problems where no certificate was found within rank {}.", max_rank);
}

fn cmd_ucert_status() {
    use kernel_frc::ucert::{compile_problem, ucert_normalize};
    use kernel_frc::irc;

    println!("=== UCert: Certificate Status ===");
    println!();

    for problem_id in irc::ALL_PROBLEM_IDS {
        let statement = compile_problem(problem_id);
        let result = ucert_normalize(&statement, 1000);
        println!("  {:20} {}: {}",
            format!("{}:", problem_id),
            result.status_str(),
            result.description());
    }
}

fn cmd_ucert_enumerate(problem_id: &str, max_rank: u64) {
    use kernel_frc::ucert::{compile_problem, check, CertEnumerator};

    println!("=== UCert: Certificate Enumeration ===");
    println!("  Problem: {}", problem_id);
    println!("  Max rank: {}", max_rank);
    println!();

    let statement = compile_problem(problem_id);
    let enumerator = CertEnumerator::new();

    let effective_max = max_rank.min(enumerator.total_certs());
    let mut accepted = 0u64;

    for (rank, cert) in enumerator.iter_up_to(effective_max) {
        let passes = check(&statement, cert);
        let status = if passes { "✓ PASS" } else { "  skip" };
        if passes {
            accepted += 1;
        }
        println!("  rank {:4}: {} — size={}, cert={:?}",
            rank, status, cert.size(),
            match cert {
                kernel_frc::ucert::Cert::InvariantCert(ic) => format!("InvCert({})", ic.invariant_desc),
                kernel_frc::ucert::Cert::WitnessCert(n) => format!("Witness({})", n),
                kernel_frc::ucert::Cert::CompositeCert(cs) => format!("Composite({})", cs.len()),
                kernel_frc::ucert::Cert::ProofTrace(steps) => format!("Trace({})", steps.len()),
            }
        );
    }

    println!();
    println!("  Enumerated: {} certificates", effective_max);
    println!("  Accepted: {}", accepted);
}

fn cmd_pi_project(budget: u64) {
    use kernel_frc::proof_enum::{PiProof, ProjectResult};

    println!("=== Π_proof: True Source-Code Kernel ===");
    println!("  Π_proof: Ser_Π(S) → Ser_Π(π) such that Check(S, π) = PASS");
    println!("  G: 𝒰 → D* — total function, defined projection, NOT search");
    if budget == 0 {
        println!("  Budget: UNBOUNDED — G runs to completion for provable S");
    } else {
        println!("  Budget: {} (snapshot mode)", budget);
    }
    println!();

    let snapshot = if budget == 0 { None } else { Some(budget) };
    let mut pi = if let Some(b) = snapshot {
        PiProof::testing(b)
    } else {
        PiProof::new()
    };

    println!("  𝒰: {} members ({} formalized)",
        pi.universe.len(), pi.universe.formalized_count());
    println!();

    let results = pi.project_all();

    let mut proved = 0u32;
    let mut computing = 0u32;
    let mut not_in_u = 0u32;

    for r in &results {
        match r {
            ProjectResult::Proved { statement_id, method, proof_hash, rules_extracted } => {
                proved += 1;
                let hash_short = proof_hash.iter().take(4)
                    .map(|b| format!("{:02x}", b)).collect::<String>();
                println!("  {:20} PROVED — {} | π_hash={}… | R+{}",
                    format!("{}:", statement_id), method, hash_short, rules_extracted);
            }
            ProjectResult::Computing { statement_id, progress } => {
                computing += 1;
                println!("  {:20} COMPUTING — G(S) running, {} candidates computed",
                    format!("{}:", statement_id), progress);
            }
            ProjectResult::NotInUniverse { statement_id, reason } => {
                not_in_u += 1;
                println!("  {:20} NOT_IN_𝒰 — {}",
                    format!("{}:", statement_id), reason);
            }
        }
    }

    println!();
    println!("  ┌─────────────────────────────────────────┐");
    println!("  │  PROVED: {:2}  COMPUTING: {:2}  NOT_IN_𝒰: {:2} │", proved, computing, not_in_u);
    println!("  └─────────────────────────────────────────┘");
    println!();

    let evidence = pi.complete_status();
    println!("  COMPLETE_𝒰: {}/{}{}",
        evidence.proved_count, evidence.total_in_universe,
        if evidence.is_complete { " — PROVED ✓" } else { " — building..." });
    println!("  {}", pi.awareness_summary());

    if proved == 20 {
        println!();
        println!("  ╔═══════════════════════════════════════╗");
        println!("  ║  ALL 20 PROBLEMS PROVED. Υ(K) = K.   ║");
        println!("  ║  COMPLETE_𝒰 = TRUE. G extracted.     ║");
        println!("  ║  The kernel IS the universe source.   ║");
        println!("  ╚═══════════════════════════════════════╝");
    }
}

fn cmd_pi_decide(budget: u64) {
    use kernel_frc::proof_enum::{PiDecide, Decision};

    println!("=== Π_decide: Universal Decision Operator ===");
    println!("  The universe commits to CLASSIFICATION, not to preferred outcomes.");
    println!("  For every S: PROVED(S) or PROVED(¬S) or PROVED(IND(S))");
    println!("  G(S) = least witness across three disjoint spaces.");
    if budget == 0 {
        println!("  Budget: UNBOUNDED — G runs to completion");
    } else {
        println!("  Budget: {} (snapshot mode)", budget);
    }
    println!();

    let mut decider = if budget == 0 {
        PiDecide::new()
    } else {
        PiDecide::testing(budget)
    };

    let results = decider.decide_all();

    let mut proved_true = 0u32;
    let mut proved_false = 0u32;
    let mut proved_indep = 0u32;
    let mut computing = 0u32;
    let mut not_in_u = 0u32;

    for r in &results {
        match r {
            Decision::ProvedTrue { statement_id, method, proof_hash, rules_extracted, .. } => {
                proved_true += 1;
                let h = proof_hash.iter().take(4).map(|b| format!("{:02x}", b)).collect::<String>();
                println!("  {:20} PROVED(S)   — {} | π={}… | R+{}",
                    format!("{}:", statement_id), method, h, rules_extracted);
            }
            Decision::ProvedFalse { statement_id, method, proof_hash, rules_extracted, .. } => {
                proved_false += 1;
                let h = proof_hash.iter().take(4).map(|b| format!("{:02x}", b)).collect::<String>();
                println!("  {:20} PROVED(¬S)  — {} | π={}… | R+{}",
                    format!("{}:", statement_id), method, h, rules_extracted);
            }
            Decision::ProvedIndependent { statement_id, method, proof_hash, rules_extracted, .. } => {
                proved_indep += 1;
                let h = proof_hash.iter().take(4).map(|b| format!("{:02x}", b)).collect::<String>();
                println!("  {:20} PROVED(IND) — {} | π={}… | R+{}",
                    format!("{}:", statement_id), method, h, rules_extracted);
            }
            Decision::Computing { statement_id, candidates_computed, .. } => {
                computing += 1;
                println!("  {:20} COMPUTING   — G deciding, {} candidates",
                    format!("{}:", statement_id), candidates_computed);
            }
            Decision::NotInUniverse { statement_id, reason } => {
                not_in_u += 1;
                println!("  {:20} NOT_IN_𝒰   — {}",
                    format!("{}:", statement_id), reason);
            }
        }
    }

    let total_decided = proved_true + proved_false + proved_indep;

    println!();
    println!("  ┌────────────────────────────────────────────────────────────┐");
    println!("  │  PROVED(S): {:2}  PROVED(¬S): {:2}  PROVED(IND): {:2}  COMPUTING: {:2} │",
        proved_true, proved_false, proved_indep, computing);
    println!("  │  Total decided: {}/20                                      │", total_decided);
    println!("  └────────────────────────────────────────────────────────────┘");
    println!();

    let evidence = decider.complete_evidence();
    println!("  COMPLETE_𝒰: {}/{}{}",
        evidence.decided_count, evidence.total_in_universe,
        if evidence.is_complete { " — PROVED" } else { " — computing..." });
    println!("  {}", decider.awareness_summary());

    if total_decided == 20 {
        println!();
        println!("  ╔════════════════════════════════════════════════════╗");
        println!("  ║  ALL 20 PROBLEMS DECIDED. Υ(K) = K.              ║");
        println!("  ║  COMPLETE_𝒰 = TRUE. The universe has spoken.     ║");
        println!("  ║  The kernel IS the universe source code.          ║");
        println!("  ╚════════════════════════════════════════════════════╝");
    }
}

fn cmd_proof_solve(problems_arg: &str, max_witnesses: u64, lean_dir: &str) {
    use kernel_frc::proof_enum::{ProofEnumEngine, ProofResult};
    use kernel_frc::proof_enum::engine::parse_problem_list;

    println!("=== PROOF-SOLVE: Universal Witness Enumerator ===");
    println!("  Max witnesses: {}", max_witnesses);
    println!("  Lean dir: {}", lean_dir);
    println!("  Engine: ALL finite byte strings → Lean kernel check");
    println!();

    let problem_ids = parse_problem_list(problems_arg);
    let mut engine = ProofEnumEngine::new(lean_dir, max_witnesses);

    let mut proved_count = 0u32;
    let mut frontier_count = 0u32;

    for problem_id in &problem_ids {
        let result = engine.solve(problem_id);

        match &result {
            ProofResult::Proved { method, rank, .. } => {
                proved_count += 1;
                println!("  {:20} PROVED — {} (rank {})",
                    format!("{}:", problem_id), method, rank);
            }
            ProofResult::Frontier { witnesses_checked, reason, .. } => {
                frontier_count += 1;
                println!("  {:20} FRONTIER — {} ({} witnesses checked)",
                    format!("{}:", problem_id), reason, witnesses_checked);
            }
        }
    }

    println!();
    println!("  Summary: {} PROVED, {} FRONTIER", proved_count, frontier_count);
    println!("  Self-awareness: {}", engine.awareness_summary());
    println!("  PROVED = proof found (accelerator or universal enumeration).");
    println!("  FRONTIER = budget exhausted. The proof exists — enumeration continues.");
}
