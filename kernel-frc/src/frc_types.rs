// FRC (Finite Reduction Certificate) types.
//
// For a statement S, an FRC is:
//   FRC(S) = (C, B*, ProofEq, ProofTotal)
// such that:
//   C is a self-delimiting finite computation (VM program)
//   B* ∈ ℕ is a bound derived from the proof, not supplied externally
//   ProofEq proves: S ⟺ (C returns 1 within B*)
//   ProofTotal proves: C is total in the pinned semantics
//
// A statement S is admissible iff an FRC exists: S admissible ⟺ ∃ FRC(S).
// This is A0 applied one meta-level higher.

use serde::{Serialize, Deserialize};
use kernel_types::{Hash32, SerPi, hash};

use crate::vm::Program;

/// Schema identifier — which reduction strategy produced this FRC.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SchemaId {
    BoundedCounterexample,
    FiniteSearch,
    EffectiveCompactness,
    ProofMining,
    AlgebraicDecision,
    CertifiedNumerics,
    /// User-defined / schema-induction-derived schema
    Derived(String),
    /// SEC-derived rule schema
    SEC(String),
}

impl SerPi for SchemaId {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// ProofEq: proves S ⟺ (VM.run(C, B*) = 1).
///
/// In practice this is a structured proof object with:
/// - the statement hash
/// - the program hash
/// - the bound
/// - the reduction chain (sequence of equivalence-preserving transforms)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofEq {
    pub statement_hash: Hash32,
    pub program_hash: Hash32,
    pub b_star: u64,
    pub reduction_chain: Vec<ReductionStep>,
    pub proof_hash: Hash32,
    /// Generated Lean4 proof term (populated when Lean generation is requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean_proof: Option<String>,
}

impl ProofEq {
    pub fn compute_hash(
        statement_hash: &Hash32,
        program_hash: &Hash32,
        b_star: u64,
        chain: &[ReductionStep],
    ) -> Hash32 {
        let mut buf = Vec::new();
        buf.extend_from_slice(statement_hash);
        buf.extend_from_slice(program_hash);
        buf.extend_from_slice(&b_star.ser_pi());
        for step in chain {
            buf.extend_from_slice(&step.ser_pi());
        }
        hash::H(&buf)
    }
}

impl SerPi for ProofEq {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// A single step in the reduction chain S → ... → (run(C,B*)=1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReductionStep {
    pub from_hash: Hash32,
    pub to_hash: Hash32,
    pub justification: String,
    pub step_hash: Hash32,
}

impl SerPi for ReductionStep {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// ProofTotal: proves C is total and respects the bound model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProofTotal {
    pub program_hash: Hash32,
    pub b_star: u64,
    pub halting_argument: String,
    pub proof_hash: Hash32,
    /// Generated Lean4 proof term (populated when Lean generation is requested).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lean_proof: Option<String>,
}

impl ProofTotal {
    pub fn compute_hash(program_hash: &Hash32, b_star: u64, argument: &str) -> Hash32 {
        let mut buf = Vec::new();
        buf.extend_from_slice(program_hash);
        buf.extend_from_slice(&b_star.ser_pi());
        buf.extend_from_slice(argument.as_bytes());
        hash::H(&buf)
    }
}

impl SerPi for ProofTotal {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// The FRC itself — the core certificate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frc {
    /// The VM program (self-delimiting finite computation)
    pub program: Program,
    /// The explicit bound derived from the proof
    pub b_star: u64,
    /// Proof that S ⟺ (run(C, B*) = 1)
    pub proof_eq: ProofEq,
    /// Proof that C halts within B*
    pub proof_total: ProofTotal,
    /// Which schema produced this FRC
    pub schema_id: SchemaId,
    /// Hash of the target statement
    pub statement_hash: Hash32,
    /// Hash of the FRC itself
    pub frc_hash: Hash32,
}

impl Frc {
    pub fn new(
        program: Program,
        b_star: u64,
        proof_eq: ProofEq,
        proof_total: ProofTotal,
        schema_id: SchemaId,
        statement_hash: Hash32,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&program.ser_pi());
        buf.extend_from_slice(&b_star.ser_pi());
        buf.extend_from_slice(&proof_eq.ser_pi());
        buf.extend_from_slice(&proof_total.ser_pi());
        buf.extend_from_slice(&schema_id.ser_pi());
        buf.extend_from_slice(&statement_hash);
        let frc_hash = hash::H(&buf);

