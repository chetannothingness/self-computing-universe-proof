use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use crate::event::Event;
use std::collections::HashSet;

/// The Ledger: the ONLY history.
///
/// Append-only log with Merkle chain (linearized replay).
/// Dependency poset preserved in events via `deps`.
/// Used nonces tracked for capability non-replay.
pub struct Ledger {
    /// All committed events, in append order.
    events: Vec<Event>,
    /// The running chain hash: H(prev_head || Ser_Π(event)).
    /// This is the LedgerHead.
    head: Hash32,
    /// Total accumulated time (ΣΔT).
    total_time: u64,
    /// Total accumulated energy/cost (ΣΔE).
    total_energy: u64,
    /// Used nonces (for capability non-replay).
    used_nonces: HashSet<Hash32>,
    /// Hash of each event for quick lookup.
    event_hashes: Vec<Hash32>,
}

impl Ledger {
    /// Create a new ledger from ⊥ (nothingness).
    pub fn new() -> Self {
        Ledger {
            events: Vec::new(),
            head: HASH_ZERO,
            total_time: 0,
            total_energy: 0,
            used_nonces: HashSet::new(),
            event_hashes: Vec::new(),
        }
    }

    /// Commit an event to the ledger. Returns the new ledger head.
    pub fn commit(&mut self, event: Event) -> Hash32 {
        let event_bytes = event.ser_pi();
        self.head = hash::chain(&self.head, &event_bytes);
        self.total_time += event.shrink;
        self.total_energy += event.cost;
        let eh = event.hash();
        self.event_hashes.push(eh);
        self.events.push(event);
        self.head
    }

    /// Current ledger head hash.
    pub fn head(&self) -> Hash32 {
        self.head
    }

    /// Total committed events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Is the ledger empty (⊥)?
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Total accumulated time.
    pub fn total_time(&self) -> u64 {
        self.total_time
    }

    /// Total accumulated energy/cost.
    pub fn total_energy(&self) -> u64 {
        self.total_energy
    }

    /// Check if a nonce has been used.
    pub fn nonce_used(&self, nonce: &Hash32) -> bool {
        self.used_nonces.contains(nonce)
    }

    /// Record a nonce as used.
    pub fn use_nonce(&mut self, nonce: Hash32) -> bool {
        self.used_nonces.insert(nonce)
    }

    /// Get all events (for replay).
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Get event hashes (for dependency checking).
    pub fn event_hashes(&self) -> &[Hash32] {
        &self.event_hashes
    }

    /// Replay: verify that replaying all events from genesis
    /// produces the same ledger head.
    pub fn verify_replay(&self) -> bool {
        let mut head = HASH_ZERO;
        for event in &self.events {
            let event_bytes = event.ser_pi();
            head = hash::chain(&head, &event_bytes);
        }
        head == self.head
    }

    /// Get the last N event hashes (for branchpoint collection).
    pub fn last_n_hashes(&self, n: usize) -> Vec<Hash32> {
        let start = if self.event_hashes.len() > n {
            self.event_hashes.len() - n
        } else {
            0
        };
        self.event_hashes[start..].to_vec()
    }
}

impl SerPi for Ledger {
    fn ser_pi(&self) -> Vec<u8> {
        // The canonical serialization of a ledger is its head hash.
        // The full replay is available via events().
        canonical_cbor_bytes(&self.head.to_vec())
    }
}

impl Default for Ledger {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Event, EventKind};

    #[test]
    fn empty_ledger_is_bot() {
        let l = Ledger::new();
        assert_eq!(l.head(), HASH_ZERO);
        assert!(l.is_empty());
        assert_eq!(l.total_time(), 0);
        assert_eq!(l.total_energy(), 0);
    }

    #[test]
    fn commit_changes_head() {
        let mut l = Ledger::new();
        let e = Event::new(EventKind::Genesis, b"start", vec![], 0, 0);
        let h1 = l.commit(e);
        assert_ne!(h1, HASH_ZERO);
        assert_eq!(l.len(), 1);
    }

    #[test]
    fn deterministic_replay() {
        let mut l = Ledger::new();
        l.commit(Event::new(EventKind::Genesis, b"start", vec![], 0, 0));
        l.commit(Event::new(EventKind::InstrumentApplied, b"test1", vec![], 1, 5));
        l.commit(Event::new(EventKind::Branch, b"branch", vec![], 0, 10));
        assert!(l.verify_replay());
    }

    #[test]
    fn nonce_tracking() {
        let mut l = Ledger::new();
        let nonce = kernel_types::hash::H(b"nonce1");
        assert!(!l.nonce_used(&nonce));
        assert!(l.use_nonce(nonce));
        assert!(l.nonce_used(&nonce));
        assert!(!l.use_nonce(nonce)); // second use returns false
    }

    #[test]
    fn time_energy_accumulate() {
        let mut l = Ledger::new();
        l.commit(Event::new(EventKind::InstrumentApplied, b"a", vec![], 5, 10));
        l.commit(Event::new(EventKind::InstrumentApplied, b"b", vec![], 3, 7));
        assert_eq!(l.total_energy(), 8);
        assert_eq!(l.total_time(), 17);
    }
}
