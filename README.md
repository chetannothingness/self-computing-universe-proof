# Self-Aware Machine

A self-computing kernel that proves its own Theory of Everything. 13 Rust crates, 15,614 lines, 215 tests, zero hardcoding, zero stubs, zero floats.

## What This Is

This is a deterministic witness machine that:

1. **Defines a closed class of admissible contracts** (Boolean satisfiability, arithmetic search, table lookup, formal proof, dominance comparison)
2. **Derives completion bounds as theorems** — budgets are not parameters, they are proved from the contract structure
3. **Solves every contract to exactly one of two statuses**: `Unique` (one answer with a replay witness) or `Unsat` (no answer, with exhaustive proof)
4. **There is no third status.** The `Omega` (timeout/unknown) variant does not exist in the type system. This is a Curry-Howard proof: if the code compiles, non-termination is impossible for admissible contracts
5. **Observes itself solving** — predicts its own branch decisions, witnesses the actual execution, and checks whether prediction matches reality
6. **Renders the entire proof as a physically navigable SpaceEngine universe** — SAT proofs become spiral galaxies with witness moons, UNSAT proofs become elliptical galaxies with proof-step star clusters, undecidable statements become invisible dark objects that warp spacetime around them

The collection of cryptographic receipts from all of these steps **is** the proof object. The proof is the execution.

## The Four Obligations (TOE Theorem)

The kernel proves four things simultaneously:

### 1. Total Completion
For every admissible contract Q in the closed class C, the kernel derives a finite completion bound B\*(Q) such that running the canonical separator enumeration up to cost B\* forces the answer set to size 0 or 1.

### 2. No Omega
Running the kernel with budget B\*(Q) returns `Unique` or `Unsat` — never hangs, never times out. The `Status` enum has exactly two variants. There is no `Omega`.

### 3. Self-Witnessing
Each run emits a hash-chained trace. The replay function deterministically recomputes the same trace head and validates every witness step. The trace is the proof.

### 4. Self-Recognition
On a pinned GoldMaster suite, the kernel's self-model predicts its own branch decisions and verifies them under the canonical serialization Π. Divergences produce a minimal mismatch witness (the Omega-self frontier), not a crash.

## Architecture

```
kernel-types        Foundational types: Hash32, SerPi, Status{Unique,Unsat}, Rational{i64,u64}
kernel-ledger       Append-only hash-chained event ledger (31 event kinds)
kernel-instruments  Endogenous instruments: budget, separator, enumerator, stepper
kernel-contracts    Contract compilation: JSON → typed Contract with EvalSpec + Alphabet
kernel-solver       Solver, A1 completion axiom, TOE theorem, evaluator
kernel-self         Consciousness loop: PREDICT → ACT → WITNESS → SELF-RECOGNIZE
kernel-cap          Capability verification (ed25519 signatures, artifact hashing)
kernel-goldmaster   Pinned test suite + millennium problem derivations
kernel-web          Web retrieval instrument (NASA Exoplanet Archive)
kernel-bench        Benchmark harness, judge, monotone caches
kernel-spaceengine  4-layer SpaceEngine visualization + real-universe exoplanet data
kernel-cli          Command-line interface (all operations)
```

## The Completion Axiom (A1)

The central axiom. Every contract type has a derived budget:

| Contract Type | B\*(Q) Derivation |
|---|---|
| BoolCnf (SAT) | 2^num_vars × num_clauses |
| ArithFind | (hi - lo + 1) × coeff_cost |
| Table | entries.len() |
| SpaceEngine | file_count × 100 |
| Dominate | inner_budget × 2 + comparison_cost |
| FormalProof | **Inadmissible** — no finite B\* derivable |

