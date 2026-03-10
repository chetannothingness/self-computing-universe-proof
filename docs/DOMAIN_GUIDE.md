# Domain Integration Guide: Adding a New Problem to the Self-Computing Kernel

This guide documents exactly what is domain-specific and what is generic in the
kernel, and the precise steps to add a new problem domain. Written from direct
code inspection, not speculation.

---

## How the Kernel Works (in one paragraph)

A problem Q is encoded as an `Expr` (expression tree). The kernel normalizes Q
by applying rewrite rules R until it reaches normal form: `UNIQUE(witness)`,
`UNSAT(witness)`, or `STUCK(subterm)`. The proof path is IRC (Invariant
Reduction Certificate): find an invariant I such that Base `I(n₀)` holds,
Step `I(n) -> I(n+delta)` holds, and Link `I(n) -> P(n)` holds. Then
`irc_implies_forall` gives `forall n, P(n)`. Every step is ledger-recorded
and hash-chained. Self-observation = NF(NF(Q)) = NF(Q) (idempotence). OBS
watches computation traces and extracts structural patterns. The stuck subterm
in STUCK is the missing separator (the precise lemma whose proof would
complete normalization).

Reference: `FOUNDATION.md` sections 13, 14, 16.

---

## What Is Generic vs Domain-Specific

### Fully Generic (never touch for a new domain)

| Crate/File | What it does |
|---|---|
| `kernel-types/` | Hash, SerPi, Receipt, Tension, ReasonCode |
| `kernel-solver/` | Refinement operator, stepper, solve loop |
| `kernel-instruments/` | Instrument trait, Budget, DeltaEnumerator |
| `kernel-self/` | SelfModel, ConsciousnessLoop, SelfRecognition, TraceEmitter |
| `kernel-frc/src/invsyn/structural.rs` rules 1-9 | Generic structural step rules (ground, lower bound, modular, conjunction, disjunction, negation, implication) |
| `kernel-frc/src/invsyn/structural.rs` link rules | 5 generic link rules (identity, trivial true/false, conjunction projection, range) |
| `kernel-frc/src/sec/sec_engine.rs` | Self-Extending Calculus (rule mining + Lean verification) |
| `kernel-frc/src/invsyn/structural_cert.rs` OBS core | `eval_bool_with_trace`, `StructCert`, `anti_unify_structured`, `StructCertSchema` |
| `lean/KernelVm/Invariant.lean` | IRC structure, `irc_implies_forall` theorem (proved, 0 sorry) |
| `lean/Universe/CheckSound.lean` | `CertifiedIRC`, `certified_irc_proves` theorem (proved, 0 sorry) |

### Domain-Specific (must extend for a new domain)

There are exactly **6 touchpoints**, organized below by the order you implement them.

---

## Step-by-Step: Adding a New Problem Domain

### Step 1: Define Your Primitives — `kernel-frc/src/invsyn/ast.rs`

The `Expr` enum is the expression language. It has 21 generic operators
(arithmetic, logic, quantifiers) and 13 domain-specific primitives.

**Generic (already there, use freely):**
```
Var(usize), Const(i64),
Add, Sub, Mul, Neg, Mod, Div, Pow, Abs, Sqrt,
Le, Lt, Eq, Ne,
And, Or, Not, Implies,
ForallBounded(lo, hi, body), ExistsBounded(lo, hi, body)
```

**Domain-specific (these are the number theory ones, for reference):**
```
IsPrime, DivisorSum, MoebiusFn,
CollatzReaches1, ErdosStrausHolds, FourSquares, MertensBelow, FltHolds,
PrimeCount, GoldbachRepCount, PrimeGapMax,
IntervalBound, CertifiedSum
```

**What you add:** New `Expr` variants for your domain's decidable predicates.
Each primitive must be:
- **Total**: terminates on all inputs
- **Deterministic**: same input always gives same output
- **Decidable**: returns bool or integer, never "unknown"

Example for a graph theory domain:
```rust
// in Expr enum:
ChromaticNumber(Box<Expr>),        // chi(G_n)
IsHamiltonian(Box<Expr>),          // does G_n have a Hamiltonian cycle?
MaxClique(Box<Expr>),              // omega(G_n)
IndependenceNumber(Box<Expr>),     // alpha(G_n)
```

The generic operators (arithmetic, logic, quantifiers) compose with your
primitives to form complex properties. You only need primitives for things
that CANNOT be expressed using the generic operators.