        Self {
            program,
            b_star,
            proof_eq,
            proof_total,
            schema_id,
            statement_hash,
            frc_hash,
        }
    }

    /// Verify internal consistency: hashes match, proofs bind correctly.
    pub fn verify_internal(&self) -> bool {
        // ProofEq must reference the correct program and bound
        if self.proof_eq.program_hash != self.program.ser_pi_hash() {
            return false;
        }
        if self.proof_eq.b_star != self.b_star {
            return false;
        }
        if self.proof_eq.statement_hash != self.statement_hash {
            return false;
        }

        // ProofTotal must reference the correct program and bound
        if self.proof_total.program_hash != self.program.ser_pi_hash() {
            return false;
        }
        if self.proof_total.b_star != self.b_star {
            return false;
        }

        // Recompute FRC hash
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.program.ser_pi());
        buf.extend_from_slice(&self.b_star.ser_pi());
        buf.extend_from_slice(&self.proof_eq.ser_pi());
        buf.extend_from_slice(&self.proof_total.ser_pi());
        buf.extend_from_slice(&self.schema_id.ser_pi());
        buf.extend_from_slice(&self.statement_hash);

        hash::H(&buf) == self.frc_hash
    }
}

impl SerPi for Frc {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// FRC search result — what the engine returns.
#[derive(Debug, Clone)]
pub enum FrcResult {
    /// FRC found: statement is admissible and decidable
    Found(Frc),
    /// No FRC exists in current schema library: statement is INVALID
    /// with minimal missing-lemma frontier
    Invalid(FrontierWitness),
}

/// Frontier witness — the first unprovable subgoal that blocks FRC construction.
/// This is NOT Ω — it is a proof that no FRC exists within the current schema closure,
/// with an exact specification of what lemma would unblock it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierWitness {
    pub statement_hash: Hash32,
    pub schemas_tried: Vec<SchemaId>,
    pub gaps: Vec<Gap>,
    pub minimal_missing_lemma: Option<MissingLemma>,
    pub frontier_hash: Hash32,
}

impl FrontierWitness {
    pub fn new(
        statement_hash: Hash32,
        schemas_tried: Vec<SchemaId>,
        gaps: Vec<Gap>,
        minimal_missing_lemma: Option<MissingLemma>,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&statement_hash);
        for s in &schemas_tried {
            buf.extend_from_slice(&s.ser_pi());
        }
        for g in &gaps {
            buf.extend_from_slice(&g.ser_pi());
        }
        if let Some(ref lemma) = minimal_missing_lemma {
            buf.extend_from_slice(&lemma.ser_pi());
        }
        let frontier_hash = hash::H(&buf);

        Self {
            statement_hash,
            schemas_tried,
            gaps,
            minimal_missing_lemma,
            frontier_hash,
        }
    }
}

impl SerPi for FrontierWitness {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// A gap — a subgoal that the schema could not prove.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gap {
    pub goal_hash: Hash32,
    pub goal_statement: String,
    pub schema_id: SchemaId,
    pub dependency_hashes: Vec<Hash32>,
    pub unresolved_bound: Option<String>,
}

impl SerPi for Gap {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// A missing lemma — the exact statement that would unblock the FRC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissingLemma {
    pub lemma_hash: Hash32,
    pub lemma_statement: String,
    pub needed_by_schema: SchemaId,
    pub needed_for_goal: Hash32,
}

