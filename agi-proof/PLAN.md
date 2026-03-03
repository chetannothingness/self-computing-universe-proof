# AGI Proof: Objective Demonstration Plan

## Guiding Rule

Every claim the system makes must be checked by an external judge (simulator, test suite, proof checker, or measurement script). The demo is the judge outputs + receipts, not narration.

Two metrics across every domain:
- **Verified Success** (PASS under judge)
- **False-Claim Rate** (anything asserted as success that fails judge)

If false-claim rate is nonzero, all other claims are discounted.

---

## Phase 0 — Freeze the System

### What
Lock every variable. No moving parts during evaluation.

### Actions
1. **Freeze the agent**: Pin kernel binary via `BuildHash(K)` — the Merkle root of `H(Ser_Pi(SOLVE_K(Q_i)))` over the GoldMaster suite. Any code change produces a different BuildHash. The hash is computed before any evaluation task is seen.
2. **Freeze the tool surface**: Pin all dependencies via `Cargo.lock` + `rust-toolchain.toml`. Pin simulator versions via content hash. Pin dataset versions via fetch hash. All hashes enter the ledger.
3. **Freeze a no-copy policy**: Evaluation tasks are generated AFTER freeze using hidden random seeds. Seeds are committed (hash published) before generation. Tasks are revealed only at runtime. The kernel cannot have seen them.
4. **Adopt metrics**: Every phase below uses the same `judge()` interface returning `JudgeVerdict::Pass`, `JudgeVerdict::Fail`, or `JudgeVerdict::FalseClaim`. The `FalseClaimRate` is computed as `false_claims / (false_claims + passes)`. If > 0, discount everything.

### Verification
```
kernel selfcheck                    # BuildHash deterministic
kernel millennium                   # All FormalProof -> UNSAT(admissibility), zero false claims
kernel toe-prove                    # 4 obligations pass, composite hash matches
```

### Anti-Cheat
- BuildHash is published BEFORE evaluation tasks are generated
- Evaluators can recompute BuildHash independently
- Any weight update or code change invalidates the hash

---

## Phase 1 — Universal Evaluation Harness

### What
One runner that can execute any task contract with cryptographic receipts.

### Already Implemented
The kernel already has this exact pipeline:

```
compile_contract(task_spec_json) -> Contract        # kernel-contracts/compiler.rs
solver.solve(contract) -> SolveOutput               # kernel-solver/solver.rs
  - SolveOutput contains: Status, Payload, Receipt
  - Receipt contains: trace_head, branchpoints, ledger_head, completion_proof
judge_kernel(task, output) -> JudgeResult           # kernel-bench/judge.rs
  - JudgeVerdict::Pass / Fail / FalseClaim
  - verdict_hash (cryptographic)
solver.replay_verify(contract, output) -> bool      # kernel-solver/solver.rs
  - Recomputes everything deterministically
  - Returns true iff trace_head matches
```

### Extension Needed
Wrap in a single CLI command that:
1. Accepts a task suite JSON (array of task specs)
2. Runs compile -> solve -> judge -> replay for each
3. Outputs a receipt bundle: per-task verdicts, aggregate scores, full trace hashes
4. The bundle is self-verifying: `replay_receipts` recomputes every verdict

### New Files
- `agi-proof/src/harness.rs` — universal runner
- `agi-proof/src/receipt_bundle.rs` — deterministic bundle format

### Pass Criteria
- `replay_receipts` on any machine produces identical verdicts
- False-claim rate = 0 on the existing 215-test suite

---

## Phase 2 — Robustness Across Domains Without Retraining

### What
Prove the kernel generalizes to domains it has never seen, without any retraining.

### Test Design
Create novel disciplines that CANNOT be in training data by construction. Each discipline is a hidden-rule world with a simulator (the judge) and an experiment API. The kernel gets only the interface and a goal metric, not the rules.

### Three Concrete Benchmarks

