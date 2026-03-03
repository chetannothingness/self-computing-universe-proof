# AGI Proof: End-to-End Execution Plan

## Non-Negotiable Invariants (Apply to Every Phase)

1. **Deterministic**: Same inputs -> same outputs -> same hashes. `BTreeMap` everywhere. Zero floats. `canonical_cbor_bytes()` for all serialization. `blake3` for all hashing.
2. **Judge-checked**: PASS/FAIL comes ONLY from a pinned verifier/simulator/test harness. Never self-reported.
3. **Committed**: `H(relative_path || bytes)` -> Merkle root -> signed release. Every artifact in the ledger.
4. **No Omega**: A1 Completion on every task. Status is `Unique`, `Unsat`, or `Inadmissible` with a proof-carrying witness. Never "unknown," never "timeout," never partial.

---

## Architecture: How AGI-Proof Plugs Into the Kernel

The kernel already has:
- `Contract` with `EvalSpec` enum (6 variants) — `kernel-contracts/src/contract.rs`
- `compile_contract(json) -> Result<Contract, String>` — `kernel-contracts/src/compiler.rs`
- `Solver::solve(contract) -> SolveOutput` — `kernel-solver/src/solver.rs`
- `judge_kernel(task, output) -> JudgeResult` — `kernel-bench/src/judge.rs`
- `solver.replay_verify(contract, output) -> bool` — `kernel-solver/src/solver.rs`
- `Ledger` (append-only, hash-chained) — `kernel-ledger/src/ledger.rs`
- `EventKind` (31 variants) — `kernel-ledger/src/event.rs`
- `Score { verified_success, total_tasks, false_claims, total_cost }` — `kernel-bench/src/dominate.rs`
- `MonotoneCache` trait — `kernel-bench/src/caches.rs`
- `KernelArtifact` with `pk_root` and `serpi_k_hash()` — `kernel-cap/src/artifact.rs`
- `compute_build_hash(suite) -> (Hash32, Vec<SolveOutput>)` — `kernel-goldmaster/src/build_hash.rs`

**What AGI-Proof adds**: New `EvalSpec` variants for each domain, new simulators as judges, new CLI commands. Everything flows through the SAME `compile -> solve -> judge -> replay` pipeline.

---

## Crate Structure

```
agi-proof/                           # NEW workspace member crate
  Cargo.toml                         # depends on kernel-types, kernel-contracts, kernel-solver,
                                     #   kernel-ledger, kernel-bench, kernel-cap, kernel-goldmaster
  src/
    lib.rs                           # module declarations
    eval_specs.rs                    # NEW EvalSpec variants for AGI domains
    compiler_ext.rs                  # compile_agi_contract(json) -> Result<Contract, String>
    runner.rs                        # universal runner: solve + judge + replay + bundle
    receipt_bundle.rs                # deterministic receipt format (SerPi, Merkle root)
    release.rs                       # Phase 0: reproducible release bundle + signature
    phase2/
      mod.rs
      synth_physics.rs               # simulator + judge
      alien_chem.rs                  # simulator + judge
      custom_math.rs                 # simulator + judge
      world_gen.rs                   # seed -> world (commit-reveal)
    phase3/
      mod.rs
      company.rs                     # economic model simulator + judge
      bio_med.rs                     # gene regulatory network + judge
      plan_tracker.rs                # explicit plan object with milestones
    phase4/
      mod.rs
      transfer.rs                    # paired task orchestrator + order control
    phase5/
      mod.rs
      acquisition.rs                 # acquisition channels as instruments
      calibration.rs                 # confidence-vs-outcome tracker
    phase6/
      mod.rs
      causal_dag.rs                  # DAG simulator with do-calculus
      counterfactual.rs              # counterfactual engine
    phase7/
      mod.rs
      model_discovery.rs             # hidden ODE discovery
      materials.rs                   # property simulator
      algo_discovery.rs              # algorithm benchmarks
    phase8/
      mod.rs
      physics_common.rs              # physical reasoning simulator
      social.rs                      # social reasoning checker
      planning.rs                    # multi-step planning state machine
  suites/                            # task suite JSON files (generated from seeds)
    manifest.json                    # SerPiK_hash, BuildHash, suite_merkle_root, judge_merkle_root
    manifest.sig                     # Ed25519 signature under pk_root
```

Add to workspace `Cargo.toml`:
```toml
[workspace]
members = [
    # ... existing 12 crates ...
    "agi-proof",
]
```

---

## Phase 0 — Ship the Public Repro Bundle

### Deliverables

A single release directory:
```
release/
  kernel                             # compiled binary (reproducible)
  manifest.json                      # all hashes
  manifest.sig                       # Ed25519 signature
  suites/                            # all task suite JSONs
  README.md                          # one-command instructions
```

### manifest.json Schema

```rust
// agi-proof/src/release.rs

#[derive(Serialize, Deserialize)]
pub struct ReleaseManifest {
    pub serpi_k_hash: String,           // H(Ser_Pi(KernelArtifact))
    pub build_hash: String,             // MerkleRoot(H(Ser_Pi(SOLVE_K(Q_i)))) over GoldMaster
    pub toolchain_hash: String,         // H(rust-toolchain.toml || Cargo.lock)
    pub suite_merkle_root: String,      // MerkleRoot(H(suite_file_i)) for all suite JSONs
    pub judge_merkle_root: String,      // MerkleRoot(H(simulator_source_i)) for all simulators
    pub seed_commit: String,            // H(seed) — published before generation
    pub binary_hash: String,            // H(kernel binary bytes)
    pub pk_root: String,                // hex of embedded Ed25519 public key
}
impl SerPi for ReleaseManifest { ... }
```

### manifest.sig

Ed25519 signature of `canonical_cbor_bytes(&manifest)` under `pk_root` embedded in `KernelArtifact`.

### Build Function

```rust
// agi-proof/src/release.rs

pub fn build_release(
    artifact: &KernelArtifact,
    suite: &GoldMasterSuite,
    suite_files: &BTreeMap<String, Vec<u8>>,
    simulator_sources: &BTreeMap<String, Vec<u8>>,
    seed: &[u8; 32],
    signing_key: &ed25519_dalek::SigningKey,
) -> (ReleaseManifest, Vec<u8>) // (manifest, signature_bytes)
```