impl SerPi for MissingLemma {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Open Problem Package — the contract format for FRC.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenProblemPackage {
    /// The formal statement S (as a string representation)
    pub statement: String,
    /// Context: definitions and imports
    pub context: String,
    /// Which schema family is allowed
    pub target_class: TargetClass,
    /// Allowed instrument set and cost model
    pub allowed_primitives: AllowedPrimitives,
    /// Expected output type
    pub expected_output: ExpectedOutput,
    /// Hash of the package
    pub opp_hash: Hash32,
}

impl OpenProblemPackage {
    pub fn new(
        statement: String,
        context: String,
        target_class: TargetClass,
        allowed_primitives: AllowedPrimitives,
        expected_output: ExpectedOutput,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(statement.as_bytes());
        buf.extend_from_slice(context.as_bytes());
        buf.extend_from_slice(&target_class.ser_pi());
        buf.extend_from_slice(&allowed_primitives.ser_pi());
        buf.extend_from_slice(&expected_output.ser_pi());
        let opp_hash = hash::H(&buf);

        Self {
            statement,
            context,
            target_class,
            allowed_primitives,
            expected_output,
            opp_hash,
        }
    }
}

impl SerPi for OpenProblemPackage {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Target class — which schema families are allowed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetClass {
    pub allowed_schemas: Vec<SchemaId>,
    pub grammar_description: String,
}

impl SerPi for TargetClass {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Allowed primitives — instrument set and cost model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllowedPrimitives {
    pub max_vm_steps: u64,
    pub max_memory_slots: usize,
    pub cost_model: String,
}

impl SerPi for AllowedPrimitives {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Expected output type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExpectedOutput {
    Proof,
    Disproof,
    Either,
}

impl SerPi for ExpectedOutput {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// FRC execution receipt — the complete verification artifact.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrcReceipt {
    pub frc_hash: Hash32,
    pub execution_outcome: u8,
    pub trace_head: Hash32,
    pub merkle_root: Hash32,
    pub statement_hash: Hash32,
    pub verified: bool,
    pub receipt_hash: Hash32,
}

impl FrcReceipt {
    pub fn new(
        frc_hash: Hash32,
        execution_outcome: u8,
        trace_head: Hash32,
        merkle_root: Hash32,
        statement_hash: Hash32,
        verified: bool,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&frc_hash);
        buf.push(execution_outcome);
        buf.extend_from_slice(&trace_head);
        buf.extend_from_slice(&merkle_root);
        buf.extend_from_slice(&statement_hash);
        buf.push(if verified { 1 } else { 0 });
        let receipt_hash = hash::H(&buf);

        Self {
            frc_hash,
            execution_outcome,
            trace_head,
            merkle_root,
            statement_hash,
            verified,
            receipt_hash,
        }
    }
}

impl SerPi for FrcReceipt {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Kernel manifest for FRC — pins the proof universe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelManifest {
    pub kernel_build_hash: Hash32,
    pub serpi_k_hash: Hash32,
    pub vm_hash: Hash32,
    pub schema_library_hash: Hash32,
    pub motif_library_hash: Hash32,
    pub class_c_hash: Hash32,
    pub manifest_hash: Hash32,
}

impl KernelManifest {
    pub fn new(
        kernel_build_hash: Hash32,
        serpi_k_hash: Hash32,
        vm_hash: Hash32,
        schema_library_hash: Hash32,
        motif_library_hash: Hash32,
        class_c_hash: Hash32,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&kernel_build_hash);
        buf.extend_from_slice(&serpi_k_hash);
        buf.extend_from_slice(&vm_hash);
        buf.extend_from_slice(&schema_library_hash);
        buf.extend_from_slice(&motif_library_hash);
        buf.extend_from_slice(&class_c_hash);
        let manifest_hash = hash::H(&buf);

