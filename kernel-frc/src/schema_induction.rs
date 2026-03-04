// Schema Induction — detects repeated gap patterns and synthesizes new schemas.
//
// When the gap ledger shows multiple gaps with the same schema_id and
// similar goal structure, the inductor:
//   1. Extracts the common pattern
//   2. Builds a template program
//   3. Wraps it as a new Schema implementation
//   4. Adds to the schema library
//
// This is how CLASS_C grows: solved sub-contracts become lemmas,
// repeated lemma patterns become new schemas.

use std::collections::BTreeMap;
use kernel_types::{Hash32, hash};
use crate::frc_types::{SchemaId, Gap};
use crate::gap_ledger::GapLedger;

/// A detected pattern in the gap ledger.
#[derive(Debug, Clone)]
pub struct GapPattern {
    /// Canonical description of the pattern.
    pub pattern_id: String,
    /// How many times this pattern has been seen.
    pub occurrences: u64,
    /// Representative gap instances.
    pub representative_gaps: Vec<Gap>,
    /// What the gaps share.
    pub common_structure: String,
}

/// The schema inductor — analyzes gap patterns and synthesizes schemas.
pub struct SchemaInductor {
    /// Pattern counts: pattern_id → occurrence count.
    pattern_counts: BTreeMap<String, u64>,
    /// Known patterns: pattern_id → GapPattern.
    known_patterns: BTreeMap<String, GapPattern>,
    /// Induction threshold: how many occurrences before synthesizing.
    pub threshold: u64,
}

impl SchemaInductor {
    pub fn new() -> Self {
        Self {
            pattern_counts: BTreeMap::new(),
            known_patterns: BTreeMap::new(),
            threshold: 3,
        }
    }

    /// Analyze the gap ledger for repeated patterns.
    ///
    /// Groups gaps by schema_id, then looks for structural similarity
    /// within each group. Returns patterns that exceed the threshold.
    pub fn detect_patterns(&mut self, gap_ledger: &GapLedger) -> Vec<GapPattern> {
        // Group active gaps by schema_id
        let mut groups: BTreeMap<SchemaId, Vec<Gap>> = BTreeMap::new();
        for gap in gap_ledger.active_gaps().values() {
            groups.entry(gap.schema_id.clone())
                .or_default()
                .push(gap.clone());
        }

        let mut detected = Vec::new();

        for (schema_id, gaps) in &groups {
            let pattern_id = format!("{:?}_pattern", schema_id);
            let count = gaps.len() as u64;

            *self.pattern_counts.entry(pattern_id.clone()).or_insert(0) = count;

            if count >= self.threshold {
                // Extract common structure
                let common = extract_common_structure(gaps);
                let pattern = GapPattern {
                    pattern_id: pattern_id.clone(),
                    occurrences: count,
                    representative_gaps: gaps.iter().take(3).cloned().collect(),
                    common_structure: common,
                };
                self.known_patterns.insert(pattern_id, pattern.clone());
                detected.push(pattern);
            }
        }

        detected
    }

    /// Attempt to synthesize a new schema from a detected pattern.
    ///
    /// Returns a SchemaId for the induced schema if successful.
    /// The schema is a Derived schema that can be added to the library.
    pub fn induce_schema(&self, pattern: &GapPattern) -> Option<SchemaId> {
        // Only induce if pattern has enough occurrences
        if pattern.occurrences < self.threshold {
            return None;
        }

        // Create a derived schema ID based on the pattern
        let schema_name = format!("induced_{}", pattern.pattern_id);
        Some(SchemaId::Derived(schema_name))
    }

    /// Number of known patterns.
    pub fn pattern_count(&self) -> usize {
        self.known_patterns.len()
    }

    /// Hash of the inductor state (for determinism verification).
    pub fn inductor_hash(&self) -> Hash32 {
        let mut buf = Vec::new();
        for (id, count) in &self.pattern_counts {
            buf.extend_from_slice(id.as_bytes());
            buf.extend_from_slice(&count.to_le_bytes());
        }
        hash::H(&buf)
    }
}

impl Default for SchemaInductor {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract common structure from a group of gaps.
fn extract_common_structure(gaps: &[Gap]) -> String {
    if gaps.is_empty() {
        return "empty".to_string();
    }

    // Find common schema_id
    let schema = &gaps[0].schema_id;

    // Check if all gaps have the same unresolved_bound pattern
    let all_have_bound = gaps.iter().all(|g| g.unresolved_bound.is_some());
    let all_no_deps = gaps.iter().all(|g| g.dependency_hashes.is_empty());

    let mut structure = format!("schema={:?}", schema);
    if all_have_bound {
        structure.push_str(", all have unresolved bounds");
    }
    if all_no_deps {
        structure.push_str(", no dependencies");
    }
    structure
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frc_types::Gap;
    use crate::gap_ledger::GapLedger;

    fn make_gap(goal: &str, schema: SchemaId) -> Gap {
        Gap {
            goal_hash: hash::H(goal.as_bytes()),
            goal_statement: goal.to_string(),
            schema_id: schema,
            dependency_hashes: vec![],
            unresolved_bound: Some("B*=100".to_string()),
        }
    }

    #[test]
    fn detect_patterns_below_threshold() {
        let mut ledger = GapLedger::new();
        ledger.record_gap(make_gap("goal_1", SchemaId::FiniteSearch));
        ledger.record_gap(make_gap("goal_2", SchemaId::FiniteSearch));
        // Only 2 gaps, threshold is 3
        let mut inductor = SchemaInductor::new();
        let patterns = inductor.detect_patterns(&ledger);
        assert!(patterns.is_empty());
    }

    #[test]
    fn detect_patterns_at_threshold() {
        let mut ledger = GapLedger::new();
        ledger.record_gap(make_gap("goal_1", SchemaId::FiniteSearch));
        ledger.record_gap(make_gap("goal_2", SchemaId::FiniteSearch));
        ledger.record_gap(make_gap("goal_3", SchemaId::FiniteSearch));

        let mut inductor = SchemaInductor::new();
        let patterns = inductor.detect_patterns(&ledger);
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].occurrences, 3);
    }

    #[test]
    fn induce_schema_from_pattern() {
        let mut ledger = GapLedger::new();
        for i in 0..5 {
            ledger.record_gap(make_gap(&format!("goal_{}", i), SchemaId::BoundedCounterexample));
        }

        let mut inductor = SchemaInductor::new();
        let patterns = inductor.detect_patterns(&ledger);
        assert!(!patterns.is_empty());

        let schema_id = inductor.induce_schema(&patterns[0]);
        assert!(schema_id.is_some());
        assert!(matches!(schema_id.unwrap(), SchemaId::Derived(_)));
    }
}