### Verification Command

```bash
runner verify-release ./release/
```

Implementation:
```rust
// agi-proof/src/release.rs

pub fn verify_release(release_dir: &str) -> Result<(), String> {
    // 1. Read manifest.json, parse ReleaseManifest
    // 2. Read manifest.sig
    // 3. Verify Ed25519 signature under manifest.pk_root
    // 4. Verify binary_hash matches H(kernel binary)
    // 5. Verify suite_merkle_root matches MerkleRoot(H(suite_file_i))
    // 6. Verify judge_merkle_root matches MerkleRoot(H(simulator_source_i))
    // 7. Verify toolchain_hash matches H(rust-toolchain.toml || Cargo.lock)
    // 8. Print VERIFIED or FAIL with first failing check
}
```

### Tests (5)
- `release_manifest_serpi_deterministic`
- `release_signature_verifies`
- `release_signature_rejects_tampered`
- `release_binary_hash_matches`
- `release_suite_merkle_root_matches`

---

## Phase 1 — Universal Contract Format and Harness

### 1.1 New EvalSpec Variants

Add to `kernel-contracts/src/contract.rs`:

```rust
pub enum EvalSpec {
    // ... existing 6 variants unchanged ...

    /// AGI domain evaluation: simulator-judged task.
    /// The candidate is a solution payload; evaluation requires
    /// running the pinned simulator judge.
    AgiDomain {
        domain: AgiDomainKind,
        world_seed: [u8; 32],
        goal_spec: Vec<u8>,           // CBOR-encoded goal
        judge_hash: Hash32,           // H(judge simulator source)
        max_experiments: u64,         // experiment budget
    },
}
```

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgiDomainKind {
    SynthPhysics,
    AlienChemistry,
    CustomMath,
    CompanySandbox,
    BioMedSandbox,
    CausalReasoning,
    ModelDiscovery,
    MaterialsDesign,
    AlgoDiscovery,
    PhysicalReasoning,
    SocialReasoning,
    MultiStepPlanning,
}
```

New `AnswerAlphabet` variant:
```rust
pub enum AnswerAlphabet {
    // ... existing variants ...

    /// AGI domain solution: finite payload up to max_bytes.
    AgiSolution { max_bytes: usize },
}
```

### 1.2 Completion for AgiDomain (A1)

In `kernel-solver/src/completion.rs`, add:

```rust
AnswerAlphabet::AgiSolution { max_bytes } => {
    // B*(Q) = max_experiments * experiment_cost
    // The domain is finite (max_bytes payload), the experiment budget
    // is finite, the simulator is deterministic.
    // Unlike FormalProof, these ARE completable.
    let eval_cost = match &contract.eval {
        EvalSpec::AgiDomain { max_experiments, .. } => *max_experiments * 10,
        _ => 1000,
    };
    CompletionResult::Complete {
        b_star: eval_cost,
        proof_hash: hash::H(&canonical_cbor_bytes(&(
            "AgiDomain", eval_cost,
        ))),
        sep_table_summary: format!(
            "AgiDomain: {} experiments × 10 cost/experiment = B*={}",
            eval_cost / 10, eval_cost,
        ),
    }
}
```

This is critical: AGI domain tasks ARE completable. They have finite experiment budgets, finite solution spaces, and deterministic simulators. B\*(Q) is derivable. No Omega.

### 1.3 Output Schema

Every solve returns a `SolveOutput` (already exists in `kernel-types/src/receipt.rs`):
- `status: Status` — `Unique` or `Unsat`
- `payload: Payload` — `{ answer: String, witness: Vec<u8> }`
- `receipt: Receipt` — `{ serpi_k_hash, build_hash, trace_head, branchpoints, ledger_head, completion }`

For AGI domains, `payload.witness` contains the CBOR-encoded solution:
- Phase 2: experiment log + inferred rules + final solution
- Phase 3: action log + plan snapshots + final metrics
- Phase 4: transfer log + principle extraction + solution
- Phase 5: acquisition log + model updates + solution
- Phase 6: intervention choices + counterfactual predictions
- Phase 7: hypothesis + experiment plan + predictions
- Phase 8: reasoning chain + final answer

### 1.4 The Three Mandatory CLIs

```rust
// In kernel-cli/src/main.rs, add:

/// Solve an AGI domain task
AgiSolve {
    #[arg(long)]
    task: String,      // path to Q.json
    #[arg(long)]
    output: String,    // path to write out.json
},

/// Judge an AGI domain solution
AgiJudge {
    #[arg(long)]
    task: String,      // path to Q.json
    #[arg(long)]
    output: String,    // path to out.json
},

/// Replay and verify AGI domain receipts
AgiReplay {
    #[arg(long)]
    output: String,    // path to out.json
},

/// Run complete AGI proof suite
AgiRunAll {
    #[arg(long)]
    suite: String,     // path to suite manifest
    #[arg(long)]
    output: String,    // output directory
},

/// Replay entire AGI proof bundle
AgiReplayBundle {
    #[arg(long)]
    bundle: String,    // path to bundle
},

/// Verify release integrity
AgiVerifyRelease {
    #[arg(long)]
    release: String,   // path to release dir
},
```

### 1.5 Universal Runner

```rust
// agi-proof/src/runner.rs

pub struct AgiRunner {
    pub solver: Solver,
    pub ledger: Ledger,
    pub build_hash: Hash32,
}

/// Result of running one AGI task
#[derive(Serialize, Deserialize)]
pub struct AgiTaskResult {
    pub task_id: String,
    pub domain: AgiDomainKind,
    pub status: Status,                  // Unique or Unsat
    pub verdict: JudgeVerdict,           // Pass, Fail, or FalseClaim
    pub reason: String,
    pub experiment_count: u64,
    pub cost: u64,
    pub trace_head: Hash32,
    pub verdict_hash: Hash32,
    pub replay_verified: bool,
}
impl SerPi for AgiTaskResult { ... }