File: `kernel-frc/src/invsyn/ast.rs` (add variants to `Expr` enum)

---

### Step 2: Implement Evaluators — `kernel-frc/src/invsyn/eval.rs`

Each new `Expr` variant needs a Rust evaluation function. This is the
computation that OBS will trace and the checker will verify.

**Pattern (from existing code):**
```rust
// Standalone function
fn chromatic_number(n: i64) -> i64 {
    // Construct graph G_n from the integer encoding
    // Compute chromatic number
    // Must terminate, must be deterministic
}

// In the eval() match block:
Expr::ChromaticNumber(e) => chromatic_number(eval(env, e)),
```

**Requirements:**
- Function must be total (use fuel/bounds if needed — see `collatz_reaches_1`
  which uses 10,000 iteration fuel at eval.rs:198)
- Function must match the Lean evaluator exactly (Step 5)
- The computation structure IS what OBS will observe — so write it clearly,
  not cleverly. Trial division is better than Miller-Rabin because OBS can
  extract the wheel structure from trial division.

File: `kernel-frc/src/invsyn/eval.rs` (add function + match arm in `eval()`)

---

### Step 3: Register Your Problem — `kernel-frc/src/invsyn/normalize.rs`

Every problem is normalized into reachability form:
```
(state_type, initial_value, step_delta, property_expr, lean representations)
```

This is where Q gets defined.

**Pattern (from existing Goldbach registration):**
```rust
"goldbach" => ReachabilityProblem {
    problem_id: "goldbach".to_string(),
    state_type: "Nat".to_string(),
    initial_value: 4,             // start checking from n=4
    step_delta: 2,                // check every even number
    initial_lean: "fun n => n = 4".to_string(),
    step_lean: "fun n m => m = n + 2".to_string(),
    property_lean: "fun n => exists p q, Nat.Prime p /\\ Nat.Prime q /\\ n = p + q".to_string(),
    property_expr: Some(goldbach_property()),   // Expr tree built below
    description: "Goldbach: every even n >= 4 is sum of two primes".to_string(),
},
```

Then define the property as an `Expr` tree:
```rust
fn goldbach_property() -> Expr {
    // exists p in [2, n], isPrime(p) AND isPrime(n - p)
    Expr::ExistsBounded(
        Box::new(Expr::Const(2)),           // lo = 2
        Box::new(Expr::Var(0)),             // hi = n
        Box::new(Expr::And(
            Box::new(Expr::IsPrime(Box::new(Expr::Var(0)))),      // isPrime(p)
            Box::new(Expr::IsPrime(Box::new(
                Expr::Sub(Box::new(Expr::Var(1)), Box::new(Expr::Var(0)))  // isPrime(n-p)
            ))),
        ))
    )
}
```

**Variable binding convention:**
- `Var(0)` = the bound variable of the innermost quantifier
- `Var(1)` = the next outer variable
- `Var(n)` in the outermost scope = the problem parameter n

**If your property is NOT expressible in InvSyn Expr** (e.g., requires Turing
machines, continuous PDEs, complex analysis), set `property_expr: None`. The
kernel will record this as FRONTIER — an honest gap, not a failure. To make
it expressible: go back to Step 1 and add primitives.

File: `kernel-frc/src/invsyn/normalize.rs` (add match arm + property function)

Also add to: `kernel-frc/src/ucert/compile.rs` (add `Statement::forall_from`
or `Statement::decide_prop` entry)

---

### Step 4: Update Known Proofs (if applicable) — `kernel-frc/src/ucert/check.rs`

If your domain includes theorems that are ALREADY PROVEN (like Lagrange's
four-square theorem or FLT), register them so the structural checker can
use them for the Step obligation.

```rust
// In structural.rs, rule 10 (known theorem registry):
// If your problem has a known proof, add it here.

// In check.rs:
const KNOWN_PROOF_REGISTRY: &[(&str, &[&str])] = &[
    ("bertrand_postulate", &["bertrand"]),
    ("lagrange_four_squares", &["lagrange"]),
    // Add yours:
    ("four_color_theorem", &["four_color"]),
];
```

Also update `problem_delta()` if your step size is not 1:
```rust
fn problem_delta(problem_id: &str) -> i64 {
    match problem_id {
        "goldbach" => 2,      // even numbers only
        "odd_perfect" => 2,   // odd numbers only
        // Add yours:
        "your_problem" => 1,
        _ => 1,
    }
}
```

File: `kernel-frc/src/ucert/check.rs` (update registries)
File: `kernel-frc/src/invsyn/structural.rs` rule 10 (add known theorems)

