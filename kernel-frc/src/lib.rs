// kernel-frc: Finite Reduction Certificate engine.
//
// The FRC engine turns every target statement into a finite, checkable
// computation via an FRC, and then proves the reduction + executes it
// inside a verified interpreter. Everything else is plumbing.
//
// From FOUNDATION.md §12:
//   S admissible ⟺ ∃ FRC(S)
//   FRC(S) = (C, B*, ProofEq, ProofTotal)
//
// From FRC_OPEN_PROBLEMS.md:
//   To solve all open problems: (i) prove reductions to finite bounded
//   computation (FRC), (ii) run them inside a verified VM, (iii) turn
//   every failure into a minimal missing-lemma contract, and (iv) iterate
//   until schema+lemma closure covers the target class.

pub mod vm;
pub mod frc_types;
pub mod schema;
pub mod schemas;
pub mod gap_ledger;
pub mod motif_library;
pub mod frc_search;
pub mod opp;
pub mod opp_verify;
pub mod predicate;
pub mod program_builder;
pub mod contract_frc;
pub mod schema_induction;
pub mod class_c;
pub mod asm;
pub mod open_problems;
pub mod millennium_frc;

pub use vm::{Vm, Program, Instruction, VmOutcome, VmFault, VmState, ExecTrace};
pub use frc_types::{
    Frc, FrcResult, SchemaId, ProofEq, ProofTotal, ReductionStep,
    FrontierWitness, Gap, MissingLemma,
    OpenProblemPackage, TargetClass, AllowedPrimitives, ExpectedOutput,
    FrcReceipt, KernelManifest, ClassC, FrcMetrics,
};
pub use schema::Schema;
pub use gap_ledger::GapLedger;
pub use motif_library::MotifLibrary;
pub use frc_search::FrcSearch;
pub use opp::OppRunner;
pub use opp_verify::OppVerifier;
