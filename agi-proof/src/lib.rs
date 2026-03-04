pub mod eval_specs;
pub mod compiler_ext;
pub mod runner;
pub mod receipt_bundle;
pub mod release;
pub mod phase_criteria;
pub mod suite_gen;

// Phase simulators — each with world generation, judges, and tests
pub mod phase2;
pub mod phase3;
pub mod phase4;
pub mod phase5;
pub mod phase6;
pub mod phase7;
pub mod phase8;