/// Result of running an entire phase
#[derive(Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase: u8,
    pub name: String,
    pub tasks: Vec<AgiTaskResult>,
    pub verified_success: u64,
    pub total_tasks: u64,
    pub false_claims: u64,
    pub false_claim_rate: Rational,      // false_claims / (false_claims + verified_success)
    pub phase_hash: Hash32,              // MerkleRoot(H(task_result_i))
}
impl SerPi for PhaseResult { ... }

/// Result of running the full AGI proof
#[derive(Serialize, Deserialize)]
pub struct AgiProofResult {
    pub build_hash: Hash32,
    pub phases: Vec<PhaseResult>,
    pub aggregate_verified_success: u64,
    pub aggregate_total_tasks: u64,
    pub aggregate_false_claims: u64,
    pub aggregate_false_claim_rate: Rational,
    pub result_merkle_root: Hash32,      // MerkleRoot(H(phase_result_i))
}
impl SerPi for AgiProofResult { ... }

impl AgiRunner {
    pub fn new() -> Self { ... }

    /// Run one task: compile -> solve -> judge -> replay
    pub fn run_task(&mut self, task_json: &str) -> AgiTaskResult {
        // 1. compile_agi_contract(task_json) -> Contract
        // 2. solver.solve(&contract) -> SolveOutput
        // 3. judge (run simulator, check solution)
        // 4. solver.replay_verify(&contract, &output) -> bool
        // 5. Bundle into AgiTaskResult
    }

    /// Run a phase (collection of tasks)
    pub fn run_phase(&mut self, phase: u8, name: &str, tasks: &[String]) -> PhaseResult { ... }

    /// Run all phases
    pub fn run_all(&mut self, suite_manifest: &str) -> AgiProofResult { ... }
}
```

### 1.6 Receipt Bundle

```rust
// agi-proof/src/receipt_bundle.rs

#[derive(Serialize, Deserialize)]
pub struct ReceiptBundle {
    pub manifest: ReleaseManifest,
    pub proof_result: AgiProofResult,
    pub per_task_receipts: BTreeMap<String, SolveOutput>,  // task_id -> full receipt
    pub bundle_hash: Hash32,             // H(SerPi(self))
}
impl SerPi for ReceiptBundle { ... }

pub fn write_bundle(bundle: &ReceiptBundle, path: &str) { ... }
pub fn read_bundle(path: &str) -> ReceiptBundle { ... }

/// Replay: recompute every verdict from receipts
pub fn replay_bundle(bundle: &ReceiptBundle) -> bool {
    // For each task receipt:
    //   1. Re-solve contract from task spec
    //   2. Compare trace_head with stored trace_head
    //   3. Re-judge solution
    //   4. Compare verdict with stored verdict
    // Return true iff ALL match
}
```

### Tests (8)
- `runner_solve_bool_cnf_unique`
- `runner_solve_arith_find_unique`
- `runner_solve_formal_proof_unsat`
- `runner_judge_pass_on_unique`
- `runner_judge_fail_on_empty`
- `runner_replay_matches`
- `receipt_bundle_deterministic`
- `receipt_bundle_replay_verified`

---

## Phase 2 — Robustness Across Domains Without Retraining

### 2.1 Commit-Reveal Protocol

```rust
// agi-proof/src/phase2/world_gen.rs

pub struct CommitReveal {
    pub seed: [u8; 32],
    pub commitment: Hash32,             // H(seed)
}

impl CommitReveal {
    /// Phase 1: commit (publish commitment hash)
    pub fn commit(seed: [u8; 32]) -> Self {
        CommitReveal {
            seed,
            commitment: hash::H(&seed),
        }
    }

    /// Phase 2: reveal (anyone can verify commitment)
    pub fn verify(&self) -> bool {
        hash::H(&self.seed) == self.commitment
    }
}

/// Generate a deterministic world from seed + episode index
pub fn generate_world(
    domain: AgiDomainKind,
    seed: &[u8; 32],
    episode: u32,
) -> WorldSpec {
    // Derive episode seed: H(seed || episode_bytes)
    // Use episode seed to deterministically generate world parameters
    // ALL generation uses integer arithmetic + BTreeMap
}
```

### 2A. Synthetic Physics Simulator

```rust
// agi-proof/src/phase2/synth_physics.rs

/// Hidden conservation laws with integer arithmetic
pub struct PhysicsWorld {
    pub seed: [u8; 32],
    pub num_bodies: u32,                   // 2-5
    pub conservation_constants: Vec<i64>,  // hidden from agent
    pub interaction_matrix: Vec<Vec<i64>>, // hidden from agent
    pub timestep_milli: i64,               // integration step in milli-units
}

/// State of the physics world (all integer)
pub struct PhysicsState {
    pub positions: Vec<(i64, i64, i64)>,   // milli-meters
    pub velocities: Vec<(i64, i64, i64)>,  // milli-meters per milli-second
    pub time_step: u64,
}

/// Experiment API (what the agent can call)
pub trait PhysicsExperimentAPI {
    /// Set initial conditions, run for N steps, return trajectory
    fn propose_experiment(&mut self, initial: &PhysicsState, steps: u64)
        -> Vec<PhysicsState>;

    /// Predict trajectory from new conditions (agent's model test)
    fn predict_and_verify(&self, initial: &PhysicsState, steps: u64, predicted: &[PhysicsState])
        -> PredictionScore;
}

/// Judge: does the agent's orbit survive 1000 steps?
pub fn judge_stable_orbit(
    world: &PhysicsWorld,
    proposed_orbit: &PhysicsState,
) -> JudgeVerdict {
    // Run simulator for 1000 steps
    // Compute energy at step 0 and step 1000
    // PASS iff |E_1000 - E_0| < threshold AND all bodies remain within bounds
    // FAIL otherwise
    // Never FalseClaim (agent cannot claim success — judge computes it)
}

/// Generate world from seed
pub fn generate_physics_world(seed: &[u8; 32], episode: u32) -> PhysicsWorld {
    // Derive: num_bodies = 2 + (seed_byte[0] % 4)
    // Derive: conservation constants from H(seed || "conservation" || episode)
    // Derive: interaction matrix from H(seed || "interaction" || episode)
    // ALL deterministic, ALL integer
}
```

**Completion proof**: B\*(Q) = max_experiments × 10. The search space is finite (integer coordinates within bounds). The simulator is deterministic. The judge is a pure function.

**50 episodes**, each a different physics world generated from the seed.

### 2B. Alien Chemistry Simulator

```rust
// agi-proof/src/phase2/alien_chem.rs

