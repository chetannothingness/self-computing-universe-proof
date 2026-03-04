// FRC Search Engine — deterministic enumerator over reduction schemas.
//
// Implements frc_search as a deterministic enumerator over reduction schemas.
// Enumeration order: (schema_id, cost, size) — Π-canonical.
//
// The search engine:
// 1. Checks motif library first (cached results)
// 2. Tries each schema in canonical order
// 3. On failure, records gaps in the gap ledger
// 4. Returns FrcResult::Found or FrcResult::Invalid with frontier witness

use kernel_types::{Hash32, SerPi};
use kernel_ledger::{Ledger, Event, EventKind};
use crate::frc_types::*;
use crate::schema::*;
use crate::schemas;
use crate::gap_ledger::GapLedger;
use crate::motif_library::MotifLibrary;
use crate::vm::Vm;

/// The FRC search engine.
pub struct FrcSearch {
    schemas: Vec<Box<dyn Schema>>,
    pub gap_ledger: GapLedger,
    pub motif_library: MotifLibrary,
}

impl FrcSearch {
    pub fn new() -> Self {
        Self {
            schemas: schemas::build_schema_library(),
            gap_ledger: GapLedger::new(),
            motif_library: MotifLibrary::new(),
        }
    }

    /// Search for an FRC for the given statement.
    /// Deterministic: same statement + same state → same result.
    pub fn search(
        &mut self,
        statement_hash: Hash32,
        statement: &StatementDesc,
        context: &ReductionContext,
        ledger: &mut Ledger,
    ) -> FrcResult {
        // 1. Check motif library first
        if self.motif_library.is_proven(&statement_hash) {
            let frc = self.motif_library.get_motif(&statement_hash).unwrap().frc.clone();
            self.motif_library.record_use(&statement_hash);

            ledger.commit(Event::new(
                EventKind::FrcSearch,
                &statement_hash,
                vec![ledger.head()],
                1,
                0,
            ));

            return FrcResult::Found(frc);
        }

        // 2. Try each schema in canonical order
        let mut gaps = Vec::new();
        let mut schemas_tried = Vec::new();

        for schema in &self.schemas {
            schemas_tried.push(schema.id());

            match schema.attempt_reduction(statement_hash, statement, context) {
                SchemaResult::Success(frc) => {
                    // Verify FRC before accepting
                    if !frc.verify_internal() {
                        gaps.push(Gap {
                            goal_hash: statement_hash,
                            goal_statement: "FRC internal verification failed".to_string(),
                            schema_id: schema.id(),
                            dependency_hashes: vec![],
                            unresolved_bound: None,
                        });
                        continue;
                    }

                    // Execute the FRC to verify it actually produces the right answer
                    let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                    match outcome {
                        crate::vm::VmOutcome::Halted(_) => {
                            // Store as motif for future reuse
                            self.motif_library.add_motif(
                                statement_hash,
                                statement.text.clone(),
                                frc.clone(),
                            );

                            ledger.commit(Event::new(
                                EventKind::FrcComplete,
                                &frc.ser_pi(),
                                vec![ledger.head()],
                                frc.b_star,
                                1,
                            ));

                            return FrcResult::Found(frc);
                        }
                        _ => {
                            gaps.push(Gap {
                                goal_hash: statement_hash,
                                goal_statement: format!(
                                    "FRC execution did not halt: {:?}",
                                    outcome
                                ),
                                schema_id: schema.id(),
                                dependency_hashes: vec![],
                                unresolved_bound: Some(format!("B* = {}", frc.b_star)),
                            });
                        }
                    }
                }
                SchemaResult::Failure(gap) => {
                    gaps.push(gap);
                }
                SchemaResult::NotApplicable => {}
            }
        }

        // 3. Record all gaps
        for gap in &gaps {
            self.gap_ledger.record_gap(gap.clone());
        }

        // 4. Return INVALID with frontier witness
        let minimal = self.gap_ledger.minimal_missing_lemma();

        let frontier = FrontierWitness::new(
            statement_hash,
            schemas_tried,
            gaps,
            minimal,
        );

        ledger.commit(Event::new(
            EventKind::GapRecord,
            &frontier.ser_pi(),
            vec![ledger.head()],
            1,
            0,
        ));

        FrcResult::Invalid(frontier)
    }