        Self {
            kernel_build_hash,
            serpi_k_hash,
            vm_hash,
            schema_library_hash,
            motif_library_hash,
            class_c_hash,
            manifest_hash,
        }
    }
}

impl SerPi for KernelManifest {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// CLASS_C — the declared class of statements claimed decidable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassC {
    pub grammar: String,
    pub allowed_schemas: Vec<SchemaId>,
    pub primitives: AllowedPrimitives,
    pub proven_motifs: Vec<Hash32>,
    pub class_hash: Hash32,
}

impl ClassC {
    pub fn new(
        grammar: String,
        allowed_schemas: Vec<SchemaId>,
        primitives: AllowedPrimitives,
        proven_motifs: Vec<Hash32>,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(grammar.as_bytes());
        for s in &allowed_schemas {
            buf.extend_from_slice(&s.ser_pi());
        }
        buf.extend_from_slice(&primitives.ser_pi());
        for h in &proven_motifs {
            buf.extend_from_slice(h);
        }
        let class_hash = hash::H(&buf);

        Self {
            grammar,
            allowed_schemas,
            primitives,
            proven_motifs,
            class_hash,
        }
    }
}

impl SerPi for ClassC {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// FRC coverage metrics — the one metric that tells you you're solving problems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrcMetrics {
    pub total_statements: u64,
    pub frc_found: u64,
    pub invalid_with_frontier: u64,
    pub gap_count: u64,
    pub distinct_gap_patterns: u64,
    pub motif_count: u64,
    pub coverage_rate_milli: u64,   // frc_found * 1000 / total_statements
    pub gap_shrink_rate_milli: u64, // reduction in distinct gaps per iteration * 1000
}

impl SerPi for FrcMetrics {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

// === Invariant Reduction Certificate (IRC) ===
//
// For a statement S ≡ ∀n, P(n), an IRC is:
//   IRC(S) = (I, Base, Step, Link)
// such that:
//   I is a finite description of an invariant predicate I(n)
//   Base proves I(0)
//   Step proves ∀n, I(n) → I(n+1)
//   Link proves ∀n, I(n) → P(n)
//
// Then ∀n, P(n) follows by Nat induction.
// FRC becomes a subroutine for discharging Base/Step/Link obligations.

/// A transition system modeling ∀n, P(n) as an induction target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionSystem {
    /// State space description, e.g. "Nat" or "(Nat, AuxState)"
    pub state_desc: String,
    /// Transition description, e.g. "n → n + 1"
    pub transition_desc: String,
    /// Property description, e.g. "P(n)"
    pub property_desc: String,
    /// Which problem this models
    pub problem_id: String,
    /// Deterministic hash of this transition system
    pub ts_hash: Hash32,
}

impl TransitionSystem {
    pub fn new(
        state_desc: String,
        transition_desc: String,
        property_desc: String,
        problem_id: String,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(state_desc.as_bytes());
        buf.extend_from_slice(transition_desc.as_bytes());
        buf.extend_from_slice(property_desc.as_bytes());
        buf.extend_from_slice(problem_id.as_bytes());
        let ts_hash = hash::H(&buf);

        Self { state_desc, transition_desc, property_desc, problem_id, ts_hash }
    }
}

impl SerPi for TransitionSystem {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// The kind of invariant being used.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum InvariantKind {
    /// I(n) = ∀m ≤ n, Q(m) — prefix accumulator
    Prefix,
    /// I(n) = f(n) ≤ bound — monotone bounding
    Bounding,
    /// I(n) = R(n mod k) — periodic/modular
    Modular,
    /// I(n) = "state(n) ∈ S" for finite state set S
    Structural,
    /// Conjunction or implication chain of simpler invariants
    Composite,
    /// Problem-specific hand-crafted invariant
    Specialized,
    /// Derived via SEC (Self-Extending Calculus) proven rule
    SECDerived,
}

impl SerPi for InvariantKind {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// An invariant — a finite description of a predicate I(n).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Invariant {
    /// What kind of invariant
    pub kind: InvariantKind,
    /// Human-readable description
    pub description: String,
    /// Lean4-compatible formal definition
    pub formal_def: String,
    /// Deterministic hash
    pub invariant_hash: Hash32,
}

impl Invariant {
    pub fn new(kind: InvariantKind, description: String, formal_def: String) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&kind.ser_pi());
        buf.extend_from_slice(description.as_bytes());
        buf.extend_from_slice(formal_def.as_bytes());
        let invariant_hash = hash::H(&buf);

        Self { kind, description, formal_def, invariant_hash }
    }
}

impl SerPi for Invariant {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Which obligation in the IRC triple (Base, Step, Link).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObligationKind {
    /// I(0) — the base case
    Base,
    /// ∀n, I(n) → I(n+1) — the inductive step
    Step,
    /// ∀n, I(n) → P(n) — the link to the target property
    Link,
}

impl SerPi for ObligationKind {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Status of an IRC obligation — discharged or gap.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObligationStatus {
    /// Discharged by a proof (FRC, algebraic, or symbolic)
    Discharged {
        method: String,
        proof_hash: Hash32,
        lean_proof: Option<String>,
    },
    /// Could not be discharged — this IS the gap
    Gap {
        reason: String,
        attempted_methods: Vec<String>,
    },
}

impl SerPi for ObligationStatus {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// A proof obligation: Base, Step, or Link.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrcObligation {
    /// Which of the three obligations
    pub kind: ObligationKind,
    /// Formal statement of the obligation
    pub statement: String,
    /// Whether it has been discharged
    pub status: ObligationStatus,
    /// Deterministic hash
    pub obligation_hash: Hash32,
}

impl IrcObligation {
    pub fn new(kind: ObligationKind, statement: String, status: ObligationStatus) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&kind.ser_pi());
        buf.extend_from_slice(statement.as_bytes());
        buf.extend_from_slice(&status.ser_pi());
        let obligation_hash = hash::H(&buf);

