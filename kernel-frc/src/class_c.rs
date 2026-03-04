// CLASS_C Definition and Coverage Metrics
//
// CLASS_C is the formal class of statements claimed decidable by the kernel.
// It is defined by:
//   - Grammar: what kinds of statements (BoolCnf, ArithFind, Table, etc.)
//   - Schemas: available reduction strategies
//   - Primitives: VM instruction set and cost model
//   - Motifs: proven lemmas available for reuse
//
// Coverage metrics track:
//   - FRC coverage rate: fraction of CLASS_C for which FRC exists
//   - Gap shrink rate: distinct unresolved patterns decreasing over time

use kernel_types::{Hash32, hash};
use serde::{Serialize, Deserialize};
use crate::frc_types::{SchemaId, AllowedPrimitives, FrcMetrics};
use crate::gap_ledger::GapLedger;
use crate::motif_library::MotifLibrary;
use crate::schema_induction::SchemaInductor;

/// Statement grammar — what kinds of statements CLASS_C covers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StatementGrammar {
    /// BoolCnf up to max_vars variables.
    BoolCnf { max_vars: usize },
    /// Polynomial ArithFind up to degree, over integer domain [lo, hi].
    ArithFind { max_degree: usize, max_domain: i64 },
    /// Table lookup up to max_entries.
    Table { max_entries: usize },
    /// Composition of grammars.
    Composite(Vec<StatementGrammar>),
}

/// Enhanced CLASS_C definition with structured grammar.
#[derive(Debug, Clone)]
pub struct ClassCDefinition {
    /// Structured grammar definition.
    pub grammar: Vec<StatementGrammar>,
    /// Available schemas (including induced).
    pub schemas: Vec<SchemaId>,
    /// VM instruction set and cost model.
    pub primitives: AllowedPrimitives,
    /// Number of proven motifs.
    pub motif_count: usize,
    /// Induced schema count.
    pub induced_schema_count: usize,
    /// Identity hash of the class.
    pub class_hash: Hash32,
}

impl ClassCDefinition {
    /// Build the CLASS_C definition from current kernel state.
    pub fn build(
        schemas: &[SchemaId],
        motif_library: &MotifLibrary,
        inductor: &SchemaInductor,
    ) -> Self {
        let grammar = vec![
            StatementGrammar::BoolCnf { max_vars: 20 },
            StatementGrammar::ArithFind { max_degree: 10, max_domain: 1_000_000 },
            StatementGrammar::Table { max_entries: 10_000 },
        ];

        let primitives = AllowedPrimitives {
            max_vm_steps: 100_000_000,
            max_memory_slots: 4096,
            cost_model: "unit".to_string(),
        };

        let all_schemas = schemas.to_vec();
        // Include any induced schemas
        let induced_count = inductor.pattern_count();

        let mut hash_buf = Vec::new();
        for g in &grammar {
            hash_buf.extend_from_slice(format!("{:?}", g).as_bytes());
        }
        for s in &all_schemas {
            hash_buf.extend_from_slice(format!("{:?}", s).as_bytes());
        }
        hash_buf.extend_from_slice(&motif_library.library_hash());
        let class_hash = hash::H(&hash_buf);

        Self {
            grammar,
            schemas: all_schemas,
            primitives,
            motif_count: motif_library.len(),
            induced_schema_count: induced_count,
            class_hash,
        }
    }

    /// Format as human-readable string.
    pub fn display(&self) -> String {
        let mut s = String::new();
        s.push_str("CLASS_C Definition:\n");
        s.push_str("  Grammar:\n");
        for g in &self.grammar {
            s.push_str(&format!("    {:?}\n", g));
        }
        s.push_str(&format!("  Schemas: {} base + {} induced\n",
            self.schemas.len(), self.induced_schema_count));
        s.push_str(&format!("  Motifs: {}\n", self.motif_count));
        s.push_str(&format!("  Class hash: {}\n", hash::hex(&self.class_hash)));
        s
    }
}

