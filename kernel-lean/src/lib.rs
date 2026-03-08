//! kernel-lean: Lean4 code generation from Rust FRC types.
//!
//! Converts VM programs, ProofEq, ProofTotal, and FRC results into
//! syntactically valid Lean4 source files that can be verified by `lake build`.

pub mod program_embed;
pub mod proof_eq_gen;
pub mod proof_total_gen;
pub mod result_gen;
pub mod manifest_gen;
pub mod bundle_gen;
pub mod lean_runner;
pub mod irc_gen;
pub mod irc_result_gen;