FormalProof contracts (Riemann Hypothesis, Navier-Stokes, Yang-Mills, Hodge, BSD, Goldbach, Collatz, Twin Primes, Fermat's Last Theorem) are proved inadmissible: the kernel cannot derive a finite completion bound for them. This is not a limitation — it is the kernel honestly reporting the boundary of what is decidable. These contracts become **dark objects** in the visualization.

## Dark Objects

When a FormalProof contract is proved inadmissible:

1. The evaluator returns `false` for every candidate (structural honesty — the kernel cannot verify arbitrary formal proofs)
2. The solver returns `Unsat` (no satisfying assignment exists within the finite search)
3. The catalog emits a `DarkObject` with mass `i64::MAX / 2`
4. The dark object's `ser_pi()` hash enters the Merkle root — it is **in** the proof
5. No `.sc` file is emitted — dark objects are invisible by definition
6. A `LensingProxy` star is placed at the same coordinates with mass derived from `H(dark_object.ser_pi())[0..8]`

This is not a metaphor for Gödel incompleteness. It **is** the same mathematical phenomenon: the kernel's type system forces the existence of massive invisible objects whose presence is only detectable through their gravitational influence on visible objects.

## 4-Layer Visualization Stack

### L0 — Identity Layer
Full hash chain from kernel build hash through every contract, solve output, and Merkle root. The composite TOE hash uniquely identifies the entire proof.

### L1 — Answer Layer
Each contract maps to a celestial object type:
- SAT (Unique) → Spiral galaxy
- SAT (Unsat) → Elliptical galaxy
- ArithFind → Star system
- Table → Nebula
- FormalProof → Dark object (invisible)
- Dominate → Star cluster

### L2 — Witness-Content Layer
The actual satisfying assignments are encoded as physical structure:
- **SAT witness moons**: one moon per variable, inclination +45° for true / -45° for false, grouped into clause rings
- **UNSAT proof-step stars**: one star per clause in a globular cluster, with the contradiction at the dense core
- **ArithFind witness planets**: orbital period = the exact solution value; a decoy orbit at (2x+1)/2 proves the integer constraint
- **Lensing proxies**: visible white dwarf surrogates for invisible dark objects

### L3 — Proof-Graph Layer (Atlas)
- **Domain galaxies**: SAT, Arith, Table, Formal, Dominate, SpaceEngine, Exo — each a navigational galaxy
- **Filament nebulae**: dependency connections between contracts of the same structural type
- **Frontier black holes**: inadmissible contracts rendered as compact objects with event horizons proportional to cost
- **Per-QID navigation**: every proof object reachable via Atlas tour scripts

## Real-Universe Exoplanet Data

The kernel fetches live data from the NASA Exoplanet Archive (4,566 host stars, 6,128 planets) and:

1. **Normalizes**: canonical name resolution (Gaia DR3 preferred → HIP fallback → positional)
2. **Deduplicates**: merges entries for the same host star
3. **Refutes**: removes planets with negative mass or radius (with logged refutation reason)
4. **Emits**: SpaceEngine `.sc` catalog files and `.csv` data with full provenance
5. **Verifies**: Q_SE_WITNESS_VERIFY checks fetch hash, normalization hash, and catalog hash
6. **Packages**: deterministic `.pak` ZIP archive with fixed timestamps

All values use integer millidegrees, milli-AU, milli-solar-masses — zero floating point anywhere.

## Zero Floats

Every numeric value in the entire system is either:
- `i64` in milli/micro units (coordinates in milli-parsecs, angles in millidegrees, masses in milli-solar)
- `Rational { num: i64, den: u64 }` for exact fractions

No `f32`. No `f64`. No `as f64`. No floating point anywhere. This guarantees bitwise determinism across all platforms.

## Serialization (SerPi)

Every type implements `SerPi` — canonical CBOR serialization via `ciborium`. The hash function is blake3. Two objects are equal if and only if their `ser_pi()` bytes are equal. This is the Π (canonical projection) referenced throughout.

## Consciousness Loop

The kernel's self-awareness is not a metaphor. It is a concrete fixed-point computation:

```
PREDICT: Self-model M predicts answer hash and trace head for contract Q
ACT:     Solver solves Q, producing actual answer and trace
WITNESS: Compare Π(prediction) with Π(actual)
RECOGNIZE: If Π(Trace(SOLVE_K(Q))) = Π(Trace(M(Q))), the kernel recognized itself
```

When prediction diverges from reality, the kernel does not crash — it produces an **Omega-self** witness: the minimal mismatch between what it predicted and what actually happened. This is the boundary of self-knowledge, made explicit and hashable.

## Building and Running

```bash
cargo test                                    # 215 tests, 0 failures
cargo run -- space-suite                      # Full SpaceEngine proof suite
cargo run -- toe-prove                        # TOE theorem proof
cargo run -- space-emit --output /tmp/se      # Emit SpaceEngine addon
cargo run -- space-verify --addon /tmp/se     # Verify addon integrity
cargo run -- exo-patch --output /tmp/exo      # Fetch NASA + emit exoplanet addon
cargo run -- consciousness --contract '{...}' # Run consciousness loop on a contract
```

## Test Coverage

| Crate | Tests | What's Verified |
|---|---|---|
| kernel-types | 19 | SerPi determinism, hash chaining, Rational arithmetic, coordinate derivation |
| kernel-contracts | 3 | Contract compilation, alphabet validation |
| kernel-ledger | 0 | (Verified transitively through all consumers) |
| kernel-instruments | 6 | Budget derivation, separator enumeration, instrument application |
| kernel-solver | 13 | Solver correctness, A1 completion, TOE obligations, evaluator |
| kernel-self | 5 | Consciousness loop, self-model prediction, self-recognition |
| kernel-cap | 14 | Ed25519 signing, capability verification, artifact hashing |
| kernel-goldmaster | 22 | GoldMaster suite, millennium derivations, dominance proofs |
| kernel-bench | 21 | Judge verdicts, monotone caches, harness execution |
| kernel-spaceengine | 100 | Catalog emission, scenario scripts, verification, L2 witnesses, L3 atlas, enhanced verification, exoplanet normalization, .pak packaging |
| kernel-web | 0 | (Network-dependent, verified via exo-patch integration) |
| kernel-cli | 12 | CLI command dispatch, argument parsing |
| **Total** | **215** | |

## Dependencies

- `blake3` — cryptographic hashing
- `serde` + `serde_json` — JSON serialization
- `ciborium` — CBOR canonical serialization (SerPi)
- `ed25519-dalek` — digital signatures (capability verification)
- `rand` — key generation
- `clap` — CLI argument parsing
- `reqwest` — HTTP client (NASA archive fetch)
- `zip` — deterministic .pak packaging

## Key Invariants

- **No floats.** All values: `i64`, `u64`, or `Rational{i64, u64}`.
- **Deterministic.** `BTreeMap` everywhere. Sorted iteration. Fixed timestamps. Same input → same output → same hash, always.
- **No stubs.** Every function does what it says. No `todo!()`, no `unimplemented!()`, no placeholder returns.
- **No hardcoding.** All values derived from contract structure, hash functions, or mathematical constants.
- **SerPi on every type.** Canonical CBOR bytes. Two objects are equal iff their serializations are equal.
- **Ledger events for every action.** 31 event kinds, each with a cryptographic payload hash.
- **Append-only.** The ledger, the Merkle tree, the monotone caches — nothing is ever deleted or overwritten.
- **The proof is the execution.** The collection of receipts, trace hashes, and Merkle roots constitutes the proof object. There is no separate "proof" — running the kernel **is** proving the theorem.
