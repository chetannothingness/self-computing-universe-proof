use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use kernel_ledger::{Event, EventKind};

/// A trace entry: one step in the kernel's self-witness.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    /// Hash of the event at this step.
    pub event_hash: Hash32,
    /// The running trace head after this step.
    pub trace_head: Hash32,
    /// The event kind.
    pub kind: EventKind,
}

impl SerPi for TraceEntry {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.event_hash.ser_pi());
        buf.extend_from_slice(&self.trace_head.ser_pi());
        buf.extend_from_slice(&self.kind.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// The TraceEmitter: emits the kernel's own computation as a witness object.
///
/// Every solve produces:
/// - TraceHead: H_T (running chain hash of all events)
/// - Branchpoints: hashes of minimal state snapshots at each branch
/// - The full trace as a sequence of TraceEntry
///
/// This is "reverse witness direction": the kernel witnesses ITSELF.
pub struct TraceEmitter {
    /// All trace entries from the current execution.
    entries: Vec<TraceEntry>,
    /// The running trace head: H_{t+1} = H(H_t || Ser_Π(Event_t)).
    head: Hash32,
}

impl TraceEmitter {
    pub fn new() -> Self {
        TraceEmitter {
            entries: Vec::new(),
            head: HASH_ZERO,
        }
    }

    /// Record an event in the trace.
    pub fn record(&mut self, event: &Event) {
        let event_bytes = event.ser_pi();
        let event_hash = event.hash();
        self.head = hash::chain(&self.head, &event_bytes);

        self.entries.push(TraceEntry {
            event_hash,
            trace_head: self.head,
            kind: event.kind.clone(),
        });
    }

    /// Get the current trace head.
    pub fn head(&self) -> Hash32 {
        self.head
    }

    /// Get all trace entries.
    pub fn entries(&self) -> &[TraceEntry] {
        &self.entries
    }

    /// Get the number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Reset the emitter for a new solve.
    pub fn reset(&mut self) {
        self.entries.clear();
        self.head = HASH_ZERO;
    }

    /// Extract the canonical trace fingerprint: a hash of all trace heads.
    /// Two executions with the same fingerprint followed the same branch structure.
    pub fn fingerprint(&self) -> Hash32 {
        let heads: Vec<Hash32> = self.entries.iter().map(|e| e.trace_head).collect();
        hash::merkle_root(&heads)
    }

    /// Extract branchpoint hashes (entries where kind == Branch).
    pub fn branchpoints(&self) -> Vec<Hash32> {
        self.entries.iter()
            .filter(|e| e.kind == EventKind::Branch)
            .map(|e| e.trace_head)
            .collect()
    }

    /// Build a trace from a SolveOutput's receipt (for comparison).
    pub fn from_receipt(output: &SolveOutput) -> TraceSnapshot {
        TraceSnapshot {
            trace_head: output.receipt.trace_head,
            branchpoints: output.receipt.branchpoints.clone(),
        }
    }
}

impl Default for TraceEmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// A snapshot of a trace for comparison purposes.
#[derive(Debug, Clone)]
pub struct TraceSnapshot {
    pub trace_head: Hash32,
    pub branchpoints: Vec<Hash32>,
}

impl SerPi for TraceSnapshot {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.trace_head.ser_pi());
        for bp in &self.branchpoints {
            buf.extend_from_slice(&bp.ser_pi());
        }
        canonical_cbor_bytes(&buf)
    }
}

impl TraceSnapshot {
    /// Compare two trace snapshots.
    /// Returns None if they match, or Some(first divergence index) if they differ.
    pub fn compare(&self, other: &TraceSnapshot) -> Option<TraceMismatch> {
        if self.trace_head != other.trace_head {
            // Find the first divergent branchpoint.
            let min_len = self.branchpoints.len().min(other.branchpoints.len());
            for i in 0..min_len {
                if self.branchpoints[i] != other.branchpoints[i] {
                    return Some(TraceMismatch {
                        divergence_index: i,
                        expected: self.branchpoints[i],
                        actual: other.branchpoints[i],
                    });
                }
            }
            // Length mismatch.
            return Some(TraceMismatch {
                divergence_index: min_len,
                expected: if self.branchpoints.len() > min_len {
                    self.branchpoints[min_len]
                } else {
                    HASH_ZERO
                },
                actual: if other.branchpoints.len() > min_len {
                    other.branchpoints[min_len]
                } else {
                    HASH_ZERO
                },
            });
        }
        None
    }
}

/// A trace mismatch: where two traces diverge.
#[derive(Debug, Clone)]
pub struct TraceMismatch {
    /// Index of the first divergent branchpoint.
    pub divergence_index: usize,
    /// Expected branchpoint hash.
    pub expected: Hash32,
    /// Actual branchpoint hash.
    pub actual: Hash32,
}

impl std::fmt::Display for TraceMismatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Trace mismatch at branchpoint {}: expected {}, got {}",
            self.divergence_index,
            hash::hex(&self.expected),
            hash::hex(&self.actual),
        )
    }
}
