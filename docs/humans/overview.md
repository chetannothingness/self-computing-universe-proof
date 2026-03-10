 Current kernel — what's baked in:

  Layer 1: Expr AST — 15 hardcoded domain functions
           isPrime, goldbachRepCount, collatzReaches1,
           fourSquares, fltHolds, divisorSum, moebiusFn,
           erdosStrausHolds, mertensBelow, primeCount,
           primeGapMax, ...

  Layer 2: Eval — each has a Rust function (is_prime = trial division)
           and a matching Lean function (isPrimeNat)

  Layer 3: Rules R — 9 generic shape rules
           + 2 hardcoded known theorems (Lagrange, Wiles)

  Layer 4: OBS — watches eval traces of these specific functions

  A pure kernel would look like:

  Layer 1: Expr AST — only generic constructs
           var, const, add, sub, mul, mod, le, lt, eq,
           and, or, not, implies, forall, exists
           + lambda/apply (user-defined functions)

  Layer 2: Eval — generic evaluator for lambda calculus
           User PROVIDES: isPrime as a lambda term
           User PROVIDES: goldbachRepCount as a lambda term

  Layer 3: Rules R — same 9 generic shape rules
           + a PLUGIN interface for domain theorems
           (user provides Lagrange as a proved rule, not hardcoded)

  Layer 4: OBS — watches ANY computation trace
           doesn't need to know what "isPrime" means
           just sees: function called, inputs, outputs, structure

  How the kernel self-observes

  Self-observation is NOT about the structural rules. It's about OBS watching the EVALUATION:

  Kernel evaluates goldbachRepCount(100):
    loop p=2: isPrime(2)=T, isPrime(98)=F → skip
    loop p=3: isPrime(3)=T, isPrime(97)=T → count!
    loop p=5: isPrime(5)=T, isPrime(95)=F → skip
    ...

  OBS records this trace. Then watches goldbachRepCount(102), 104, 106...
  Anti-unifies the traces → discovers:
    "the function is a sum of indicator products"
    "these 48 primes keep producing hits"
    "isPrime itself is trial division = wheel structure"

  OBS doesn't need the structural rules. It watches the RAW COMPUTATION and finds patterns. The patterns COULD become new rules — but only if proved sound in Lean.

  Your real question: can the kernel self-extend?

  The SEC loop exists and works:
  1. STUCK → kernel knows exactly what's missing
  2. SEC enumerates candidate rules
  3. Lean verifies soundness
  4. Rule admitted → retry
