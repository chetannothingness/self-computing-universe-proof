// Gap Ledger — tracks failed FRC attempts and missing lemma witnesses.
//
// Every failed FRC attempt produces:
//   Gap = (goal_hash, goal_statement, dependencies_hashes)
//
// Gaps are stored keyed by goal_hash. When a gap is resolved (a lemma
// is proven for it), the original FRC attempt can be retried.
//
// When a family of gaps repeats (same pattern), the kernel adds a new
// schema via schema induction.

use std::collections::BTreeMap;
use kernel_types::{Hash32, hash};
use crate::frc_types::{Gap, MissingLemma, SchemaId};

/// A resolved lemma — a gap that has been filled.
#[derive(Debug, Clone)]
pub struct ResolvedLemma {
    pub goal_hash: Hash32,
    pub lemma_hash: Hash32,
    pub resolved_by_schema: SchemaId,
}

/// The gap ledger — canonical database of unresolved gaps.
pub struct GapLedger {
    /// Active gaps keyed by goal_hash
    gaps: BTreeMap<Hash32, Gap>,
    /// Resolved gaps (for audit)
    resolved: BTreeMap<Hash32, ResolvedLemma>,
    /// Gap patterns: schema_id → count of gaps from that schema
    pattern_counts: BTreeMap<SchemaId, u64>,
}

impl GapLedger {
    pub fn new() -> Self {
        Self {
            gaps: BTreeMap::new(),
            resolved: BTreeMap::new(),
            pattern_counts: BTreeMap::new(),
        }
    }

    /// Record a gap from a failed FRC attempt.
    pub fn record_gap(&mut self, gap: Gap) {
        *self.pattern_counts.entry(gap.schema_id.clone()).or_insert(0) += 1;
        self.gaps.insert(gap.goal_hash, gap);
    }

    /// Mark a gap as resolved.
    pub fn resolve_gap(&mut self, goal_hash: Hash32, lemma_hash: Hash32, schema: SchemaId) {
        if self.gaps.remove(&goal_hash).is_some() {
            self.resolved.insert(goal_hash, ResolvedLemma {
                goal_hash,
                lemma_hash,
                resolved_by_schema: schema,
            });
        }
    }

    /// Get an unresolved gap by its goal hash.
    pub fn get_gap(&self, goal_hash: &Hash32) -> Option<&Gap> {
        self.gaps.get(goal_hash)
    }

    /// Get all unresolved gaps.
    pub fn active_gaps(&self) -> &BTreeMap<Hash32, Gap> {
        &self.gaps
    }

    /// Number of unresolved gaps.
    pub fn active_count(&self) -> usize {
        self.gaps.len()
    }

    /// Number of resolved gaps.
    pub fn resolved_count(&self) -> usize {
        self.resolved.len()
    }

    /// Find the minimal missing lemma — the gap with fewest dependencies.
    pub fn minimal_missing_lemma(&self) -> Option<MissingLemma> {
        self.gaps.values()
            .min_by_key(|g| g.dependency_hashes.len())
            .map(|g| MissingLemma {
                lemma_hash: g.goal_hash,
                lemma_statement: g.goal_statement.clone(),
                needed_by_schema: g.schema_id.clone(),
                needed_for_goal: g.goal_hash,
            })
    }

    /// Get gap pattern counts — schemas that produce repeated gaps
    /// are candidates for schema induction.
    pub fn pattern_counts(&self) -> &BTreeMap<SchemaId, u64> {
        &self.pattern_counts
    }

    /// Number of distinct gap patterns.
    pub fn distinct_patterns(&self) -> usize {
        self.pattern_counts.len()
    }

    /// Ledger hash — Merkle identity for the current gap state.
    pub fn ledger_hash(&self) -> Hash32 {
        let hashes: Vec<Hash32> = self.gaps.keys().copied().collect();
        if hashes.is_empty() {
            return kernel_types::HASH_ZERO;
        }
        hash::merkle_root(&hashes)
    }
}

impl Default for GapLedger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_gap(id: &[u8]) -> Gap {
        Gap {
            goal_hash: hash::H(id),
            goal_statement: format!("goal_{}", hex::encode(id)),
            schema_id: SchemaId::FiniteSearch,
            dependency_hashes: vec![],
            unresolved_bound: None,
        }
    }

    // hex helper since we don't have hex crate
    mod hex {
        pub fn encode(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{:02x}", b)).collect()
        }
    }

    #[test]
    fn empty_ledger() {
        let ledger = GapLedger::new();
        assert_eq!(ledger.active_count(), 0);
        assert_eq!(ledger.resolved_count(), 0);
        assert!(ledger.minimal_missing_lemma().is_none());
        assert_eq!(ledger.ledger_hash(), kernel_types::HASH_ZERO);
    }

    #[test]
    fn record_and_retrieve() {
        let mut ledger = GapLedger::new();
        let gap = make_gap(b"test1");
        let gh = gap.goal_hash;
        ledger.record_gap(gap);

        assert_eq!(ledger.active_count(), 1);
        assert!(ledger.get_gap(&gh).is_some());
    }

    #[test]
    fn resolve_gap() {
        let mut ledger = GapLedger::new();
        let gap = make_gap(b"test2");
        let gh = gap.goal_hash;
        ledger.record_gap(gap);

        ledger.resolve_gap(gh, hash::H(b"lemma"), SchemaId::BoundedCounterexample);
        assert_eq!(ledger.active_count(), 0);
        assert_eq!(ledger.resolved_count(), 1);
    }

    #[test]
    fn minimal_missing_lemma_fewest_deps() {
        let mut ledger = GapLedger::new();

        let mut g1 = make_gap(b"g1");
        g1.dependency_hashes = vec![hash::H(b"d1"), hash::H(b"d2")];
        let mut g2 = make_gap(b"g2");
        g2.dependency_hashes = vec![]; // fewer deps

        ledger.record_gap(g1);
        ledger.record_gap(g2.clone());

        let ml = ledger.minimal_missing_lemma().unwrap();
        assert_eq!(ml.lemma_hash, g2.goal_hash);
    }

    #[test]
    fn pattern_counting() {
        let mut ledger = GapLedger::new();
        ledger.record_gap(make_gap(b"a"));
        ledger.record_gap(make_gap(b"b"));
        ledger.record_gap(make_gap(b"c"));

        assert_eq!(*ledger.pattern_counts().get(&SchemaId::FiniteSearch).unwrap(), 3);
    }

    #[test]
    fn ledger_hash_deterministic() {
        let mut l1 = GapLedger::new();
        let mut l2 = GapLedger::new();

        l1.record_gap(make_gap(b"x"));
        l2.record_gap(make_gap(b"x"));

        assert_eq!(l1.ledger_hash(), l2.ledger_hash());
    }

    #[test]
    fn ledger_hash_changes_on_new_gap() {
        let mut ledger = GapLedger::new();
        let h1 = ledger.ledger_hash();
        ledger.record_gap(make_gap(b"new"));
        let h2 = ledger.ledger_hash();
        assert_ne!(h1, h2);
    }
}
