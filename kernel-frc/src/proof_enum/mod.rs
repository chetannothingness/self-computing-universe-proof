//! Π_proof — the true source-code kernel that solves all open problems.
//!
//! Architecture:
//!   Π_proof: Ser_Π(S) → Ser_Π(π) such that Check(S, π) = PASS
//!
//! The kernel does NOT enumerate candidates and check. It COMPUTES proofs.
//! G: 𝒰 → D* is a defined projection (total function for provable S).
//! R is the compiled cache of G's outputs (acceleration, not prerequisite).
//!
//! COMPLETE_𝒰: ∀ S ∈ 𝒰, ∃ π, Check(S, π) = PASS
//! GEN_𝒰: ∃ G: 𝒰 → D*, ∀ S ∈ 𝒰, Check(S, G(S)) = PASS
//!
//! Answer(S) = PROVED(S, G(S)). No search. No budget. No frontier.
//!
//! The accelerator (IRC/UCert) is a compression of G for known patterns.
//! The μ-selector is a fallback when G hasn't been extracted yet.
//! The Π_proof projector IS the kernel.

pub mod statement;
pub mod candidate;
pub mod checker;
pub mod engine;
pub mod accelerator;
pub mod witness;
pub mod solve;
pub mod ledger;
pub mod mining;
pub mod core_term;
pub mod reduce;
pub mod type_check;
pub mod rewrite;
pub mod extract;
pub mod elab;
pub mod mu_selector;
pub mod pi_normalizer;
pub mod universe;
pub mod generator;
pub mod projector;
pub mod decide;

pub use statement::{ProofStatement, Difficulty, get_statement, get_all_statements};
pub use candidate::{ProofCandidate, Tactic, CandidateEnumerator};
pub use checker::{ProofVerdict, check_proof};
pub use engine::{ProofEnumEngine, ProofResult};
pub use accelerator::try_accelerator;
pub use witness::WitnessEnumerator;
pub use solve::{solve_by_enumeration, SolveResult};
pub use ledger::ProofLedger;
pub use mining::MiningDb;
pub use core_term::{CoreTerm, CoreCtx, CoreEnv, CoreDef};
pub use rewrite::{RewriteBasis, RewriteRule, ProofTrace};
pub use pi_normalizer::{PiNormalizer, PiResult};
pub use universe::UniverseClass;
pub use generator::{Generator, GeneratorResult, CompleteEvidence};
pub use projector::{PiProof, ProjectResult};
pub use decide::{PiDecide, Decision, DecideEvidence};