/// Coverage report — computed from FRC search results.
#[derive(Debug, Clone)]
pub struct CoverageReport {
    /// Total contracts in suite.
    pub total_contracts: u64,
    /// Contracts with FRC found.
    pub frc_found: u64,
    /// Contracts with INVALID frontier (inadmissible).
    pub invalid_with_frontier: u64,
    /// FRC coverage rate (frc_found * 1000 / total) in per-mille.
    pub coverage_rate_milli: u64,
    /// Active unresolved gaps.
    pub gap_count: u64,
    /// Gap shrink rate (decrease in gaps per iteration, per-mille).
    pub gap_shrink_rate_milli: u64,
    /// Proven lemma count.
    pub motif_count: u64,
    /// Schema count (base + induced).
    pub schema_count: u64,
}

impl CoverageReport {
    /// Compute coverage from FRC metrics and gap ledger.
    pub fn compute(
        frc_found: u64,
        invalid_count: u64,
        gap_ledger: &GapLedger,
        motif_library: &MotifLibrary,
        schema_count: u64,
        previous_gap_count: Option<u64>,
    ) -> Self {
        let total = frc_found + invalid_count;
        let coverage_rate = if total > 0 {
            frc_found * 1000 / total
        } else {
            0
        };

        let gap_count = gap_ledger.active_count() as u64;
        let gap_shrink = match previous_gap_count {
            Some(prev) if prev > 0 => {
                let reduction = prev.saturating_sub(gap_count);
                reduction * 1000 / prev
            }
            _ => 0,
        };

        Self {
            total_contracts: total,
            frc_found,
            invalid_with_frontier: invalid_count,
            coverage_rate_milli: coverage_rate,
            gap_count,
            gap_shrink_rate_milli: gap_shrink,
            motif_count: motif_library.len() as u64,
            schema_count,
        }
    }

    /// Convert to FrcMetrics (the existing type).
    pub fn to_frc_metrics(&self) -> FrcMetrics {
        FrcMetrics {
            total_statements: self.total_contracts,
            frc_found: self.frc_found,
            invalid_with_frontier: self.invalid_with_frontier,
            gap_count: self.gap_count,
            distinct_gap_patterns: 0, // filled by caller
            motif_count: self.motif_count,
            coverage_rate_milli: self.coverage_rate_milli,
            gap_shrink_rate_milli: self.gap_shrink_rate_milli,
        }
    }

    /// Format as human-readable string.
    pub fn display(&self) -> String {
        format!(
            "Coverage Report:\n  Total: {}\n  FRC found: {}\n  INVALID: {}\n  \
             Coverage: {:.1}%\n  Gaps: {}\n  Gap shrink: {:.1}%\n  Motifs: {}\n  Schemas: {}",
            self.total_contracts,
            self.frc_found,
            self.invalid_with_frontier,
            self.coverage_rate_milli as f64 / 10.0,
            self.gap_count,
            self.gap_shrink_rate_milli as f64 / 10.0,
            self.motif_count,
            self.schema_count,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frc_types::SchemaId;

    #[test]
    fn class_c_covers_all_types() {
        let schemas = vec![
            SchemaId::BoundedCounterexample,
            SchemaId::FiniteSearch,
            SchemaId::EffectiveCompactness,
            SchemaId::ProofMining,
            SchemaId::AlgebraicDecision,
            SchemaId::CertifiedNumerics,
        ];
        let motif_lib = MotifLibrary::new();
        let inductor = SchemaInductor::new();
        let class_c = ClassCDefinition::build(&schemas, &motif_lib, &inductor);

        // Covers BoolCnf, ArithFind, Table
        assert_eq!(class_c.grammar.len(), 3);
        assert_eq!(class_c.schemas.len(), 6);
    }

    #[test]
    fn coverage_rate_100_percent_solvable() {
        let gap_ledger = GapLedger::new();
        let motif_lib = MotifLibrary::new();
        let report = CoverageReport::compute(10, 0, &gap_ledger, &motif_lib, 6, None);
        assert_eq!(report.coverage_rate_milli, 1000); // 100%
    }

    #[test]
    fn gap_shrink_rate_positive() {
        let gap_ledger = GapLedger::new(); // 0 active gaps
        let motif_lib = MotifLibrary::new();
        let report = CoverageReport::compute(8, 2, &gap_ledger, &motif_lib, 6, Some(5));
        // Previous had 5 gaps, now 0 → shrink = 5/5 = 100%
        assert_eq!(report.gap_shrink_rate_milli, 1000);
    }
}
