use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use crate::alphabet::AnswerAlphabet;
use serde::{Serialize, Deserialize};

/// How to evaluate candidate answers against the contract.
///
/// For finite domain contracts, this specifies which candidates
/// are satisfying. The evaluator is INSIDE K, not imported.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EvalSpec {
    /// Lookup table: maps candidate → {SAT, UNSAT}.
    /// Represented as a list of (candidate, result) pairs.
    Table(Vec<(Vec<u8>, Vec<u8>)>),

    /// Boolean formula in CNF over variables 0..n-1.
    /// Each clause is a list of literals (positive = var, negative = NOT var).
    BoolCnf {
        num_vars: usize,
        clauses: Vec<Vec<i32>>,
    },

    /// Arithmetic: find x in [lo, hi] such that f(x) == target.
    /// f is represented as a polynomial with integer coefficients.
    ArithFind {
        coefficients: Vec<i64>,
        target: i64,
    },

    /// Formal proof verification.
    /// The candidate is a proof term; evaluation requires a pinned
    /// formal verifier (Lean/Coq/Isabelle). Since the verifier is
    /// external and the proof space is not finitely enumerable,
    /// the kernel MUST return Ω with a frontier describing what
    /// separator is missing.
    ///
    /// This is NOT a failure — it is the structurally correct output
    /// for a problem whose answer requires witnessing beyond the
    /// kernel's current instrument closure.
    FormalProof {
        /// The formal statement to prove/disprove.
        statement: String,
        /// The formal system (e.g., "Lean4", "Isabelle/HOL").
        formal_system: String,
        /// Known dependencies / required lemmas.
        known_dependencies: Vec<String>,
        /// The specific frontier: what would need to be true
        /// for UNIQUE to be achievable.
        required_separator: String,
    },

    /// Dominance evaluation: DOMINATE(S, M).
    /// Binary verdict: DOMINANT or NOT_DOMINANT.
    /// The kernel runs a task suite, compares its score against
    /// a competitor using lexicographic scoring, and produces
    /// a proof-carrying verdict.
    Dominate {
        /// Hash of the task suite.
        suite_hash: Vec<u8>,
        /// Identifier of the competitor being compared against.
        competitor_id: String,
        /// Scoring rule: lexicographic on (verified_success, -false_claims, -cost).
        scoring: String,
    },

    /// SpaceEngine verification: verify catalog integrity against kernel state.
    /// Used for both Part A (kernel-derived) and Part B (real-universe) —
    /// the difference is in which hashes are pinned.
    SpaceEngine {
        /// Merkle root of catalog files.
        catalog_hash: Vec<u8>,
        /// H(.se script bytes).
        scenario_hash: Vec<u8>,
        /// BuildHash(K) that generated the catalogs.
        kernel_build_hash: Vec<u8>,
    },
}

impl SerPi for EvalSpec {
    fn ser_pi(&self) -> Vec<u8> {
        match self {
            EvalSpec::Table(pairs) => canonical_cbor_bytes(&("Table", pairs)),
            EvalSpec::BoolCnf { num_vars, clauses } => {
                canonical_cbor_bytes(&("BoolCnf", *num_vars as u64, clauses))
            }
            EvalSpec::ArithFind { coefficients, target } => {
                canonical_cbor_bytes(&("ArithFind", coefficients, target))
            }
            EvalSpec::FormalProof { statement, formal_system, known_dependencies, required_separator } => {
                canonical_cbor_bytes(&("FormalProof", statement.as_str(), formal_system.as_str(),
                    known_dependencies, required_separator.as_str()))
            }
            EvalSpec::Dominate { suite_hash, competitor_id, scoring } => {
                canonical_cbor_bytes(&("Dominate", suite_hash, competitor_id.as_str(), scoring.as_str()))
            }
            EvalSpec::SpaceEngine { catalog_hash, scenario_hash, kernel_build_hash } => {
                canonical_cbor_bytes(&("SpaceEngine", catalog_hash, scenario_hash, kernel_build_hash))
            }
        }
    }
}

/// Tie-break rule for when multiple answers survive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Tiebreak {
    /// Lexicographically smallest answer.
    LexMin,
    /// First found (by canonical instrument order).
    FirstFound,
}

impl SerPi for Tiebreak {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            Tiebreak::LexMin => 0,
            Tiebreak::FirstFound => 1,
        };
        canonical_cbor_bytes(&tag)
    }
}

/// A compiled contract: the finite object that the solver operates on.
///
/// Must include:
/// - AnswerType (finite alphabet)
/// - Verifier inside K
/// - Budget
/// - Tiebreak
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contract {
    /// Unique ID: H(Ser_Π(contract)).
    pub qid: Hash32,
    /// The finite answer alphabet.
    pub answer_alphabet: AnswerAlphabet,
    /// How to evaluate candidates.
    pub eval: EvalSpec,
    /// Budget for solving.
    pub budget: ContractBudget,
    /// Tie-break rule.
    pub tiebreak: Tiebreak,
    /// Human-readable description.
    pub description: String,
}

/// Budget embedded in a contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractBudget {
    pub max_cost: u64,
    pub max_steps: u64,
}

impl SerPi for ContractBudget {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(self.max_cost, self.max_steps))
    }
}

impl Contract {
    /// Create a contract and compute its canonical ID.
    pub fn new(
        answer_alphabet: AnswerAlphabet,
        eval: EvalSpec,
        budget: ContractBudget,
        tiebreak: Tiebreak,
        description: String,
    ) -> Self {
        let mut c = Contract {
            qid: [0u8; 32],
            answer_alphabet,
            eval,
            budget,
            tiebreak,
            description,
        };
        c.qid = c.compute_qid();
        c
    }

    fn compute_qid(&self) -> Hash32 {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.answer_alphabet.ser_pi());
        buf.extend_from_slice(&self.eval.ser_pi());
        buf.extend_from_slice(&self.budget.ser_pi());
        buf.extend_from_slice(&self.tiebreak.ser_pi());
        buf.extend_from_slice(&self.description.ser_pi());
        hash::H(&buf)
    }
}

impl SerPi for Contract {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid.ser_pi());
        buf.extend_from_slice(&self.answer_alphabet.ser_pi());
        buf.extend_from_slice(&self.eval.ser_pi());
        buf.extend_from_slice(&self.budget.ser_pi());
        buf.extend_from_slice(&self.tiebreak.ser_pi());
        buf.extend_from_slice(&self.description.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}
