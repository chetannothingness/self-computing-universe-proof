//! SEC — Self-Extending Calculus: Proof-Carrying Rule Miner.
//!
//! SEC replaces the dead-end "Gap" with a proof-carrying rule miner.
//! The kernel enumerates candidate inference schemas, proves their soundness
//! in Lean4 (no sorry), and only then adds them as usable rules.
//! Rules are objects with proofs, not heuristics.
//!
//! Architecture:
//! ```text
//! Gap (failing obligation)
//!   ↓
//! SEC Engine
//!   ├─ RuleSynEnumerator: enumerate candidate rule schemas by (size, hash)
//!   ├─ rule_lean_gen: generate Sound_<hash>.lean for each candidate
//!   ├─ lean_runner: run `lake build` — Lean is the ONLY soundness oracle
//!   ├─ If Lean accepts (no sorry): add ProvenRule to RuleDb (Merkle-committed)
//!   └─ Retry structural checker with enlarged rule set
//!   ↓
//! StructuralVerdict::Verified (or still NotVerifiable)
//! ```

pub mod rule_syn;
pub mod prefix_ban;
pub mod rule_db;
pub mod rule_enum;
pub mod rule_lean_gen;
pub mod sec_engine;

pub use rule_syn::{RuleSyn, RuleExpr, RuleKind};
pub use prefix_ban::{is_prefix_invariant, step_is_independent};
pub use rule_db::{ProvenRule, RuleDb};
pub use rule_enum::{enumerate_candidates, enumerate_for_gap};
pub use rule_lean_gen::{generate_soundness_file, theorem_name_for_rule, import_for_rule};
pub use sec_engine::{SecEngine, SecResult, GapTarget};