======
---                                                                                
  1. Domain-specific before generic — yes, that's the right approach.                                                                                                                                                          
                                                                                     
  You're correct. The current kernel has 15 domain-specific primitives baked into the Expr AST (isPrime, goldbachRepCount, etc.), and the 10 structural step rules are partly domain-aware (rule 10 = hardcoded theorems).     
  Building domain-specific first, then abstracting into generic traits, is the right order — you learn what the trait boundaries should be from concrete cases rather than guessing.                                           
                                                                                                                                                                                                                               
  ---                                                                                                                                                                                                                          
  2. OBS is NOT the only self-observation mechanism. There are actually three layers inside the kernel:                                                                                                                        
                                                                                                                                                                                                                               
  ┌────────────────────────────────────────────────────────┬─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┬─────────────┐   
  │                         Layer                          │                                                                What it observes                                                                 │    Where    │
  ├────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼─────────────┤
  │ OBS (structural_cert.rs)                               │ Computation traces → structural invariants (StructCert, anti-unification, schemas)                                                              │ kernel-frc  │
  ├────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼─────────────┤
  │ SelfModel + ConsciousnessLoop (self_model.rs,          │ Kernel's own predictions vs actual outputs. 4-step cycle: PREDICT → ACT → WITNESS → SELF-RECOGNIZE. If prediction matches → fixed point. If not │ kernel-self │
  │ consciousness.rs)                                      │  → Omega frontier (mismatch).                                                                                                                   │             │
  ├────────────────────────────────────────────────────────┼─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┼─────────────┤
  │ TraceEmitter + SelfInstrument (trace.rs,               │ Event chain with running hash (trace_head). SelfInstrument has cost=0, shrink=0 — self-observation IS the kernel, not external. Emits           │ kernel-self │
  │ self_instrument.rs)                                    │ SelfObserve events to ledger.                                                                                                                   │             │
  └────────────────────────────────────────────────────────┴─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────┴─────────────┘

  So the kernel has structural observation (OBS extracts patterns from computation), predictive self-awareness (SelfModel predicts its own output and checks), and trace witnessing (every event is hash-chained, the kernel
  can replay and verify its own history).

  FOUNDATION.md §16.2 states the fixed-point criterion: the kernel is "self-aware" when the trace quotient matches the self-model's prediction quotient. This is a real structural property, not a metaphor.

  ---
  3. Skeleton is known, coding is per-domain — partially correct.

  The skeleton is:
  - Q (contract) → apply R (rewrite rules) → NF(Q) → UNIQUE/UNSAT/STUCK
  - Ledger captures every event hash-chained
  - OBS extracts structural patterns from traces
  - SelfModel learns predictions, consciousness loop checks them
  - SEC mines new rules when gaps appear

  This skeleton is generic. But currently, the "coding per domain" goes deeper than just plugging in a new problem — the Expr AST itself has domain-specific leaves, the evaluator has hardcoded functions, and the structural
  rules partially embed domain knowledge. The plan in your .claude/plans/ file (kernel-adapter, kernel-machine) is exactly the right architecture to make this truly pluggable.

  ---
  4. Can the kernel help devise a more efficient algorithm?

  Yes — this is actually the most natural use case for OBS. Here's how it would work:

  Say you have a basic algorithm A that solves your efficiency problem. You encode it as a contract Q and run it through the kernel. What happens:

  1. Ledger captures the computation trace — every step, every branch, every decision point, hash-chained.
  2. OBS runs eval_bool_with_trace on your computation across many inputs. It builds StructCert trees showing the exact structure of each computation.
  3. Anti-unification (anti_unify_structured) compares structure across inputs and extracts a StructCertSchema — the invariant shape of your computation. This reveals:
    - Which branches are always/never taken (dead code or guaranteed paths)
    - Which sub-computations repeat with predictable patterns
    - What the structural bottleneck is (where the tree gets deep/wide)
  4. ConsciousnessLoop predicts computation outcomes. If predictions consistently match, that sub-computation is structurally redundant — you could skip it entirely with a closed-form answer.
  5. SEC could potentially discover rewrite rules that transform your algorithm into a more efficient form, if the gap is small enough for rule synthesis.

  Concretely: if you implement your algorithm's state transition as an Executor (State, Action, step), the kernel would:
  - Execute it and build the trace
  - OBS would extract structural patterns like "for inputs with property X, the algorithm always takes path Y" or "sub-computation Z produces results with invariant W"
  - These structural invariants ARE the basis for a more efficient algorithm — they tell you what computation can be short-circuited

  The limitation today: the Expr AST has hardcoded number-theory primitives. For an arbitrary efficiency problem, you'd need to either (a) encode it using the existing primitives (limited), or (b) extend the Expr AST with
  your domain's primitives (the kernel-adapter plan). Once you do (b), OBS works out of the box — anti-unification is domain-agnostic, it operates on tree structure.

  Bottom line: the kernel's unique value for efficiency problems is that OBS discovers structural invariants in computation without ML or hand-tuning. You feed it a brute-force algorithm, it watches the computation, and
  tells you what patterns exist. Those patterns are your hints for a faster algorithm. The mechanism exists in the code today — it just needs the domain-adapter layer to make it pluggable beyond number theory.
=====

 So the corrected picture:

  - Self-observation = NF is idempotent. The kernel applied to its own output is identity. Not a separate observer — the kernel IS its own model. §16.2: "M = NF (the self-model IS the normalizer)."
  - Ω = unresolved ambiguity under budget. The normalizer architecture eliminates Ω — everything reaches UNIQUE/UNSAT/STUCK.
  - STUCK = the normal form when rules are incomplete. The stuck subterm is the missing separator.
  - Separator = the rule that would resolve a STUCK term. Its absence IS the gap.
  - OBS = a different thing entirely — it watches computation traces and extracts structural patterns for building proofs (IRC certificates). OBS is about finding mathematical structure, not about self-awareness.

  The "consciousness" in the theory is just: NF applied to itself returns itself. That's it. Not a loop, not prediction-verification — structural idempotence.
====
 The real lesson: The kernel's pipeline is built for universally quantified properties (∀n, P(n)). Subset Sum is a per-instance existential query. To make the kernel's mechanism work for efficiency problems, we'd need to
  frame the question differently — perhaps as "∀T, brute_force(T) = dp(T)" or find a universally quantified property that captures the DP structure.