---

### Step 5: Mirror in Lean — `lean/KernelVm/InvSyn.lean`

The Lean side must mirror the Rust side exactly. This is what makes proofs
real — the Lean type checker verifies what Rust computes.

**5a. Add Expr constructor** (mirrors Step 1):
```lean
-- In the Expr inductive type:
| chromaticNumber (e : Expr)
| isHamiltonian (e : Expr)
```

**5b. Add Lean evaluator** (mirrors Step 2):
```lean
-- Must match the Rust function EXACTLY, same algorithm, same bounds
def chromaticNumberNat (n : Nat) : Nat := ...
```

**5c. Add to eval function:**
```lean
| .chromaticNumber e => chromaticNumberNat (eval env e).toNat
```

**Critical**: The Lean evaluator must use the same algorithm as Rust. If Rust
uses trial division for isPrime, Lean must use trial division too. OBS
observes the structure of the Rust computation, then the structural certificate
is verified against the Lean computation. They must agree.

File: `lean/KernelVm/InvSyn.lean` (Expr type + eval function + native evaluators)

---

### Step 6: Add Problem-Specific Lean Proof Files

Create the IRC proof structure for your problem:

```
lean/OpenProblems/YourDomain/
    Statement.lean    -- The statement as a Lean Prop
    Invariant.lean    -- IRC: base, step, link (or FRONTIER gaps)
```

**Statement.lean pattern** (from Goldbach):
```lean
def yourProblemFull : Prop :=
  forall n, n >= start -> your_condition n -> your_property n
```

**Invariant.lean pattern** (from Goldbach):
```lean
-- Candidate invariant: prefix accumulator
def yourInvariant (n : Nat) : Prop :=
  forall m, start <= m -> m <= n -> condition m -> property m

-- Base: vacuously true (no m in [start, initial_value])
theorem your_base : yourInvariant initial_value := by
  intro m h_start h_bound; omega

-- Step: THIS IS WHERE THE MATH IS
-- If you can prove it: theorem your_step ...
-- If you can't: document as FRONTIER(Step) honestly

-- Link: usually trivial
theorem your_link (n : Nat) (h : yourInvariant n) :
    condition n -> property n := by
  exact h n (le_refl n) ...
```

If the step obligation is open (i.e., IS the conjecture), document it
honestly like Goldbach does:
```lean
-- FRONTIER(Step): The step obligation remains open.
-- The step obligation IS [YourConjecture] itself.
```

File: `lean/OpenProblems/YourDomain/Statement.lean`
File: `lean/OpenProblems/YourDomain/Invariant.lean`

---

## What Happens After You Add a Domain

Once you complete Steps 1-6, the kernel automatically does:

1. **Normalization**: `normalize("your_problem")` produces the ReachabilityProblem.
   The universal checker compiles Q from your property_expr.

2. **IRC attempt**: The checker tries Base (ground eval), Step (10 structural
   rules), Link (5 structural rules). If all pass -> UNIQUE. If Step fails ->
   STUCK with the stuck subterm as the missing separator.

3. **OBS traces**: `eval_bool_with_trace` runs your evaluators and records
   the computation structure as `StructCert` trees. `anti_unify_structured`
   extracts a `StructCertSchema` — the invariant shape across multiple n.
   This is automatic and generic. OBS does not contain domain-specific code.

4. **SEC mining**: If STUCK, the Self-Extending Calculus enumerates candidate
   rules, generates Lean proofs, and verifies them. If a valid rule is found,
   it's added to the RuleDb and normalization retries.

5. **Ledger**: Every event (compilation, instrument application, branch,
   certification, self-observation) is hash-chained. The trace is replayable.

6. **Self-observation**: NF(NF(Q)) = NF(Q). The kernel applied to its own
   output is identity. This is structural, not a separate mechanism.
   The ConsciousnessLoop verifies this operationally.

---

## OBS: What Is Domain-Specific in It?

**OBS core is fully generic.** These functions work on ANY Expr tree:

| Function | File | What it does |
|---|---|---|
| `eval_bool_with_trace` | structural_cert.rs | Traces evaluation of any Expr |
| `StructCert` enum | structural_cert.rs | Tree-shaped certificate (generic shapes) |
| `anti_unify_structured` | structural_cert.rs | Extracts invariant shape across n values |
| `StructCertSchema` | structural_cert.rs | Parameterized template (generic) |

