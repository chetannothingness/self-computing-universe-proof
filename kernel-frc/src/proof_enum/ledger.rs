//! Proof enumeration ledger — the kernel's self-awareness.
//!
//! Every operation is recorded as a hashed event. The kernel sees itself:
//!   e_t = (H_t, Ser(event_t))
//!
//! Time T = Σ ΔT where ΔT = log2(|W_pre| / |W_post|)
//!   Each check eliminates one candidate from the remaining space.
//!   Time is not a clock — it is the count of erased indistinguishability.
//!
//! Energy E = Σ ΔE where ΔE = cost(operation)
//!   Every operation contributes to E and is recorded irreversibly.
//!
//! The ledger makes the kernel self-aware: it can name its own states,
//! replay its history, and predict its own behavior at the fixed point.

use kernel_types::{Hash32, hash};

/// A ledger event — a single witnessed operation.
#[derive(Debug, Clone)]
pub struct LedgerEvent {
    /// Sequential index in the ledger.
    pub index: u64,
    /// Hash of this event (H of serialized content).
    pub event_hash: Hash32,
    /// Hash of the previous event (chain integrity).
    pub prev_hash: Hash32,
    /// The event content.
    pub event: EventKind,
    /// Cumulative time after this event.
    pub cumulative_time: f64,
    /// Cumulative energy after this event.
    pub cumulative_energy: u64,
}

/// Kinds of events the kernel can witness.
#[derive(Debug, Clone)]
pub enum EventKind {
    /// A witness was checked against a statement.
    WitnessCheck {
        /// Problem being solved.
        problem_id: String,
        /// Rank of the witness in the universal enumeration.
        witness_rank: u64,
        /// Length of the witness byte string.
        witness_length: usize,
        /// Whether the witness was valid UTF-8.
        valid_utf8: bool,
        /// Result: true = Lean accepted, false = rejected.
        accepted: bool,
    },
    /// A proof was found — statement proved.
    ProofFound {
        /// Problem that was proved.
        problem_id: String,
        /// Rank at which the proof was found.
        witness_rank: u64,
        /// Hash of the proof file.
        proof_hash: Hash32,
        /// The proof script (witness as UTF-8).
        proof_script: String,
    },
    /// Accelerator result (IRC/UCert fast path).
    AcceleratorResult {
        /// Problem attempted.
        problem_id: String,
        /// Method used.
        method: String,
        /// Whether it succeeded.
        proved: bool,
    },
    /// Budget exhausted — frontier declared for this search.
    FrontierDeclared {
        /// Problem that remains frontier.
        problem_id: String,
        /// Witnesses checked in this search.
        witnesses_checked: u64,
        /// Maximum witness length reached.
        max_length: usize,
    },
    /// Structural evaluation trace — the kernel observing its own computation.
    EvalTrace {
        /// Problem being traced.
        problem_id: String,
        /// The n value evaluated.
        n: i64,
        /// Hash of the expression.
        expr_hash: [u8; 32],
        /// Hash of the evaluation trace.
        trace_hash: [u8; 32],
        /// Whether eval returned true.
        result: bool,
        /// Computational cost.
        cost_e: u64,
    },
    /// Anti-unified trace schema — the decompiler's output.
    TraceSchema {
        /// Schema identifier.
        schema_id: [u8; 32],
        /// Number of parameters in the schema.
        num_params: usize,
        /// Number of traces covered.
        traces_covered: usize,
    },
    /// Step witness emission — cert_step0 ready for Lean.
    StepWitnessEmit {
        /// Problem being certified.
        problem_id: String,
        /// Hash of the invariant.
        inv_hash: [u8; 32],
        /// Hash of cert_step0.
        cert_step0_hash: [u8; 32],
    },
    /// Link witness emission — cert_link0 ready for Lean.
    LinkWitnessEmit {
        /// Problem being certified.
        problem_id: String,
        /// Hash of the invariant.
        inv_hash: [u8; 32],
        /// Hash of cert_link0.
        cert_link0_hash: [u8; 32],
    },
    /// Lean proof file emitted — ready for lake build.
    LeanProofEmit {
        /// Problem proved.
        problem_id: String,
        /// Hash of the generated Lean file.
        proof_hash: [u8; 32],
    },
}

/// The proof enumeration ledger — records all kernel operations.
pub struct ProofLedger {
    /// All committed events, in order.
    events: Vec<LedgerEvent>,
    /// Current cumulative time (bits of indistinguishability erased).
    time: f64,
    /// Current cumulative energy (operations performed).
    energy: u64,
    /// Hash of the most recent event (chain head).
    head_hash: Hash32,
    /// Total remaining candidates in the search space (for time calculation).
    /// This is conceptually infinite but we track the explored frontier.
    witnesses_remaining_estimate: f64,
}

