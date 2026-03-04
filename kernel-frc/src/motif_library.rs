// Motif Library — stores proven lemmas for reuse.
//
// When a gap is resolved, the resulting lemma is added here.
// The FRC search engine consults this library before attempting
// schema reduction, potentially short-circuiting known results.

use std::collections::BTreeMap;
use kernel_types::{Hash32, hash};
use crate::frc_types::Frc;

/// A proven motif — a reusable lemma with its FRC.
#[derive(Debug, Clone)]
pub struct Motif {
    pub goal_hash: Hash32,
    pub lemma_description: String,
    pub frc: Frc,
    pub use_count: u64,
}

/// The motif library — canonical store of proven lemmas.
pub struct MotifLibrary {
    motifs: BTreeMap<Hash32, Motif>,
}

impl MotifLibrary {
    pub fn new() -> Self {
        Self {
            motifs: BTreeMap::new(),
        }
    }

    /// Add a proven motif (lemma + FRC).
    pub fn add_motif(&mut self, goal_hash: Hash32, description: String, frc: Frc) {
        self.motifs.insert(goal_hash, Motif {
            goal_hash,
            lemma_description: description,
            frc,
            use_count: 0,
        });
    }

    /// Look up a motif by goal hash.
    pub fn get_motif(&self, goal_hash: &Hash32) -> Option<&Motif> {
        self.motifs.get(goal_hash)
    }

    /// Record a use of a motif (for tracking which lemmas are most useful).
    pub fn record_use(&mut self, goal_hash: &Hash32) {
        if let Some(motif) = self.motifs.get_mut(goal_hash) {
            motif.use_count += 1;
        }
    }

    /// Check if a goal has already been proven.
    pub fn is_proven(&self, goal_hash: &Hash32) -> bool {
        self.motifs.contains_key(goal_hash)
    }

    /// Number of stored motifs.
    pub fn len(&self) -> usize {
        self.motifs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.motifs.is_empty()
    }

    /// All motif hashes (for ClassC).
    pub fn motif_hashes(&self) -> Vec<Hash32> {
        self.motifs.keys().copied().collect()
    }

    /// Library hash — Merkle identity of all stored motifs.
    pub fn library_hash(&self) -> Hash32 {
        let hashes: Vec<Hash32> = self.motifs.values()
            .map(|m| m.frc.frc_hash)
            .collect();
        if hashes.is_empty() {
            return kernel_types::HASH_ZERO;
        }
        hash::merkle_root(&hashes)
    }
}

impl Default for MotifLibrary {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::SerPi;
    use crate::vm::{Instruction, Program};
    use crate::frc_types::*;

    fn make_test_frc(label: &[u8]) -> Frc {
        let prog = Program::new(vec![Instruction::Halt(1)]);
        let prog_hash = prog.ser_pi_hash();
        let stmt = hash::H(label);

        let proof_eq = ProofEq {
            statement_hash: stmt,
            program_hash: prog_hash,
            b_star: 10,
            reduction_chain: vec![],
            proof_hash: hash::H(b"eq"),
        };
        let proof_total = ProofTotal {
            program_hash: prog_hash,
            b_star: 10,
            halting_argument: "trivial".to_string(),
            proof_hash: hash::H(b"total"),
        };

        Frc::new(prog, 10, proof_eq, proof_total, SchemaId::FiniteSearch, stmt)
    }

    #[test]
    fn empty_library() {
        let lib = MotifLibrary::new();
        assert!(lib.is_empty());
        assert_eq!(lib.library_hash(), kernel_types::HASH_ZERO);
    }

    #[test]
    fn add_and_retrieve() {
        let mut lib = MotifLibrary::new();
        let goal = hash::H(b"goal1");
        let frc = make_test_frc(b"goal1");
        lib.add_motif(goal, "test lemma".to_string(), frc);

        assert!(lib.is_proven(&goal));
        assert_eq!(lib.len(), 1);
        assert!(lib.get_motif(&goal).is_some());
    }

    #[test]
    fn use_counting() {
        let mut lib = MotifLibrary::new();
        let goal = hash::H(b"goal2");
        lib.add_motif(goal, "test".to_string(), make_test_frc(b"goal2"));

        lib.record_use(&goal);
        lib.record_use(&goal);
        lib.record_use(&goal);

        assert_eq!(lib.get_motif(&goal).unwrap().use_count, 3);
    }

    #[test]
    fn library_hash_deterministic() {
        let mut l1 = MotifLibrary::new();
        let mut l2 = MotifLibrary::new();

        let goal = hash::H(b"g");
        l1.add_motif(goal, "t".to_string(), make_test_frc(b"g"));
        l2.add_motif(goal, "t".to_string(), make_test_frc(b"g"));

        assert_eq!(l1.library_hash(), l2.library_hash());
    }

    #[test]
    fn library_hash_changes() {
        let mut lib = MotifLibrary::new();
        let h1 = lib.library_hash();
        lib.add_motif(hash::H(b"new"), "new".to_string(), make_test_frc(b"new"));
        let h2 = lib.library_hash();
        assert_ne!(h1, h2);
    }

    #[test]
    fn motif_hashes_list() {
        let mut lib = MotifLibrary::new();
        lib.add_motif(hash::H(b"a"), "a".to_string(), make_test_frc(b"a"));
        lib.add_motif(hash::H(b"b"), "b".to_string(), make_test_frc(b"b"));
        assert_eq!(lib.motif_hashes().len(), 2);
    }
}