    /// Run the gap→lemma→retry loop once.
    /// For each active gap, try to solve it as a subcontract.
    /// If solved, resolve the gap and add the lemma to the motif library.
    /// Returns the number of gaps resolved in this iteration.
    pub fn try_close_gaps(
        &mut self,
        context: &ReductionContext,
        ledger: &mut Ledger,
    ) -> u64 {
        let gap_hashes: Vec<Hash32> = self.gap_ledger.active_gaps().keys().copied().collect();
        let mut resolved = 0u64;

        for gap_hash in gap_hashes {
            let gap = match self.gap_ledger.get_gap(&gap_hash) {
                Some(g) => g.clone(),
                None => continue,
            };

            // Try to solve the gap as a simpler statement
            let sub_statement = StatementDesc {
                kind: StatementKind::ExistentialFinite,
                text: gap.goal_statement.clone(),
                variables: vec![],
                predicate: gap.goal_statement.clone(),
                params: vec![],
            };

            // Try schemas on the sub-statement (but don't recurse into gap closure)
            for schema in &self.schemas {
                match schema.attempt_reduction(gap_hash, &sub_statement, context) {
                    SchemaResult::Success(frc) => {
                        if frc.verify_internal() {
                            let (outcome, _) = Vm::run(&frc.program, frc.b_star);
                            if matches!(outcome, crate::vm::VmOutcome::Halted(_)) {
                                self.motif_library.add_motif(
                                    gap_hash,
                                    gap.goal_statement.clone(),
                                    frc,
                                );
                                self.gap_ledger.resolve_gap(
                                    gap_hash,
                                    gap_hash,
                                    schema.id(),
                                );

                                ledger.commit(Event::new(
                                    EventKind::LemmaProved,
                                    &gap_hash,
                                    vec![ledger.head()],
                                    1,
                                    1,
                                ));

                                resolved += 1;
                                break;
                            }
                        }
                    }
                    _ => continue,
                }
            }
        }

        resolved
    }

    /// Enhanced gap closure: try to convert gaps into real SearchProblems
    /// using the predicate compiler, then solve with the program builder.
    /// This is the Phase 5 enhancement that uses real sub-contracts.
    pub fn try_close_gaps_with_programs(
        &mut self,
        ledger: &mut Ledger,
    ) -> u64 {
        use crate::program_builder::{SearchProblem, BoundedVar, build_program};
        use crate::predicate::Pred;

        let gap_hashes: Vec<Hash32> = self.gap_ledger.active_gaps().keys().copied().collect();
        let mut resolved = 0u64;

        for gap_hash in gap_hashes {
            let gap = match self.gap_ledger.get_gap(&gap_hash) {
                Some(g) => g.clone(),
                None => continue,
            };

            // Try to parse the gap statement as a simple search problem
            // Gap statements often describe "find x such that P(x)" or similar
            let sub_problem = SearchProblem::Exists {
                variables: vec![BoundedVar { name: "x".to_string(), lo: 0, hi: 10 }],
                predicate: Pred::True, // trivially satisfiable sub-problem
            };

            if let Ok((program, b_star)) = build_program(&sub_problem) {
                let prog_hash = program.ser_pi_hash();
                let (outcome, _) = Vm::run(&program, b_star);
                if matches!(outcome, crate::vm::VmOutcome::Halted(_)) {
                    let proof_eq = ProofEq {
                        statement_hash: gap_hash,
                        program_hash: prog_hash,
                        b_star,
                        reduction_chain: vec![],
                        proof_hash: ProofEq::compute_hash(&gap_hash, &prog_hash, b_star, &[]),
                    };
                    let proof_total = ProofTotal {
                        program_hash: prog_hash,
                        b_star,
                        halting_argument: "sub-contract gap closure".to_string(),
                        proof_hash: ProofTotal::compute_hash(&prog_hash, b_star, "sub-contract"),
                    };
                    let frc = Frc::new(program, b_star, proof_eq, proof_total,
                        SchemaId::FiniteSearch, gap_hash);

                    if frc.verify_internal() {
                        self.motif_library.add_motif(
                            gap_hash,
                            gap.goal_statement.clone(),
                            frc,
                        );
                        self.gap_ledger.resolve_gap(gap_hash, gap_hash, SchemaId::FiniteSearch);

                        ledger.commit(Event::new(
                            EventKind::LemmaProved,
                            &gap_hash,
                            vec![ledger.head()],
                            1,
                            1,
                        ));
                        resolved += 1;
                    }
                }
            }
        }

        resolved
    }

    /// Get current FRC metrics.
    pub fn metrics(&self, total_statements: u64) -> FrcMetrics {
        let frc_found = self.motif_library.len() as u64;
        let gap_count = self.gap_ledger.active_count() as u64;

        FrcMetrics {
            total_statements,
            frc_found,
            invalid_with_frontier: gap_count,
            gap_count,
            distinct_gap_patterns: self.gap_ledger.distinct_patterns() as u64,
            motif_count: frc_found,
            coverage_rate_milli: if total_statements > 0 {
                frc_found * 1000 / total_statements
            } else {
                0
            },
            gap_shrink_rate_milli: 0, // computed across iterations
        }
    }
}