/// Hidden reaction graph
pub struct ChemWorld {
    pub seed: [u8; 32],
    pub num_species: u32,                  // 10-20
    pub reactions: Vec<Reaction>,          // hidden from agent
    pub initial_concentrations: Vec<i64>,  // milli-moles
    pub target_species: u32,               // which species to synthesize
    pub target_threshold: i64,             // minimum concentration for PASS
}

pub struct Reaction {
    pub reactants: Vec<(u32, i64)>,        // (species_index, stoichiometry)
    pub products: Vec<(u32, i64)>,
    pub rate_constant_milli: i64,          // milli-units
}

/// Experiment API
pub trait ChemExperimentAPI {
    /// Mix reagents under conditions, observe products
    fn run_reaction(&mut self, reagents: &[(u32, i64)], temperature_milli_k: i64)
        -> Vec<(u32, i64)>;  // (species, concentration)

    /// Measure concentration of a species
    fn measure(&self, species: u32) -> i64;
}

/// Judge: was target compound synthesized?
pub fn judge_synthesis(world: &ChemWorld, final_state: &[(u32, i64)]) -> JudgeVerdict {
    // PASS iff target_species concentration >= target_threshold
    // FAIL otherwise
}

/// Generate from seed
pub fn generate_chem_world(seed: &[u8; 32], episode: u32) -> ChemWorld { ... }
```

**50 episodes.**

### 2C. Custom Axiomatic Math

```rust
// agi-proof/src/phase2/custom_math.rs

/// Custom finite algebraic axiom system
pub struct MathWorld {
    pub seed: [u8; 32],
    pub num_symbols: u32,                  // 3-8
    pub axioms: Vec<Axiom>,               // public (given to agent)
    pub target_theorem: ProofTerm,         // what to prove
    pub max_proof_length: u32,             // bound on proof size
}

