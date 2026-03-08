//! InvSyn — Structural Invariant Synthesis Engine
//!
//! Replaces bounded FRC discharge with structural invariant synthesis.
//! The engine finds an invariant `inv : InvSyn.Expr` whose structural properties
//! guarantee Base/Step/Link, and emits real Lean4 proof terms via
//! `native_decide` on finite checkers + soundness theorems.
//!
//! Core rule: ∞ is a fixed point. Every unbounded proof is a finite invariant
//! certificate whose Base/Step/Link are actual Lean theorems produced from
//! finite checkers + soundness lemmas. No bounded prefix ever discharges ∀.

pub mod ast;
pub mod eval;
pub mod layers;
pub mod search;
pub mod proof_gen;
pub mod normalize;
pub mod structural;
pub mod structural_cert;

pub use ast::{Expr, Layer};
pub use eval::{eval, eval_bool, mk_env, mk_env2, to_prop};
pub use search::{InvSynSearch, InvSynResult};
pub use normalize::{normalize, ReachabilityProblem};
pub use proof_gen::{generate_lean_proof, generate_proved_lean_file, generate_frontier_lean_file, LeanProofBundle};
pub use structural::{structural_step_check, structural_link_check, structural_step_check_with_rules, structural_link_check_with_rules, StructuralVerdict};
pub use structural_cert::{eval_bool_with_trace, generate_trace_corpus, anti_unify, validate_schema, emit_certificates, run_pipeline, run_pipeline_auto, generate_bounded_vacuous_lean_proof, get_problem_body, generate_all_proofs, PipelineResult};