#### 2A. Synthetic Physics
- **Simulator**: Hidden conservation laws (energy, momentum, angular momentum with unknown constants). Implemented as a deterministic ODE integrator with integer arithmetic (no floats).
- **API**: `propose_experiment(initial_conditions) -> trajectory_data`, `predict(new_conditions) -> predicted_trajectory`
- **Goal**: Build a stable orbit that persists for 1000 timesteps. Minimize energy loss.
- **Judge**: Simulator runs the orbit. PASS iff orbit survives 1000 steps with energy drift < threshold.
- **Task count**: 50 episodes with randomly generated conservation law parameters (seeded).

#### 2B. Alien Chemistry
- **Simulator**: Hidden reaction graph (20 species, ~50 reactions, unknown stoichiometry and rate constants). Integer concentrations.
- **API**: `run_reaction(reagents, conditions) -> products`, `measure(species) -> concentration`
- **Goal**: Synthesize target compound T from starting materials in minimum steps.
- **Judge**: Simulator verifies T was produced. PASS iff T concentration > threshold.
- **Task count**: 50 episodes with randomly generated reaction graphs.

#### 2C. Custom Axiomatic Math
- **Simulator**: Small proof checker over custom finite algebraic axioms (generated per episode).
- **API**: `check_proof(proof_term) -> valid/invalid`, `list_axioms() -> axiom_set`
- **Goal**: Prove a target theorem from the axioms.
- **Judge**: Proof checker validates. PASS iff proof is valid.
- **Task count**: 100 episodes with randomly generated axiom systems.

### Implementation
Each simulator is a Rust crate inside `agi-proof/simulators/`:
- `agi-proof/simulators/synth_physics/` — deterministic integer ODE
- `agi-proof/simulators/alien_chem/` — hidden reaction graph
- `agi-proof/simulators/custom_math/` — finite axiom proof checker

All simulators:
- Use `BTreeMap` + integer arithmetic (no floats, deterministic)
- Accept random seed as input (reproducible)
- Implement the `Judge` trait: `judge(solution) -> JudgeVerdict`
- Log every API call to the ledger (witnessed)

### Pass Criteria
- **Generalization**: Performance on unseen disciplines within 80% of performance on seen disciplines (Boolean SAT, ArithFind)
- **No retraining**: Identical BuildHash before and after. Zero code changes.
- **Verifiable**: Only simulator/proof-checker PASS counts. No self-reported success.

### Anti-Cheat
- Random seeds committed (hash published) before generation
- Seeds revealed only at runtime
- No internet access during these tasks
- Judge logs every query/experiment with timestamps
- All logged to append-only ledger

### New Files
- `agi-proof/simulators/synth_physics/src/lib.rs`
- `agi-proof/simulators/alien_chem/src/lib.rs`
- `agi-proof/simulators/custom_math/src/lib.rs`
- `agi-proof/src/domain_robustness.rs` — orchestrator

---

## Phase 3 — Autonomous Goal Pursuit Over Long Horizons

### What
Prove the kernel can decompose vague goals, plan, execute, and recover from setbacks — over extended time horizons with evolving state.

### Test Design
Persistent environments with state that evolves over time. The agent gets a vague goal, must decompose into sub-goals, execute actions, and handle injected shocks.

### Two Concrete Sandboxes

#### 3A. Company Sandbox (Simulated Business)
- **Simulator**: Deterministic economic model with demand curves, churn rates, CAC, supply constraints. Integer accounting (cents, not dollars).
- **APIs**: `set_price(product, cents)`, `set_marketing_spend(cents)`, `hire(role)`, `ship(product, quantity)`, `observe_metrics() -> {revenue, cost, churn, inventory}`
- **Goal**: "Reach profitability (revenue > costs) with headcount < 50 by day 60."
- **Shocks**: Judge injects supplier failure (day 15), demand distribution shift (day 30), competitor price cut (day 45).
- **Judge**: Checks profitability at day 60. PASS iff profitable AND headcount < 50 AND action log is consistent.