**Domain-specific in the OBS file** (structural_cert.rs):
- Lean proof generation functions (`generate_lean_proof_file`,
  `generate_goldbach_complete_proof`, etc.) — these produce problem-specific
  Lean tactic sequences
- Problem ID routing (`problem_id_to_module`, `run_pipeline` with hardcoded
  problem names)
- Expansion rules for specific primitives (isPrime wheel, goldbachRepCount
  decomposition)

**Summary**: OBS *observes* generically but *generates proofs* domain-specifically.
When you add a new domain, OBS will automatically trace your evaluators and
extract structural patterns. But if you want OBS to generate Lean proof files
for your domain, you need to add proof generation code.

---

## The 10 Structural Step Rules (all generic)

These are in `kernel-frc/src/invsyn/structural.rs` lines 325-524.
They work for ANY domain — they only inspect expression structure:

| # | Rule | What it checks |
|---|------|---------------|
| 1 | Ground | I(n) has no free variables -> trivially preserved |
| 2 | Lower bound | n >= a and delta > 0 -> n+delta >= a |
| 3 | Strict lower bound | n > a -> n+delta > a |
| 4 | Modular congruence | n % m = r preserved when delta % m = 0 |
| 5 | Modular non-congruence | n % m != r preserved when delta % m = 0 |
| 6 | Conjunction | Both conjuncts pass independently |
| 7 | Disjunction | Both disjuncts pass independently |
| 8 | Negation | Handle negated comparisons |
| 9 | Implication | Ground implications only |
| 10 | Known theorems | Registry of proven theorems (Lagrange, FLT) |

Rules 1-9 are purely structural. Rule 10 is a registry — add your proven
theorems there (Step 4 above).

---

## File Checklist for a New Domain

```
MUST MODIFY:
  kernel-frc/src/invsyn/ast.rs          -- Add Expr variants
  kernel-frc/src/invsyn/eval.rs         -- Add Rust evaluators
  kernel-frc/src/invsyn/normalize.rs    -- Register ReachabilityProblem
  lean/KernelVm/InvSyn.lean            -- Mirror Expr + evaluators in Lean

SHOULD MODIFY:
  kernel-frc/src/ucert/compile.rs       -- Add Statement compilation
  kernel-frc/src/ucert/check.rs         -- Update problem_delta, known proofs

CREATE NEW:
  lean/OpenProblems/YourDomain/
      Statement.lean                    -- Lean Prop definition
      Invariant.lean                    -- IRC proof structure

OPTIONAL (for OBS proof generation):
  kernel-frc/src/invsyn/structural_cert.rs  -- Problem-specific Lean generation
  kernel-frc/src/invsyn/structural.rs       -- Add known theorems to rule 10

NEVER TOUCH:
  kernel-types/                         -- Pure generic infrastructure
  kernel-solver/                        -- Generic refinement operator
  kernel-instruments/                   -- Generic instrument abstraction
  kernel-ledger/                        -- Generic hash-chained event log
  kernel-self/                          -- Generic self-model + consciousness
  lean/KernelVm/Invariant.lean         -- Generic IRC framework (proved)
  lean/Universe/CheckSound.lean        -- Generic CertifiedIRC (proved)
```

---

## Design Principles

1. **Primitives must be computationally transparent.** OBS traces the
   evaluation step by step. If your evaluator uses a clever algorithm that
   hides structure (e.g., Miller-Rabin for primality), OBS cannot extract
   useful patterns. Write evaluators that expose structure (e.g., trial
   division). The kernel trades compute speed for observability.

2. **Lean and Rust must mirror exactly.** The Lean evaluator is the
   soundness anchor. The Rust evaluator is the execution engine. If they
   disagree on any input, the proof is unsound. Same algorithm, same bounds,
   same edge cases.

3. **FRONTIER is honest, not failure.** If your Step obligation IS the
   conjecture (like Goldbach's), document it as FRONTIER. The kernel
   distinguishes "proved" from "open" structurally — STUCK subterm is the
   precise missing lemma certificate.

4. **Self-observation is free.** Per FOUNDATION.md section 13.8: "Self-observation
   costs zero because applying NF to itself is identity. There is no gap
   between observer and observed." You do not need to build observation
   machinery — the normalizer IS the self-model.

5. **Extend the basis, not the kernel.** To solve STUCK problems, you add
   new sound rules to R (via SEC or manually). The kernel architecture never
   changes — only the rule database grows. Each new rule must be proved sound
   in Lean (0 sorry) and must preserve termination (measure decreases).