        Self { kind, statement, status, obligation_hash }
    }

    pub fn is_discharged(&self) -> bool {
        matches!(self.status, ObligationStatus::Discharged { .. })
    }
}

impl SerPi for IrcObligation {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Invariant Reduction Certificate — complete proof of ∀n, P(n) via induction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Irc {
    /// The transition system being proved
    pub transition_system: TransitionSystem,
    /// The invariant I(n)
    pub invariant: Invariant,
    /// I(0)
    pub base: IrcObligation,
    /// ∀n, I(n) → I(n+1)
    pub step: IrcObligation,
    /// ∀n, I(n) → P(n)
    pub link: IrcObligation,
    /// Hash of the target statement
    pub statement_hash: Hash32,
    /// Hash of the entire IRC
    pub irc_hash: Hash32,
}

impl Irc {
    pub fn new(
        transition_system: TransitionSystem,
        invariant: Invariant,
        base: IrcObligation,
        step: IrcObligation,
        link: IrcObligation,
        statement_hash: Hash32,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&transition_system.ser_pi());
        buf.extend_from_slice(&invariant.ser_pi());
        buf.extend_from_slice(&base.ser_pi());
        buf.extend_from_slice(&step.ser_pi());
        buf.extend_from_slice(&link.ser_pi());
        buf.extend_from_slice(&statement_hash);
        let irc_hash = hash::H(&buf);

        Self { transition_system, invariant, base, step, link, statement_hash, irc_hash }
    }

    /// Count how many obligations are discharged (0-3).
    pub fn obligations_discharged(&self) -> u8 {
        let mut count = 0u8;
        if self.base.is_discharged() { count += 1; }
        if self.step.is_discharged() { count += 1; }
        if self.link.is_discharged() { count += 1; }
        count
    }

    /// True iff all three obligations are discharged — complete proof.
    pub fn is_complete(&self) -> bool {
        self.obligations_discharged() == 3
    }

    /// Verify internal consistency: hashes match, obligations bind correctly.
    pub fn verify_internal(&self) -> bool {
        // Recompute IRC hash
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.transition_system.ser_pi());
        buf.extend_from_slice(&self.invariant.ser_pi());
        buf.extend_from_slice(&self.base.ser_pi());
        buf.extend_from_slice(&self.step.ser_pi());
        buf.extend_from_slice(&self.link.ser_pi());
        buf.extend_from_slice(&self.statement_hash);

        if hash::H(&buf) != self.irc_hash {
            return false;
        }

        // Verify obligation kinds are correct
        if self.base.kind != ObligationKind::Base { return false; }
        if self.step.kind != ObligationKind::Step { return false; }
        if self.link.kind != ObligationKind::Link { return false; }

        true
    }
}

impl SerPi for Irc {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Result of IRC search.
#[derive(Debug, Clone)]
pub enum IrcResult {
    /// All three obligations discharged — complete proof of ∀n, P(n)
    Proved(Irc),
    /// At least one obligation remains as a gap
    Frontier(IrcFrontier),
}

/// IRC frontier — documents what was tried and what failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrcFrontier {
    pub statement_hash: Hash32,
    pub candidates_tried: Vec<InvariantCandidate>,
    pub best_candidate: Option<Irc>,
    pub frontier_hash: Hash32,
}

impl IrcFrontier {
    pub fn new(
        statement_hash: Hash32,
        candidates_tried: Vec<InvariantCandidate>,
        best_candidate: Option<Irc>,
    ) -> Self {
        let mut buf = Vec::new();
        buf.extend_from_slice(&statement_hash);
        for c in &candidates_tried {
            buf.extend_from_slice(&c.ser_pi());
        }
        if let Some(ref irc) = best_candidate {
            buf.extend_from_slice(&irc.ser_pi());
        }
        let frontier_hash = hash::H(&buf);

        Self { statement_hash, candidates_tried, best_candidate, frontier_hash }
    }
}

impl SerPi for IrcFrontier {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

/// Record of an invariant candidate attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvariantCandidate {
    pub invariant: Invariant,
    pub base_status: ObligationStatus,
    pub step_status: ObligationStatus,
    pub link_status: ObligationStatus,
    pub obligations_discharged: u8,
}