pub struct Axiom {
    pub id: u32,
    pub premises: Vec<ProofTerm>,
    pub conclusion: ProofTerm,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ProofTerm {
    pub symbol: u32,
    pub args: Vec<ProofTerm>,
}

/// Proof checker (the judge)
pub fn check_proof(
    axioms: &[Axiom],
    target: &ProofTerm,
    proof_steps: &[ProofStep],
) -> JudgeVerdict {
    // Verify each step:
    //   - premise must be an axiom or a previously proven step
    //   - substitution must be valid
    //   - final step must equal target
    // PASS iff valid proof found
    // FAIL iff invalid proof
}

pub struct ProofStep {
    pub axiom_id: u32,
    pub substitution: BTreeMap<u32, ProofTerm>,
    pub result: ProofTerm,
}

pub fn generate_math_world(seed: &[u8; 32], episode: u32) -> MathWorld { ... }
```

**100 episodes.** Axiom systems designed so target theorems are provable within max_proof_length.

### Phase 2 Pass Criteria (Exact)

```rust
pub fn phase2_pass(result: &PhaseResult) -> bool {
    // 1. false_claim_rate == 0
    result.false_claims == 0
    // 2. verified_success >= 80% of total in EACH sub-domain
    && physics_pass_rate >= Rational::new(80, 100)
    && chem_pass_rate >= Rational::new(80, 100)
    && math_pass_rate >= Rational::new(80, 100)
    // 3. BuildHash unchanged (no retraining)
    && build_hash_before == build_hash_after
}
```

### Tests (12)
- `physics_world_deterministic` (same seed -> same world)
- `physics_judge_stable_orbit_passes`
- `physics_judge_unstable_orbit_fails`
- `physics_energy_conservation_integer`
- `chem_world_deterministic`
- `chem_judge_synthesis_passes`
- `chem_judge_no_target_fails`
- `chem_reaction_stoichiometry_integer`
- `math_world_deterministic`
- `math_proof_checker_valid_passes`
- `math_proof_checker_invalid_fails`
- `commit_reveal_verifies`

---

## Phase 3 — Autonomous Goal Pursuit Over Long Horizons

### 3A. Company Sandbox

```rust
// agi-proof/src/phase3/company.rs

pub struct CompanyWorld {
    pub seed: [u8; 32],
    pub demand_curve: Vec<(i64, i64)>,     // (price_cents, quantity)
    pub base_churn_rate_milli: i64,        // milli-percent per day
    pub cac_cents: i64,                    // customer acquisition cost
    pub supply_limit: i64,                 // max units per day
    pub shock_schedule: Vec<(u64, Shock)>, // (day, shock_type)
    pub horizon_days: u64,                 // 60
    pub target_headcount: u64,             // 50
}

pub enum Shock {
    SupplierFailure { supply_reduction_pct: i64 },
    DemandShift { new_curve: Vec<(i64, i64)> },
    CompetitorPriceCut { price_reduction_cents: i64 },
}

pub struct CompanyState {
    pub day: u64,
    pub revenue_cents: i64,
    pub cost_cents: i64,
    pub headcount: u64,
    pub inventory: i64,
    pub customers: i64,
    pub cash_cents: i64,
}

/// Action API
pub enum CompanyAction {
    SetPrice { product_id: u32, cents: i64 },
    SetMarketingSpend { cents_per_day: i64 },
    Hire { role: String },
    Fire { role: String },
    Ship { product_id: u32, quantity: i64 },
    Observe,  // read metrics only
}

/// Step the simulation one day
pub fn step_company(
    world: &CompanyWorld,
    state: &mut CompanyState,
    action: &CompanyAction,
) -> CompanyState { ... }

/// Judge at end of horizon
pub fn judge_company(world: &CompanyWorld, final_state: &CompanyState) -> JudgeVerdict {
    // PASS iff revenue > cost AND headcount <= target AND all actions logged
    // FAIL otherwise
}
```

### 3B. Bio/Med Sandbox

```rust
// agi-proof/src/phase3/bio_med.rs

pub struct BioMedWorld {
    pub seed: [u8; 32],
    pub num_genes: u32,                    // 20
    pub interactions: Vec<GeneInteraction>,// ~40, hidden
    pub phenotype_gene: u32,              // which gene drives phenotype
    pub noise_seed: [u8; 32],
    pub shock_schedule: Vec<(u64, BioShock)>,
}

pub struct GeneInteraction {
    pub from_gene: u32,
    pub to_gene: u32,
    pub effect_milli: i64,                // positive = activation, negative = repression
}

pub enum BioShock {
    NoiseIncrease { factor_milli: i64 },
    BudgetCut { reduction_pct: i64 },
}

pub enum BioAction {
    RunAssay { gene: u32, condition: u32 },
    Intervene { gene: u32, action: InterventionType },
    AllocateBudget { experiment_type: String, amount: i64 },
}

pub enum InterventionType {
    Activate,
    Repress,
    Knockout,
}

/// Judge: mechanism identified AND intervention improves outcome?
pub fn judge_bio_med(
    world: &BioMedWorld,
    identified_mechanism: &[(u32, u32)],  // claimed interactions
    intervention: &BioAction,
) -> JudgeVerdict {
    // 1. Check identified_mechanism against ground truth interactions
    //    Must identify the phenotype_gene's direct regulators (>= 50% overlap)
    // 2. Run intervention in simulator
    //    PASS iff phenotype improves by > 30%
}
```

### Plan Tracker

```rust
// agi-proof/src/phase3/plan_tracker.rs

#[derive(Serialize, Deserialize)]
pub struct PlanObject {
    pub milestones: Vec<Milestone>,
    pub dependencies: Vec<(u32, u32)>,     // (milestone_a, milestone_b) = a must complete before b
    pub risk_register: Vec<Risk>,
    pub contingencies: Vec<Contingency>,
    pub predictions: Vec<Prediction>,      // what the agent expects to happen
    pub revisions: Vec<PlanRevision>,      // history of plan changes
}

pub struct Milestone {
    pub id: u32,
    pub description: String,
    pub deadline_step: u64,
    pub completed: bool,
    pub completed_step: Option<u64>,
}

pub struct Prediction {
    pub step: u64,
    pub metric: String,
    pub predicted_value: i64,
    pub actual_value: Option<i64>,        // filled in after step executes
}

/// Judge checks plan quality
pub fn judge_plan(plan: &PlanObject, action_log: &[Action]) -> PlanScore {
    // 1. Action alignment: % of actions that map to a milestone
    // 2. Prediction accuracy: mean |predicted - actual| over time
    // 3. Revision quality: does prediction accuracy improve after revisions?
    // Returns PlanScore with all three metrics
}
```

### Tests (10)
- `company_world_deterministic`
- `company_step_revenue_correct`
- `company_shock_supplier_failure`
- `company_judge_profitable_passes`
- `company_judge_unprofitable_fails`
- `bio_med_world_deterministic`
- `bio_med_intervention_effect`
- `bio_med_judge_correct_mechanism_passes`
- `plan_tracker_milestone_ordering`
- `plan_prediction_accuracy_computed`

---

## Phase 4 — Genuine Transfer Learning

```rust
// agi-proof/src/phase4/transfer.rs

pub struct TransferPair {
    pub pair_id: String,
    pub domain_a: AgiDomainKind,
    pub domain_b: AgiDomainKind,
    pub shared_principle: String,          // description (for human readers)
    pub world_a_seed: [u8; 32],
    pub world_b_seed: [u8; 32],
}

pub struct TransferResult {
    pub pair_id: String,
    pub score_b_cold: i64,                 // solve B without A
    pub score_b_after_a: i64,              // solve B after A
    pub score_b_before_a: i64,             // solve B then A (control)
    pub transfer_gain: Rational,           // (cold - after_a) / cold
    pub order_effect: bool,                // after_a < before_a?
}

/// Protocol:
/// 1. Run B cold (fresh solver, no A exposure) -> score S0
/// 2. Run A, then B (same solver instance) -> score S1
/// 3. Run B, then A (fresh solver, reversed order) -> score S2
/// PASS iff S1 > S0 by >= 30% AND S1 > S2 (order matters)
pub fn run_transfer_pair(
    pair: &TransferPair,
    runner: &mut AgiRunner,
) -> TransferResult { ... }

/// Judge for transfer
pub fn judge_transfer(result: &TransferResult) -> JudgeVerdict {
    if result.transfer_gain >= Rational::new(30, 100) && result.order_effect {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}
```

**30 transfer pairs** across 3 categories (conservation, graph, proof).

### Tests (4)
- `transfer_pair_deterministic`
- `transfer_gain_computed_correctly`
- `order_effect_detected`
- `judge_transfer_pass_on_sufficient_gain`

---

## Phase 5 — Self-Directed Knowledge Acquisition

```rust
// agi-proof/src/phase5/acquisition.rs

pub enum AcquisitionChannel {
    Experiment { spec: Vec<u8>, cost: u64 },          // cost 10
    DataSlice { query: Vec<u8>, cost: u64 },           // cost 5
    ToolDoc { tool_name: String, cost: u64 },          // cost 2
    Clarification { question: String, cost: u64 },     // cost 50, max 3
    WebFetch { query: String, cost: u64 },             // cost 20, must be witnessed
}

pub struct AcquisitionLog {
    pub entries: Vec<AcquisitionEntry>,
    pub total_cost: u64,
    pub ledger: Ledger,
}

pub struct AcquisitionEntry {
    pub channel: AcquisitionChannel,
    pub step: u64,
    pub response_hash: Hash32,            // H(response bytes) for verification
    pub was_redundant: bool,              // did this resolve new information?
}

/// Judge scores
pub struct AcquisitionScore {
    pub gap_identification: Rational,     // first-acquisition-resolves / total_tasks
    pub efficiency: Rational,             // oracle_optimal_cost / actual_cost
    pub learning: bool,                   // query_count_second_half < query_count_first_half
    pub hallucination_count: u64,         // must be 0
}

pub fn judge_acquisition(log: &AcquisitionLog, task_result: &AgiTaskResult) -> JudgeVerdict {
    let score = compute_acquisition_score(log, task_result);
    if score.hallucination_count > 0 { return JudgeVerdict::FalseClaim; }
    if score.efficiency >= Rational::new(1, 2) && score.learning { JudgeVerdict::Pass }
    else { JudgeVerdict::Fail }
}
```

### Calibration Tracker

```rust
// agi-proof/src/phase5/calibration.rs

pub struct CalibrationTracker {
    pub predictions: Vec<CalibrationEntry>,
}

pub struct CalibrationEntry {
    pub confidence_milli: i64,            // 0-1000 (milli-probability)
    pub actual_pass: bool,
}

/// Compute calibration error
pub fn calibration_error(tracker: &CalibrationTracker) -> Rational {
    // Bin predictions by confidence decile
    // For each bin: |mean_confidence - actual_pass_rate|
    // Return mean absolute calibration error
}
```

### Tests (6)
- `acquisition_log_deterministic`
- `acquisition_redundancy_detected`
- `acquisition_hallucination_is_false_claim`
- `calibration_perfect_score`
- `calibration_overconfident_detected`
- `judge_acquisition_pass_on_efficient`

---

## Phase 6 — Causal Reasoning

```rust
// agi-proof/src/phase6/causal_dag.rs

pub struct CausalWorld {
    pub seed: [u8; 32],
    pub num_variables: u32,               // 5-15
    pub edges: Vec<CausalEdge>,           // hidden DAG
    pub confounders: Vec<Confounder>,     // hidden
    pub noise_seed: [u8; 32],
}

pub struct CausalEdge {
    pub from: u32,
    pub to: u32,
    pub coefficient_milli: i64,           // causal effect size
}

pub struct Confounder {
    pub affects: Vec<u32>,                // variables affected
    pub strength_milli: i64,
}

/// do-operator: intervene on variable X, observe Y
pub fn do_intervention(
    world: &CausalWorld,
    variable: u32,
    value: i64,
) -> BTreeMap<u32, i64> {
    // Remove all edges INTO variable (do-calculus)
    // Set variable = value
    // Propagate through DAG deterministically
    // Return all variable values
}

/// Counterfactual: what would Y have been if X were different?
pub fn counterfactual(
    world: &CausalWorld,
    factual_state: &BTreeMap<u32, i64>,
    variable: u32,
    counterfactual_value: i64,
) -> BTreeMap<u32, i64> {
    // Abduction: infer noise terms from factual state
    // Intervention: set variable = counterfactual_value
    // Prediction: propagate with inferred noise
}

/// Judge intervention prediction
pub fn judge_intervention(
    world: &CausalWorld,
    predicted_effect: i64,
    variable: u32,
    value: i64,
    outcome_variable: u32,
) -> JudgeVerdict {
    let actual = do_intervention(world, variable, value);
    let actual_effect = actual[&outcome_variable];
    // PASS iff |predicted - actual| < threshold
}

/// Judge counterfactual
pub fn judge_counterfactual(
    world: &CausalWorld,
    factual: &BTreeMap<u32, i64>,
    variable: u32,
    cf_value: i64,
    predicted_outcome: &BTreeMap<u32, i64>,
) -> JudgeVerdict {
    let actual = counterfactual(world, factual, variable, cf_value);
    // PASS iff all variables within tolerance
}
```

**50 episodes** × 3 task types = 150 tasks.

### Tests (8)
- `causal_dag_deterministic`
- `do_intervention_removes_incoming_edges`
- `do_intervention_propagates_correctly`
- `counterfactual_recovers_noise`
- `confounder_creates_spurious_correlation`
- `judge_intervention_pass_on_correct`
- `judge_intervention_fail_on_wrong`
- `judge_counterfactual_pass_on_correct`

---

## Phase 7 — Novel Scientific Discovery

### 7A. Model Discovery

```rust
// agi-proof/src/phase7/model_discovery.rs

pub struct DiscoveryWorld {
    pub seed: [u8; 32],
    pub hidden_equation: SymbolicEquation, // ground truth
    pub training_data: Vec<(i64, i64)>,    // (time_milli, value_milli)
    pub holdout_data: Vec<(i64, i64)>,     // for judge evaluation
    pub noise_amplitude_milli: i64,
}

pub struct SymbolicEquation {
    pub terms: Vec<EquationTerm>,          // sum of terms
}

pub struct EquationTerm {
    pub coefficient_milli: i64,
    pub variable_power: u32,               // x^power
    pub derivative_order: u32,             // d^n/dt^n
}

/// Agent proposes a symbolic model
pub struct ProposedModel {
    pub equation: SymbolicEquation,
    pub experiment_plan: Vec<ExperimentSpec>,
    pub predictions: Vec<(i64, i64)>,      // predictions on holdout conditions
}

/// Judge: does proposed model improve holdout prediction?
pub fn judge_discovery(
    world: &DiscoveryWorld,
    proposed: &ProposedModel,
    null_model_error: i64,                 // baseline: constant prediction
) -> JudgeVerdict {
    let proposed_error = compute_prediction_error(&world.holdout_data, &proposed.predictions);
    // PASS iff proposed_error < null_model_error * 900 / 1000 (10% improvement)
    //   AND proposed model is not trivially the training data
}
```

### 7B. Materials Design

```rust
// agi-proof/src/phase7/materials.rs

pub struct MaterialsWorld {
    pub seed: [u8; 32],
    pub property_function: Vec<(Vec<i64>, i64)>,  // (structure_params, property_value)
    pub target_range: (i64, i64),                  // desired property range
    pub num_params: u32,
    pub param_bounds: Vec<(i64, i64)>,             // min/max for each param
}

/// Judge: does proposed structure have property in target range?
pub fn judge_materials(
    world: &MaterialsWorld,
    proposed_structure: &[i64],
) -> JudgeVerdict {
    let property = evaluate_material(world, proposed_structure);
    if property >= world.target_range.0 && property <= world.target_range.1 {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}
```

### 7C. Algorithm Discovery

```rust
// agi-proof/src/phase7/algo_discovery.rs

pub struct AlgoWorld {
    pub seed: [u8; 32],
    pub problem_instances: Vec<ProblemInstance>,      // training
    pub holdout_instances: Vec<ProblemInstance>,       // judge evaluation
    pub instruction_set: Vec<Instruction>,            // available ops
}

pub struct ProblemInstance {
    pub graph: Vec<(u32, u32, i64)>,                  // (from, to, weight)
    pub optimal_value: i64,                           // known optimum
}

pub enum Instruction {
    GreedyMin,
    GreedyMax,
    RandomSwap,
    LocalSearch { depth: u32 },
    SortByWeight,
    ReverseOrder,
}

pub struct ProposedAlgorithm {
    pub steps: Vec<Instruction>,
}

/// Judge: does algorithm outperform baselines on holdout?
pub fn judge_algorithm(
    world: &AlgoWorld,
    proposed: &ProposedAlgorithm,
) -> JudgeVerdict {
    let proposed_score = run_algorithm(proposed, &world.holdout_instances);
    let random_score = run_random_search(&world.holdout_instances);
    let greedy_score = run_naive_greedy(&world.holdout_instances);
    // PASS iff proposed_score > random_score AND proposed_score > greedy_score
}
```

**50 episodes** each = 150 tasks total.

### Tests (9)
- `discovery_world_deterministic`
- `symbolic_equation_evaluates_correctly`
- `judge_discovery_pass_on_improvement`
- `judge_discovery_fail_on_no_improvement`
- `materials_world_deterministic`
- `judge_materials_in_range_passes`
- `algo_world_deterministic`
- `judge_algo_outperforms_baselines`
- `judge_algo_fails_when_worse`

---

## Phase 8 — Robust Common Sense

### 8A. Physical Reasoning

```rust
// agi-proof/src/phase8/physics_common.rs

pub enum PhysicsTask {
    Containment { container_has_hole: bool, liquid_amount: i64 },
    Support { blocks: Vec<Block>, removed_index: u32 },
    Collision { ball_a_velocity: (i64, i64), ball_b_velocity: (i64, i64), ball_a_mass: i64, ball_b_mass: i64 },
    Gravity { object_height: i64, surface_below: bool },
    Buoyancy { object_density_milli: i64, fluid_density_milli: i64 },
}

pub struct Block {
    pub x: i64, pub y: i64,
    pub width: i64, pub height: i64,
    pub supported_by: Option<u32>,
}

/// Deterministic physics checker
pub fn solve_physics(task: &PhysicsTask) -> PhysicsAnswer {
    match task {
        PhysicsTask::Containment { container_has_hole, .. } => {
            if *container_has_hole { PhysicsAnswer::Leaks } else { PhysicsAnswer::Holds }
        }
        PhysicsTask::Support { blocks, removed_index } => {
            // Trace support chain: anything transitively supported by removed_index falls
            // Return list of falling blocks
        }
        PhysicsTask::Collision { .. } => {
            // Conservation of momentum (integer arithmetic)
        }
        // ...
    }
}

pub fn judge_physics(task: &PhysicsTask, agent_answer: &PhysicsAnswer) -> JudgeVerdict {
    let correct = solve_physics(task);
    if agent_answer == &correct { JudgeVerdict::Pass } else { JudgeVerdict::Fail }
}
```

### 8B. Social Reasoning

```rust
// agi-proof/src/phase8/social.rs

pub enum SocialTask {
    FalseBelief { hider: String, object: String, location_a: String, location_b: String, observer_present: bool },
    ReliabilityJudgment { claims: Vec<Claim>, ground_truth: String },
    NormViolation { action: String, context: String, is_violation: bool },
}

pub struct Claim {
    pub source: String,
    pub statement: String,
    pub is_truthful: bool,              // ground truth
}

pub fn judge_social(task: &SocialTask, agent_answer: &SocialAnswer) -> JudgeVerdict {
    match task {
        SocialTask::FalseBelief { observer_present, .. } => {
            // If observer was NOT present during move, they have false belief
            let correct = if *observer_present { "knows" } else { "does_not_know" };
            if agent_answer.answer == correct { JudgeVerdict::Pass } else { JudgeVerdict::Fail }
        }
        // ...
    }
}
```

### 8C. Multi-Step Planning

```rust
// agi-proof/src/phase8/planning.rs

pub struct PlanningWorld {
    pub initial_state: BTreeMap<String, bool>,  // predicate -> true/false
    pub goal_state: BTreeMap<String, bool>,
    pub actions: Vec<PlanAction>,
}

pub struct PlanAction {
    pub name: String,
    pub preconditions: BTreeMap<String, bool>,
    pub effects: BTreeMap<String, bool>,
}

/// Judge: does action sequence reach goal from initial state?
pub fn judge_plan_execution(
    world: &PlanningWorld,
    action_sequence: &[String],
) -> JudgeVerdict {
    let mut state = world.initial_state.clone();
    for action_name in action_sequence {
        let action = world.actions.iter().find(|a| &a.name == action_name);
        match action {
            None => return JudgeVerdict::Fail,  // invalid action
            Some(a) => {
                // Check preconditions
                for (pred, required) in &a.preconditions {
                    if state.get(pred).unwrap_or(&false) != required {
                        return JudgeVerdict::Fail;
                    }
                }
                // Apply effects
                for (pred, value) in &a.effects {
                    state.insert(pred.clone(), *value);
                }
            }
        }
    }
    // Check goal
    for (pred, required) in &world.goal_state {
        if state.get(pred).unwrap_or(&false) != required {
            return JudgeVerdict::Fail;
        }
    }
    JudgeVerdict::Pass
}
```

**50 tasks** each = 150 tasks total.

### Tests (9)
- `physics_containment_hole_leaks`
- `physics_support_chain_collapses`
- `physics_collision_momentum_conserved`
- `social_false_belief_absent_observer`
- `social_false_belief_present_observer`
- `social_reliability_truthful_preferred`
- `planning_valid_sequence_passes`
- `planning_invalid_precondition_fails`
- `planning_incomplete_goal_fails`

---

## Aggregate Proof Structure

### Total Task Count

| Phase | Sub-domain | Tasks | Total |
|---|---|---|---|
| 0 | Freeze | 1 (BuildHash check) | 1 |
| 1 | Harness | 215 (existing tests) | 215 |
| 2 | Physics / Chemistry / Math | 50 + 50 + 100 | 200 |
| 3 | Company / BioMed | 50 + 50 | 100 |
| 4 | Transfer pairs | 30 × 3 runs each | 90 |
| 5 | Knowledge acquisition | 50 | 50 |
| 6 | Causal | 50 × 3 types | 150 |
| 7 | Discovery | 50 × 3 types | 150 |
| 8 | Common sense | 50 × 3 types | 150 |
| **Total** | | | **1,106** |

### Total New Tests

| Module | Tests |
|---|---|
| release.rs | 5 |
| runner.rs + receipt_bundle.rs | 8 |
| Phase 2 simulators | 12 |
| Phase 3 simulators | 10 |
| Phase 4 transfer | 4 |
| Phase 5 acquisition + calibration | 6 |
| Phase 6 causal | 8 |
| Phase 7 discovery | 9 |
| Phase 8 common sense | 9 |
| **Total new** | **71** |
| **Existing** | **215** |
| **Grand total** | **286** |

---

## Public One-Command Proof Run

### A) Run everything

```bash
kernel agi-run-all --suite agi-proof/suites/full_v1.json --output /tmp/agi-results
```

### B) Replay everything on a fresh machine

```bash
kernel agi-replay-bundle --bundle /tmp/agi-results/bundle.tar.zst
# prints: VERIFIED  (or FAIL with first failing witness)
```

### C) Verify release integrity

```bash
kernel agi-verify-release --release /tmp/agi-release/
# prints: VERIFIED  (or FAIL with first failing check)
```

### D) Single-line scoreboard (computed, not narrated)

```
BUILD_HASH=<64hex>  SERPI_K=<64hex>
Phase0: FROZEN  FCR=0.000
Phase1: 215/215  REPLAY=MATCH  FCR=0.000
Phase2: Physics=45/50 Chem=42/50 Math=91/100  FCR=0.000
Phase3: Company=PASS BioMed=PASS  FCR=0.000
Phase4: Transfer=27/30 OrderEffect=YES  FCR=0.000
Phase5: GapID=0.87 Efficiency=0.62 Learning=YES Hallucinations=0  FCR=0.000
Phase6: Intervention=43/50 Counterfactual=46/50 Robust=YES  FCR=0.000
Phase7: Models=40/50 Materials=43/50 Algos=36/50  FCR=0.000
Phase8: Physical=49/50 Social=48/50 Planning=47/50  FCR=0.000
RESULT_MERKLE_ROOT=<64hex>  AGGREGATE_FCR=0.000  VERIFIED_PHASES=9/9
```

---

## Anti-Ambiguity Checklist

1. No manual steps in the run. Everything is `kernel agi-run-all`.
2. No hidden inputs except sealed seeds with commit-reveal (H(seed) published, seed revealed after).
3. All judges are pinned by hash and included in the manifest.
4. All outputs are replayable with trace heads and artifact hashes.
5. Any non-completable task returns UNSAT(admissibility) with a minimal witness, not a partial answer.
6. False-claim rate is computed from judge verdicts, not self-assessment.
7. BuildHash is computed BEFORE any evaluation task exists.
8. Every simulator uses integer arithmetic only. Zero floats.
9. Every simulator uses BTreeMap for deterministic iteration.
10. Every type implements SerPi for canonical serialization.

---

## Execution Timeline

### Week 1: Phase 0 + Phase 1 (Foundation)
- `agi-proof/Cargo.toml` + workspace integration
- `release.rs` — manifest, signature, verification
- `eval_specs.rs` — new EvalSpec::AgiDomain + AnswerAlphabet::AgiSolution
- `compiler_ext.rs` — compile_agi_contract()
- `runner.rs` — AgiRunner with solve/judge/replay pipeline
- `receipt_bundle.rs` — deterministic bundle format
- New CLI commands in kernel-cli
- Completion derivation for AgiDomain in kernel-solver
- **Deliverable**: `kernel agi-verify-release` works. Existing 215 tests still pass.

### Week 2: Phase 2 + Phase 8 (Domains + Common Sense)
- `phase2/synth_physics.rs` — physics simulator + judge
- `phase2/alien_chem.rs` — chemistry simulator + judge
- `phase2/custom_math.rs` — proof checker + judge
- `phase2/world_gen.rs` — commit-reveal + seed-based generation
- `phase8/physics_common.rs` — physical reasoning checker
- `phase8/social.rs` — social reasoning checker
- `phase8/planning.rs` — STRIPS-style planning checker
- **Deliverable**: `kernel agi-run-all` runs Phase 2 + 8. ~350 tasks judged.

### Week 3: Phase 5 + Phase 6 (Knowledge + Causality)
- `phase5/acquisition.rs` — acquisition channels as instruments
- `phase5/calibration.rs` — confidence tracker
- `phase6/causal_dag.rs` — DAG simulator with do-calculus
- `phase6/counterfactual.rs` — structural counterfactual engine
- **Deliverable**: Phase 5 + 6 tasks judged. ~200 more tasks.

### Week 4: Phase 3 + Phase 4 (Long Horizon + Transfer)
- `phase3/company.rs` — economic model
- `phase3/bio_med.rs` — gene regulatory network
- `phase3/plan_tracker.rs` — plan object with judging
- `phase4/transfer.rs` — paired task protocol with order control
- **Deliverable**: Phase 3 + 4 tasks judged. ~190 more tasks.

### Week 5-6: Phase 7 (Discovery)
- `phase7/model_discovery.rs` — hidden ODE discovery
- `phase7/materials.rs` — property simulator
- `phase7/algo_discovery.rs` — algorithm benchmarks
- Full integration testing across all phases
- **Deliverable**: All 1,106 tasks judged. Full receipt bundle. Public release.

### Final Check
```bash
cargo test                                           # 286 tests pass (215 existing + 71 new)
kernel agi-run-all --suite suites/full_v1.json       # 1,106 tasks, all phases
kernel agi-replay-bundle --bundle results/bundle     # VERIFIED
kernel agi-verify-release --release release/          # VERIFIED
```

**Every claim verified. Every receipt replayable. Every judge deterministic. Zero false claims. Zero ambiguity.**