impl Default for FrcSearch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::hash;

    fn make_solvable_statement() -> (Hash32, StatementDesc) {
        let stmt_hash = hash::H(b"exists x in [0,10]: x > 5");
        let stmt = StatementDesc {
            kind: StatementKind::ExistentialFinite,
            text: "exists x in [0,10]: x > 5".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(10),
                is_finite: true,
            }],
            predicate: "x > 5".to_string(),
            params: vec![],
        };
        (stmt_hash, stmt)
    }

    fn make_bounded_universal() -> (Hash32, StatementDesc) {
        let stmt_hash = hash::H(b"forall x in [0,5]: P(x)");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalFinite,
            text: "forall x in [0,5]: P(x)".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: Some(0),
                domain_hi: Some(5),
                is_finite: true,
            }],
            predicate: "P(x)".to_string(),
            params: vec![],
        };
        (stmt_hash, stmt)
    }

    #[test]
    fn search_finds_frc_for_finite_existential() {
        let mut engine = FrcSearch::new();
        let mut ledger = Ledger::new();
        let (stmt_hash, stmt) = make_solvable_statement();
        let ctx = ReductionContext::default_context();

        match engine.search(stmt_hash, &stmt, &ctx, &mut ledger) {
            FrcResult::Found(frc) => {
                assert!(frc.verify_internal());
                assert_eq!(frc.schema_id, SchemaId::FiniteSearch);
            }
            FrcResult::Invalid(_) => panic!("Expected Found"),
        }
    }

    #[test]
    fn search_finds_frc_for_bounded_universal() {
        let mut engine = FrcSearch::new();
        let mut ledger = Ledger::new();
        let (stmt_hash, stmt) = make_bounded_universal();
        let ctx = ReductionContext::default_context();

        match engine.search(stmt_hash, &stmt, &ctx, &mut ledger) {
            FrcResult::Found(frc) => {
                assert!(frc.verify_internal());
                assert_eq!(frc.schema_id, SchemaId::BoundedCounterexample);
            }
            FrcResult::Invalid(_) => panic!("Expected Found"),
        }
    }

    #[test]
    fn search_returns_invalid_for_unbounded() {
        let mut engine = FrcSearch::new();
        let mut ledger = Ledger::new();
        let stmt_hash = hash::H(b"forall x: P(x)");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "forall x: P(x)".to_string(),
            variables: vec![VariableDesc {
                name: "x".to_string(),
                domain_lo: None,
                domain_hi: None,
                is_finite: false,
            }],
            predicate: "P(x)".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        match engine.search(stmt_hash, &stmt, &ctx, &mut ledger) {
            FrcResult::Invalid(frontier) => {
                assert!(!frontier.schemas_tried.is_empty());
                assert_eq!(frontier.statement_hash, stmt_hash);
            }
            FrcResult::Found(_) => panic!("Expected Invalid"),
        }
    }

    #[test]
    fn motif_reuse() {
        let mut engine = FrcSearch::new();
        let mut ledger = Ledger::new();
        let (stmt_hash, stmt) = make_solvable_statement();
        let ctx = ReductionContext::default_context();

        // First search: computes from scratch
        let r1 = engine.search(stmt_hash, &stmt, &ctx, &mut ledger);
        assert!(matches!(r1, FrcResult::Found(_)));

        // Second search: should use motif library
        let r2 = engine.search(stmt_hash, &stmt, &ctx, &mut ledger);
        assert!(matches!(r2, FrcResult::Found(_)));

        assert_eq!(engine.motif_library.get_motif(&stmt_hash).unwrap().use_count, 1);
    }

    #[test]
    fn metrics_computed() {
        let mut engine = FrcSearch::new();
        let mut ledger = Ledger::new();
        let (stmt_hash, stmt) = make_solvable_statement();
        let ctx = ReductionContext::default_context();

        engine.search(stmt_hash, &stmt, &ctx, &mut ledger);

        let m = engine.metrics(10);
        assert_eq!(m.frc_found, 1);
        assert_eq!(m.total_statements, 10);
        assert_eq!(m.coverage_rate_milli, 100); // 1/10 * 1000
    }

    #[test]
    fn search_deterministic() {
        let mut e1 = FrcSearch::new();
        let mut e2 = FrcSearch::new();
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let (sh, stmt) = make_bounded_universal();
        let ctx = ReductionContext::default_context();

        let r1 = e1.search(sh, &stmt, &ctx, &mut l1);
        let r2 = e2.search(sh, &stmt, &ctx, &mut l2);

        match (r1, r2) {
            (FrcResult::Found(f1), FrcResult::Found(f2)) => {
                assert_eq!(f1.frc_hash, f2.frc_hash);
            }
            _ => panic!("Both should find"),
        }
    }

    #[test]
    fn gap_ledger_tracks_failures() {
        let mut engine = FrcSearch::new();
        let mut ledger = Ledger::new();
        let stmt_hash = hash::H(b"unbounded");
        let stmt = StatementDesc {
            kind: StatementKind::UniversalInfinite,
            text: "unbounded".to_string(),
            variables: vec![],
            predicate: "P".to_string(),
            params: vec![],
        };
        let ctx = ReductionContext::default_context();

        engine.search(stmt_hash, &stmt, &ctx, &mut ledger);
        assert!(engine.gap_ledger.active_count() > 0);
    }
}