impl SerPi for InvariantCandidate {
    fn ser_pi(&self) -> Vec<u8> {
        kernel_types::serpi::canonical_cbor_bytes(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{Instruction, Program};

    fn make_test_program() -> Program {
        Program::new(vec![
            Instruction::Push(1),
            Instruction::Halt(1),
        ])
    }

    fn make_test_frc() -> Frc {
        let prog = make_test_program();
        let prog_hash = prog.ser_pi_hash();
        let stmt_hash = hash::H(b"test_statement");

        let proof_eq = ProofEq {
            statement_hash: stmt_hash,
            program_hash: prog_hash,
            b_star: 100,
            reduction_chain: vec![ReductionStep {
                from_hash: stmt_hash,
                to_hash: prog_hash,
                justification: "direct reduction".to_string(),
                step_hash: hash::H(b"step1"),
            }],
            proof_hash: hash::H(b"proof_eq"),
            lean_proof: None,
        };

        let proof_total = ProofTotal {
            program_hash: prog_hash,
            b_star: 100,
            halting_argument: "program has 2 instructions, halts at instruction 1".to_string(),
            proof_hash: hash::H(b"proof_total"),
            lean_proof: None,
        };

        Frc::new(prog, 100, proof_eq, proof_total, SchemaId::FiniteSearch, stmt_hash)
    }

    #[test]
    fn frc_verify_internal() {
        let frc = make_test_frc();
        assert!(frc.verify_internal());
    }

    #[test]
    fn frc_serpi_deterministic() {
        let f1 = make_test_frc();
        let f2 = make_test_frc();
        assert_eq!(f1.ser_pi(), f2.ser_pi());
        assert_eq!(f1.frc_hash, f2.frc_hash);
    }

    #[test]
    fn frc_hash_differs_by_statement() {
        let prog = make_test_program();
        let prog_hash = prog.ser_pi_hash();
        let stmt1 = hash::H(b"statement_1");
        let stmt2 = hash::H(b"statement_2");

        let proof_eq1 = ProofEq {
            statement_hash: stmt1,
            program_hash: prog_hash,
            b_star: 100,
            reduction_chain: vec![],
            proof_hash: hash::H(b"eq1"),
            lean_proof: None,
        };
        let proof_eq2 = ProofEq {
            statement_hash: stmt2,
            program_hash: prog_hash,
            b_star: 100,
            reduction_chain: vec![],
            proof_hash: hash::H(b"eq2"),
            lean_proof: None,
        };

        let pt = ProofTotal {
            program_hash: prog_hash,
            b_star: 100,
            halting_argument: "halts".to_string(),
            proof_hash: hash::H(b"pt"),
            lean_proof: None,
        };

        let frc1 = Frc::new(prog.clone(), 100, proof_eq1, pt.clone(), SchemaId::FiniteSearch, stmt1);
        let frc2 = Frc::new(prog, 100, proof_eq2, pt, SchemaId::FiniteSearch, stmt2);

        assert_ne!(frc1.frc_hash, frc2.frc_hash);
    }

    #[test]
    fn schema_id_ordering() {
        assert!(SchemaId::BoundedCounterexample < SchemaId::FiniteSearch);
        assert!(SchemaId::FiniteSearch < SchemaId::EffectiveCompactness);
        assert!(SchemaId::EffectiveCompactness < SchemaId::ProofMining);
        assert!(SchemaId::ProofMining < SchemaId::AlgebraicDecision);
        assert!(SchemaId::AlgebraicDecision < SchemaId::CertifiedNumerics);
    }

    #[test]
    fn frontier_witness_deterministic() {
        let stmt = hash::H(b"test");
        let fw1 = FrontierWitness::new(stmt, vec![SchemaId::FiniteSearch], vec![], None);
        let fw2 = FrontierWitness::new(stmt, vec![SchemaId::FiniteSearch], vec![], None);
        assert_eq!(fw1.frontier_hash, fw2.frontier_hash);
    }

    #[test]
    fn opp_hash_deterministic() {
        let opp1 = OpenProblemPackage::new(
            "forall x, P(x)".to_string(),
            "".to_string(),
            TargetClass { allowed_schemas: vec![SchemaId::FiniteSearch], grammar_description: "first-order".to_string() },
            AllowedPrimitives { max_vm_steps: 10000, max_memory_slots: 256, cost_model: "unit".to_string() },
            ExpectedOutput::Either,
        );
        let opp2 = OpenProblemPackage::new(
            "forall x, P(x)".to_string(),
            "".to_string(),
            TargetClass { allowed_schemas: vec![SchemaId::FiniteSearch], grammar_description: "first-order".to_string() },
            AllowedPrimitives { max_vm_steps: 10000, max_memory_slots: 256, cost_model: "unit".to_string() },
            ExpectedOutput::Either,
        );
        assert_eq!(opp1.opp_hash, opp2.opp_hash);
    }

    #[test]
    fn gap_serpi_deterministic() {
        let g = Gap {
            goal_hash: hash::H(b"goal"),
            goal_statement: "∀x, P(x)".to_string(),
            schema_id: SchemaId::BoundedCounterexample,
            dependency_hashes: vec![],
            unresolved_bound: Some("B*(x) = x^2 + 1".to_string()),
        };
        assert_eq!(g.ser_pi(), g.ser_pi());
    }

    #[test]
    fn missing_lemma_serpi_deterministic() {
        let ml = MissingLemma {
            lemma_hash: hash::H(b"lemma"),
            lemma_statement: "P(x) → Q(x) for all x ≤ B*".to_string(),
            needed_by_schema: SchemaId::ProofMining,
            needed_for_goal: hash::H(b"goal"),
        };
        assert_eq!(ml.ser_pi(), ml.ser_pi());
    }

    #[test]
    fn class_c_hash_deterministic() {
        let c1 = ClassC::new(
            "first-order arithmetic".to_string(),
            vec![SchemaId::FiniteSearch, SchemaId::BoundedCounterexample],
            AllowedPrimitives { max_vm_steps: 10000, max_memory_slots: 256, cost_model: "unit".to_string() },
            vec![],
        );
        let c2 = ClassC::new(
            "first-order arithmetic".to_string(),
            vec![SchemaId::FiniteSearch, SchemaId::BoundedCounterexample],
            AllowedPrimitives { max_vm_steps: 10000, max_memory_slots: 256, cost_model: "unit".to_string() },
            vec![],
        );
        assert_eq!(c1.class_hash, c2.class_hash);
    }

    #[test]
    fn frc_receipt_deterministic() {
        let r1 = FrcReceipt::new(
            hash::H(b"frc"), 1, hash::H(b"trace"), hash::H(b"merkle"),
            hash::H(b"stmt"), true,
        );
        let r2 = FrcReceipt::new(
            hash::H(b"frc"), 1, hash::H(b"trace"), hash::H(b"merkle"),
            hash::H(b"stmt"), true,
        );
        assert_eq!(r1.receipt_hash, r2.receipt_hash);
    }

    #[test]
    fn expected_output_serpi_differ() {
        assert_ne!(ExpectedOutput::Proof.ser_pi(), ExpectedOutput::Disproof.ser_pi());
        assert_ne!(ExpectedOutput::Proof.ser_pi(), ExpectedOutput::Either.ser_pi());
    }

    #[test]
    fn kernel_manifest_deterministic() {
        let m1 = KernelManifest::new(
            hash::H(b"build"), hash::H(b"serpi"), hash::H(b"vm"),
            hash::H(b"schemas"), hash::H(b"motifs"), hash::H(b"class_c"),
        );
        let m2 = KernelManifest::new(
            hash::H(b"build"), hash::H(b"serpi"), hash::H(b"vm"),
            hash::H(b"schemas"), hash::H(b"motifs"), hash::H(b"class_c"),
        );
        assert_eq!(m1.manifest_hash, m2.manifest_hash);
    }

    // === IRC tests ===

    fn make_test_ts() -> TransitionSystem {
        TransitionSystem::new(
            "Nat".to_string(),
            "n → n + 1".to_string(),
            "P(n)".to_string(),
            "test_problem".to_string(),
        )
    }

    fn make_test_invariant() -> Invariant {
        Invariant::new(
            InvariantKind::Prefix,
            "∀m ≤ n, P(m)".to_string(),
            "def testInvariant (n : Nat) : Prop := ∀ m, m ≤ n → P m".to_string(),
        )
    }

    #[test]
    fn irc_complete_verify_internal() {
        let ts = make_test_ts();
        let inv = make_test_invariant();
        let stmt_hash = hash::H(b"test_full_statement");

        let base = IrcObligation::new(
            ObligationKind::Base,
            "I(0)".to_string(),
            ObligationStatus::Discharged {
                method: "Trivial".to_string(),
                proof_hash: hash::H(b"base_proof"),
                lean_proof: None,
            },
        );
        let step = IrcObligation::new(
            ObligationKind::Step,
            "∀n, I(n) → I(n+1)".to_string(),
            ObligationStatus::Discharged {
                method: "FRC:BoundedCounterexample".to_string(),
                proof_hash: hash::H(b"step_proof"),
                lean_proof: None,
            },
        );
        let link = IrcObligation::new(
            ObligationKind::Link,
            "∀n, I(n) → P(n)".to_string(),
            ObligationStatus::Discharged {
                method: "Trivial".to_string(),
                proof_hash: hash::H(b"link_proof"),
                lean_proof: None,
            },
        );

        let irc = Irc::new(ts, inv, base, step, link, stmt_hash);
        assert!(irc.verify_internal());
        assert!(irc.is_complete());
        assert_eq!(irc.obligations_discharged(), 3);
    }

    #[test]
    fn irc_frontier_verify_internal() {
        let ts = make_test_ts();
        let inv = make_test_invariant();
        let stmt_hash = hash::H(b"open_problem");

        let base = IrcObligation::new(
            ObligationKind::Base,
            "I(0)".to_string(),
            ObligationStatus::Discharged {
                method: "Trivial".to_string(),
                proof_hash: hash::H(b"base"),
                lean_proof: None,
            },
        );
        let step = IrcObligation::new(
            ObligationKind::Step,
            "∀n, I(n) → I(n+1)".to_string(),
            ObligationStatus::Gap {
                reason: "This is the open problem".to_string(),
                attempted_methods: vec!["FRC".to_string(), "Algebraic".to_string()],
            },
        );
        let link = IrcObligation::new(
            ObligationKind::Link,
            "∀n, I(n) → P(n)".to_string(),
            ObligationStatus::Discharged {
                method: "Trivial".to_string(),
                proof_hash: hash::H(b"link"),
                lean_proof: None,
            },
        );

        let irc = Irc::new(ts, inv, base, step, link, stmt_hash);
        assert!(irc.verify_internal());
        assert!(!irc.is_complete());
        assert_eq!(irc.obligations_discharged(), 2);
    }

    #[test]
    fn irc_hash_deterministic() {
        let mk = || {
            let ts = make_test_ts();
            let inv = make_test_invariant();
            let base = IrcObligation::new(
                ObligationKind::Base, "I(0)".to_string(),
                ObligationStatus::Discharged {
                    method: "Trivial".to_string(),
                    proof_hash: hash::H(b"b"), lean_proof: None,
                },
            );
            let step = IrcObligation::new(
                ObligationKind::Step, "step".to_string(),
                ObligationStatus::Gap {
                    reason: "open".to_string(), attempted_methods: vec![],
                },
            );
            let link = IrcObligation::new(
                ObligationKind::Link, "link".to_string(),
                ObligationStatus::Discharged {
                    method: "Trivial".to_string(),
                    proof_hash: hash::H(b"l"), lean_proof: None,
                },
            );
            Irc::new(ts, inv, base, step, link, hash::H(b"s"))
        };
        assert_eq!(mk().irc_hash, mk().irc_hash);
        assert_eq!(mk().ser_pi(), mk().ser_pi());
    }

    #[test]
    fn irc_frontier_deterministic() {
        let stmt = hash::H(b"test");
        let f1 = IrcFrontier::new(stmt, vec![], None);
        let f2 = IrcFrontier::new(stmt, vec![], None);
        assert_eq!(f1.frontier_hash, f2.frontier_hash);
    }

    #[test]
    fn invariant_kind_serpi_differ() {
        assert_ne!(InvariantKind::Prefix.ser_pi(), InvariantKind::Bounding.ser_pi());
        assert_ne!(InvariantKind::Modular.ser_pi(), InvariantKind::Structural.ser_pi());
    }

    #[test]
    fn transition_system_deterministic() {
        let ts1 = make_test_ts();
        let ts2 = make_test_ts();
        assert_eq!(ts1.ts_hash, ts2.ts_hash);
    }
}