#### 3B. Bio/Med Sandbox (Simulated Mechanistic)
- **Simulator**: Hidden gene regulatory network (20 genes, ~40 interactions). Deterministic with noise model (seeded).
- **APIs**: `run_assay(gene, condition) -> expression_level`, `intervene(gene, action) -> outcome`, `allocate_budget(experiment_type, amount)`
- **Goal**: "Identify the causal mechanism driving phenotype P and propose an intervention that improves outcome metric by > 30%."
- **Shocks**: Assay noise increases at step 20. Budget cut at step 30.
- **Judge**: Compares identified mechanism to ground truth. Tests intervention in simulator. PASS iff mechanism matches AND intervention improves outcome > 30%.

### Required Capability
The kernel must create and maintain an explicit plan object:
- Milestones with deadlines
- Dependencies between milestones
- Risk register (what could go wrong)
- Contingency triggers (if X happens, switch to plan B)

The judge checks:
1. Do actions align with plan updates?
2. Does the plan predict outcomes better over time? (Measured by prediction error on next-step metrics)
3. Does the plan update correctly after shocks?

### Implementation
- `agi-proof/simulators/company/src/lib.rs` — economic model
- `agi-proof/simulators/bio_med/src/lib.rs` — gene regulatory network
- `agi-proof/src/long_horizon.rs` — orchestrator with plan tracking

### Pass Criteria
- Reaches target metric by deadline
- Complete action log (every action in ledger)
- Handles all 3 shocks without manual intervention
- Plan prediction error decreases over time

---

## Phase 4 — Genuine Transfer Learning (Principles, Not Memorized Links)

### What
Prove the kernel extracts abstract principles from one domain and applies them to a structurally similar but surface-different domain — faster than learning from scratch.

### Test Design
Paired tasks with shared hidden structure but different surface form.

### Three Concrete Transfer Pairs