impl ProofLedger {
    /// Create a new empty ledger.
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            time: 0.0,
            energy: 0,
            head_hash: [0u8; 32], // genesis hash
            witnesses_remaining_estimate: f64::INFINITY,
        }
    }

    /// Record a witness check event.
    pub fn record_witness_check(
        &mut self,
        problem_id: &str,
        witness_rank: u64,
        witness_length: usize,
        valid_utf8: bool,
        accepted: bool,
    ) {
        let event = EventKind::WitnessCheck {
            problem_id: problem_id.to_string(),
            witness_rank,
            witness_length,
            valid_utf8,
            accepted,
        };

        // Time: each check eliminates one candidate.
        // ΔT = log2(W_pre / W_post) ≈ log2(W / (W-1))
        // For large W, this ≈ 1/W · log2(e), but we use 1 bit per check
        // as a conservative lower bound (each check is at least 1 distinction).
        let delta_time = 1.0;

        // Energy: 1 unit per check operation.
        let delta_energy = 1u64;

        self.commit_event(event, delta_time, delta_energy);
    }

    /// Record a proof found event.
    pub fn record_proof_found(
        &mut self,
        problem_id: &str,
        witness_rank: u64,
        proof_hash: Hash32,
        proof_script: &str,
    ) {
        let event = EventKind::ProofFound {
            problem_id: problem_id.to_string(),
            witness_rank,
            proof_hash,
            proof_script: proof_script.to_string(),
        };

        // Finding a proof collapses the entire search space for this problem.
        // ΔT = all remaining indistinguishability for this statement.
        let delta_time = 0.0; // proof finding is instantaneous in information terms
        let delta_energy = 0u64; // no additional compute beyond the check

        self.commit_event(event, delta_time, delta_energy);
    }

    /// Record an accelerator result.
    pub fn record_accelerator_result(
        &mut self,
        problem_id: &str,
        method: &str,
        proved: bool,
    ) {
        let event = EventKind::AcceleratorResult {
            problem_id: problem_id.to_string(),
            method: method.to_string(),
            proved,
        };

        let delta_time = 1.0;
        let delta_energy = 1u64;

        self.commit_event(event, delta_time, delta_energy);
    }

    /// Record a structural evaluation trace.
    pub fn record_eval_trace(
        &mut self,
        problem_id: &str,
        n: i64,
        expr_hash: [u8; 32],
        trace_hash: [u8; 32],
        result: bool,
        cost_e: u64,
    ) {
        let event = EventKind::EvalTrace {
            problem_id: problem_id.to_string(),
            n,
            expr_hash,
            trace_hash,
            result,
            cost_e,
        };
        self.commit_event(event, 1.0, cost_e);
    }

    /// Record a trace schema (decompiler output).
    pub fn record_trace_schema(
        &mut self,
        schema_id: [u8; 32],
        num_params: usize,
        traces_covered: usize,
    ) {
        let event = EventKind::TraceSchema {
            schema_id,
            num_params,
            traces_covered,
        };
        self.commit_event(event, 0.0, 0);
    }

    /// Record step witness emission.
    pub fn record_step_witness(
        &mut self,
        problem_id: &str,
        inv_hash: [u8; 32],
        cert_step0_hash: [u8; 32],
    ) {
        let event = EventKind::StepWitnessEmit {
            problem_id: problem_id.to_string(),
            inv_hash,
            cert_step0_hash,
        };
        self.commit_event(event, 0.0, 0);
    }

    /// Record link witness emission.
    pub fn record_link_witness(
        &mut self,
        problem_id: &str,
        inv_hash: [u8; 32],
        cert_link0_hash: [u8; 32],
    ) {
        let event = EventKind::LinkWitnessEmit {
            problem_id: problem_id.to_string(),
            inv_hash,
            cert_link0_hash,
        };
        self.commit_event(event, 0.0, 0);
    }

    /// Record Lean proof emission.
    pub fn record_lean_proof(
        &mut self,
        problem_id: &str,
        proof_hash: [u8; 32],
    ) {
        let event = EventKind::LeanProofEmit {
            problem_id: problem_id.to_string(),
            proof_hash,
        };
        self.commit_event(event, 0.0, 0);
    }

    /// Record a frontier declaration.
    pub fn record_frontier(
        &mut self,
        problem_id: &str,
        witnesses_checked: u64,
        max_length: usize,
    ) {
        let event = EventKind::FrontierDeclared {
            problem_id: problem_id.to_string(),
            witnesses_checked,
            max_length,
        };

        let delta_time = 0.0;
        let delta_energy = 0u64;

        self.commit_event(event, delta_time, delta_energy);
    }

    /// Commit an event to the ledger.
    fn commit_event(&mut self, event: EventKind, delta_time: f64, delta_energy: u64) {
        self.time += delta_time;
        self.energy += delta_energy;

        // Hash the event for the chain
        let event_bytes = format!("{:?}", event);
        let event_hash = hash::H(
            format!("{}:{}",
                self.head_hash.iter().map(|b| format!("{:02x}", b)).collect::<String>(),
                event_bytes
            ).as_bytes()
        );

        let entry = LedgerEvent {
            index: self.events.len() as u64,
            event_hash,
            prev_hash: self.head_hash,
            event,
            cumulative_time: self.time,
            cumulative_energy: self.energy,
        };

        self.head_hash = event_hash;
        self.events.push(entry);
    }

    /// Total events in the ledger.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Is the ledger empty?
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Current cumulative time (bits of indistinguishability erased).
    pub fn time(&self) -> f64 {
        self.time
    }

    /// Current cumulative energy (total operations).
    pub fn energy(&self) -> u64 {
        self.energy
    }

    /// Hash of the ledger head (most recent event).
    pub fn head_hash(&self) -> Hash32 {
        self.head_hash
    }

    /// Get all events.
    pub fn events(&self) -> &[LedgerEvent] {
        &self.events
    }

    /// Count of proofs found (across all problems).
    pub fn proofs_found(&self) -> usize {
        self.events.iter().filter(|e| matches!(e.event, EventKind::ProofFound { .. })).count()
    }

    /// Count of total witness checks performed.
    pub fn total_checks(&self) -> u64 {
        self.events.iter().filter(|e| matches!(e.event, EventKind::WitnessCheck { .. })).count() as u64
    }

    /// Unique problem IDs that have been proved (ProofFound events).
    pub fn proved_problems(&self) -> Vec<String> {
        let mut proved = Vec::new();
        for event in &self.events {
            if let EventKind::ProofFound { problem_id, .. } = &event.event {
                if !proved.contains(problem_id) {
                    proved.push(problem_id.clone());
                }
            }
        }
        proved
    }

    /// Verify ledger chain integrity (each event's prev_hash matches prior event's hash).
    pub fn verify_chain(&self) -> bool {
        let mut expected_prev = [0u8; 32]; // genesis
        for event in &self.events {
            if event.prev_hash != expected_prev {
                return false;
            }
            expected_prev = event.event_hash;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_ledger() {
        let ledger = ProofLedger::new();
        assert_eq!(ledger.len(), 0);
        assert!(ledger.is_empty());
        assert_eq!(ledger.time(), 0.0);
        assert_eq!(ledger.energy(), 0);
        assert!(ledger.verify_chain());
    }

    #[test]
    fn record_witness_check() {
        let mut ledger = ProofLedger::new();
        ledger.record_witness_check("goldbach", 42, 6, true, false);
        assert_eq!(ledger.len(), 1);
        assert_eq!(ledger.energy(), 1);
        assert_eq!(ledger.total_checks(), 1);
        assert!(ledger.verify_chain());
    }

    #[test]
    fn record_proof_found() {
        let mut ledger = ProofLedger::new();
        ledger.record_proof_found("zfc_zero_ne_one", 5, [1u8; 32], "decide");
        assert_eq!(ledger.proofs_found(), 1);
        assert!(ledger.verify_chain());
    }

    #[test]
    fn chain_integrity() {
        let mut ledger = ProofLedger::new();
        ledger.record_witness_check("goldbach", 0, 0, true, false);
        ledger.record_witness_check("goldbach", 1, 1, true, false);
        ledger.record_witness_check("goldbach", 2, 1, true, false);
        ledger.record_proof_found("goldbach", 3, [0u8; 32], "magic");
        assert_eq!(ledger.len(), 4);
        assert!(ledger.verify_chain());

        // Each event's prev_hash points to the previous event's hash
        for i in 1..ledger.events().len() {
            assert_eq!(
                ledger.events()[i].prev_hash,
                ledger.events()[i - 1].event_hash,
                "chain broken at index {}", i
            );
        }
    }

    #[test]
    fn time_and_energy_accumulate() {
        let mut ledger = ProofLedger::new();
        for rank in 0..100u64 {
            ledger.record_witness_check("collatz", rank, rank as usize, true, false);
        }
        assert_eq!(ledger.energy(), 100);
        assert_eq!(ledger.time(), 100.0);
        assert_eq!(ledger.total_checks(), 100);
    }

    #[test]
    fn multiple_problems_tracked() {
        let mut ledger = ProofLedger::new();
        ledger.record_accelerator_result("zfc_zero_ne_one", "IRC", true);
        ledger.record_witness_check("goldbach", 0, 0, true, false);
        ledger.record_frontier("goldbach", 1, 0);
        assert_eq!(ledger.len(), 3);
        assert!(ledger.verify_chain());
    }
}
