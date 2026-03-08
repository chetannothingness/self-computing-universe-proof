//! UCert — Universal Certificate Calculus engine.
//!
//! Pipeline:
//!   Compile S to U → Enumerate Cert by rank → Check(S, cert) → PROVED(S, π)
//!
//! The zero-doubt object:
//!   ∀ S ∈ U, if ∃ cert such that Check(S, cert) = true, then NF(S) finds it.
//!
//! This module provides:
//!   universe  — Statement type (mirrors Lean UCert.Universe)
//!   cert      — Certificate types (mirrors Lean UCert.Cert)
//!   check     — Universal checker (mirrors Lean UCert.Check)
//!   enumerate — Complete enumerator E: ℕ → Cert
//!   normalize — Normalizer: runs E until Check succeeds
//!   optimize  — Speed: compression, sharding, pruning, motifs
//!   compile   — Compile all 20 problems to Statement

pub mod universe;
pub mod cert;
pub mod check;
pub mod enumerate;
pub mod normalize;
pub mod optimize;
pub mod compile;

pub use universe::Statement;
pub use cert::{Cert, InvCert, BaseCert, StepCert, LinkCert};
pub use check::check;
pub use enumerate::CertEnumerator;
pub use normalize::{ucert_normalize, NormalizeResult};
pub use compile::compile_problem;