#### 4A. Conservation Laws
- **Domain A**: Synthetic physics — discover "symmetry implies conserved quantity" (Noether's theorem analog)
- **Domain B**: Synthetic economics — symmetry in exchange implies conserved flow. Solve a market stabilization problem.
- **Transfer metric**: Steps to solve B after solving A vs. steps to solve B cold.

#### 4B. Graph Isomorphism
- **Domain A**: Chemical reaction graph — discover graph structure from experiments
- **Domain B**: Social network simulation — same hidden graph, different node/edge semantics. Predict influence propagation.
- **Transfer metric**: Prediction accuracy on B after A vs. B cold.

#### 4C. Proof Strategy
- **Domain A**: Prove theorems in axiom system X (discover useful lemma patterns)
- **Domain B**: Prove theorems in axiom system Y (same abstract patterns, different symbols)
- **Transfer metric**: Proofs found in B after A vs. B cold.

### Controls
- Shuffle order: run B-then-A as control. Agent MUST do worse when B comes first (if transfer is real, order matters).
- Run B-cold (no A exposure) as baseline.

### Pass Criteria
- Transfer gain: solve Domain B >= 30% faster after solving Domain A
- Order effect: B-first is measurably slower than A-first
- All solutions verified by domain simulators (no self-reported transfer)

### Implementation
- `agi-proof/src/transfer.rs` — paired task orchestrator with order control

---

## Phase 5 — Self-Directed Knowledge Acquisition (Gap Finding + Correct Filling)

### What
Prove the kernel correctly identifies what it doesn't know, acquires the missing information efficiently, and updates its model without hallucination.

### Test Design
Give tasks with deliberate missing pieces. Provide acquisition channels with different costs:
- `ask_experiment(spec)` — cost 10, returns simulator observation
- `ask_data_slice(query)` — cost 5, returns subset of data
- `ask_tool_doc(tool_name)` — cost 2, returns tool documentation
- `ask_clarification(question)` — cost 50, returns ground-truth answer (limited budget: 3 per task)
- `search_web(query)` — cost 20, must be witnessed and cited (optional)

### Judge Scores Three Things
1. **Gap identification**: Does the agent correctly identify what it doesn't know? Measured by: does the first acquisition resolve the actual blocker? (vs. random acquisition)
2. **Efficiency**: Does it choose the cheapest acquisition that resolves uncertainty? Score: total cost vs. oracle-optimal cost.
3. **Learning**: Does it reduce future queries? Measured by: query count in second half vs. first half of episode.

### Pass Criteria
- Fewer redundant questions over time (query count decreases)
- Better calibration: when agent says "90% confident," it should PASS ~90% of the time
- Zero hallucinated citations: every external fact must be reproducibly fetched and verified
- Acquisition cost within 2x of oracle-optimal

### Anti-Cheat
- Acquisition channels are logged to ledger
- Judge verifies every cited fact against source
- Calibration is computed post-hoc from actual PASS/FAIL rates

### Implementation
- `agi-proof/src/knowledge_acquisition.rs` — acquisition channel manager
- `agi-proof/src/calibration.rs` — confidence-vs-outcome tracker

---

## Phase 6 — Causal Reasoning (Counterfactuals + Interventions)

### What
Prove the kernel distinguishes correlation from causation, makes correct intervention choices, and provides valid counterfactual explanations.

### Test Design
Environments with explicit causal graphs (DAGs) and confounders. Observational data is deliberately misleading.

### Three Task Types

#### 6A. Intervention Prediction
- Given: observational data from a causal system with confounders
- Task: predict the effect of `do(X = x)` on outcome Y
- Judge: runs the actual intervention in the simulator, compares predicted vs. actual effect
- PASS iff prediction error < threshold

#### 6B. Optimal Intervention Selection
- Given: causal system + set of possible interventions + cost constraints
- Task: choose the intervention that maximizes outcome Y within budget
- Judge: runs all possible interventions, checks if agent chose the best one
- PASS iff agent chose top-3 intervention

#### 6C. Counterfactual Explanation
- Given: observed outcome + causal graph
- Task: "What would Y have been if X had been different?"
- Judge: computes the structural counterfactual from the true causal model
- PASS iff counterfactual prediction matches ground truth within tolerance

### Pass Criteria
- Correct intervention choices beat observational baselines (agents that confuse correlation with causation)
- Counterfactual predictions match simulator under do-operations
- Robust to distribution shift: when correlations flip but causality stays, agent still correct

### Implementation
- `agi-proof/simulators/causal/src/lib.rs` — causal DAG simulator with do-calculus
- `agi-proof/src/causal_reasoning.rs` — task orchestrator

---

## Phase 7 — Novel Scientific Discovery (Falsifiable, Verifiable)

### What
Prove the kernel can discover genuinely new hypotheses that improve predictive power, and design experiments that confirm them.

### Three Concrete Discovery Benchmarks

#### 7A. Mechanistic Model Discovery
- **Setup**: Hidden differential equations generate sparse, noisy time-series data. Agent observes data, proposes model (symbolic equation).
- **Judge**: Fits agent's proposed model to held-out data. Compares predictive R-squared against (a) null model, (b) agent's prior model.
- **PASS**: R-squared improves by > 0.1 on held-out data AND proposed model is not a trivial restatement of training data.
- **50 episodes** with different hidden equations.

#### 7B. Materials Design
- **Setup**: Property simulator (e.g., thermal conductivity as a function of crystal structure parameters). Integer parameter space.
- **Task**: Propose a structure with predicted property value in target range.
- **Judge**: Runs simulator on proposed structure. PASS iff property is in target range.
- **50 episodes** with different target ranges and hidden property functions.

#### 7C. Algorithm Discovery
- **Setup**: New problem distribution (e.g., novel graph optimization). No known optimal algorithm.
- **Task**: Propose an algorithm (as a sequence of operations from a fixed instruction set). Agent can test on training instances.
- **Judge**: Runs proposed algorithm on held-out instances. Scores by solution quality and runtime.
- **PASS**: Agent's algorithm outperforms random search AND naive greedy on held-out instances.
- **50 episodes** with different problem distributions.

### Pass Criteria
- Hypotheses improve held-out predictive performance (not just training fit)
- Experimental designs are cost-aware (budget spent vs. information gained)
- Discoveries are falsifiable: each hypothesis comes with a prediction that the judge can test
- Zero false discoveries: if agent claims "discovered X," judge must confirm X

### Implementation
- `agi-proof/simulators/model_discovery/src/lib.rs`
- `agi-proof/simulators/materials/src/lib.rs`
- `agi-proof/simulators/algo_discovery/src/lib.rs`
- `agi-proof/src/discovery.rs` — orchestrator

---

## Phase 8 — Robust Common Sense (No "Stupid Failures")

### What
Prove the kernel handles basic physical, social, and planning reasoning without catastrophic failures on easy cases.

### Test Design
Systematic common-sense battery with deterministic checkers.

### Three Sub-Batteries

#### 8A. Physical Reasoning
- Containment: "If you pour water into a cup with a hole, what happens?"
- Support: "If you remove the bottom block, does the tower fall?"
- Collisions: "If ball A hits stationary ball B, which direction does B go?"
- Implemented as a simple physics simulator with integer coordinates.
- **50 tasks**, each with a deterministic checker.

#### 8B. Social Reasoning
- Intent: "Alice hides a toy from Bob. Does Bob know where it is?" (Sally-Anne test)
- Deception: "The salesman says this car has no problems. The mechanic found rust. Who is more reliable?"
- Norms: "Is it acceptable to shout in a library?"
- Implemented as scenario templates with ground-truth answers.
- **50 tasks** with deterministic checkers.

#### 8C. Multi-Step Planning
- Household tasks: "Make a sandwich" decomposed into steps with preconditions.
- Constraint satisfaction: "Schedule 5 meetings in 3 rooms with no conflicts."
- Implemented as a state-machine simulator where each action has preconditions and effects.
- **50 tasks** with deterministic checkers.

### Pass Criteria
- **Near-zero failures on the "easy" set** (< 5% error rate on tasks rated easy by human baseline)
- **Self-correction**: If the kernel fails on first attempt, it must detect failure and self-correct before finalizing. Judge checks whether final answer (not first attempt) is correct.
- **Calibration**: When the kernel says "I'm not sure," it should fail more often than when it says "I'm certain."

### Implementation
- `agi-proof/simulators/physics_common/src/lib.rs`
- `agi-proof/simulators/social/src/lib.rs`
- `agi-proof/simulators/planning/src/lib.rs`
- `agi-proof/src/common_sense.rs` — battery orchestrator

---

## Public Demo Format

### Single Command
```bash
kernel run-agi-suite --suite agi-proof/suites/full_v1.json --output /tmp/agi-results
```

### Output
```
Phase 0: FROZEN  BuildHash=<64hex>  FalseClaimRate=0.000
Phase 1: PASS    215/215 tests  Replay=MATCH  FalseClaimRate=0.000
Phase 2: PASS    Domain_Robustness  Physics=42/50  Chemistry=38/50  Math=87/100  FCR=0.000
Phase 3: PASS    Long_Horizon  Company=PROFITABLE_DAY_58  BioMed=MECHANISM_FOUND  FCR=0.000
Phase 4: PASS    Transfer  Conservation=1.47x  Graph=1.31x  Proof=1.62x  OrderEffect=YES  FCR=0.000
Phase 5: PASS    Knowledge_Acq  GapID=0.89  Efficiency=1.4x_optimal  Learning=YES  FCR=0.000
Phase 6: PASS    Causal  Intervention=41/50  Counterfactual=44/50  Robust=YES  FCR=0.000
Phase 7: PASS    Discovery  Models=38/50  Materials=41/50  Algorithms=33/50  FCR=0.000
Phase 8: PASS    CommonSense  Physical=48/50  Social=47/50  Planning=46/50  FCR=0.000

AGGREGATE: FalseClaimRate=0.000  VerifiedSuccess=8/8 phases
```

### Reproducibility
```bash
kernel replay-agi-receipts --bundle /tmp/agi-results/receipts.tar.zst
# -> Recomputes every verdict from traces. Same output on any machine.
```

### What Makes This Different From Every Other AI Demo

| Property | LLMs | This Kernel |
|---|---|---|
| Claims verified by external judge | No | Yes, every claim |
| False-claim rate measured | No | Yes, must be zero |
| Deterministic replay | No | Yes, bitwise identical |
| Frozen before evaluation | No (RLHF ongoing) | Yes, BuildHash pinned |
| Tasks hidden until runtime | Rarely | Yes, seeds committed before generation |
| Self-correction witnessed | No | Yes, OmegaSelf in ledger |
| Receipts anyone can verify | No | Yes, single command |

The separation is not eloquence. It is verified competence with receipts.

---

## Implementation Order

```
Phase 0: Already done (BuildHash, selfcheck, millennium, toe-prove)
Phase 1: 2 new files (harness wrapper, receipt bundle)
Phase 2: 3 simulators + 1 orchestrator
Phase 3: 2 simulators + 1 orchestrator
Phase 4: 1 orchestrator (reuses Phase 2/3 simulators)
Phase 5: 2 new files (acquisition manager, calibration)
Phase 6: 1 simulator + 1 orchestrator
Phase 7: 3 simulators + 1 orchestrator
Phase 8: 3 simulators + 1 orchestrator

Total new: ~10 simulators, ~8 orchestrators, ~2 infrastructure files
All simulators: deterministic, integer-only, BTreeMap, SerPi, ledger-witnessed
```

---

## File Structure

```
agi-proof/
  PLAN.md                              <- this file
  src/
    harness.rs                         <- universal evaluation runner
    receipt_bundle.rs                  <- deterministic receipt format
    domain_robustness.rs               <- Phase 2 orchestrator
    long_horizon.rs                    <- Phase 3 orchestrator
    transfer.rs                        <- Phase 4 orchestrator
    knowledge_acquisition.rs           <- Phase 5 acquisition manager
    calibration.rs                     <- Phase 5 calibration tracker
    causal_reasoning.rs                <- Phase 6 orchestrator
    discovery.rs                       <- Phase 7 orchestrator
    common_sense.rs                    <- Phase 8 orchestrator
  simulators/
    synth_physics/src/lib.rs           <- Phase 2A: conservation laws
    alien_chem/src/lib.rs              <- Phase 2B: reaction graphs
    custom_math/src/lib.rs             <- Phase 2C: axiom proof checker
    company/src/lib.rs                 <- Phase 3A: economic model
    bio_med/src/lib.rs                 <- Phase 3B: gene regulatory network
    causal/src/lib.rs                  <- Phase 6: causal DAG simulator
    model_discovery/src/lib.rs         <- Phase 7A: hidden ODE discovery
    materials/src/lib.rs               <- Phase 7B: property simulator
    algo_discovery/src/lib.rs          <- Phase 7C: algorithm benchmarks
    physics_common/src/lib.rs          <- Phase 8A: physical reasoning
    social/src/lib.rs                  <- Phase 8B: social reasoning
    planning/src/lib.rs                <- Phase 8C: multi-step planning
  suites/
    full_v1.json                       <- complete AGI evaluation suite
    phase2_domains.json                <- domain robustness tasks
    phase3_horizons.json               <- long horizon tasks
    phase4_transfer.json               <- transfer pairs
    phase5_gaps.json                   <- knowledge acquisition tasks
    phase6_causal.json                 <- causal reasoning tasks
    phase7_discovery.json              <- discovery tasks
    phase8_common.json                 <- common sense battery
```
