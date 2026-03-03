use kernel_types::SerPi;
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// The kernel state: a finite map from keys to values.
/// Both keys and values are canonical byte strings.
///
/// BTreeMap ensures deterministic iteration order.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// The state map. BTreeMap for canonical key ordering.
    pub entries: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl State {
    pub fn new() -> Self {
        State {
            entries: BTreeMap::new(),
        }
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.entries.get(key)
    }

    pub fn set(&mut self, key: Vec<u8>, value: Vec<u8>) {
        self.entries.insert(key, value);
    }

    pub fn remove(&mut self, key: &[u8]) {
        self.entries.remove(key);
    }

    pub fn apply_delta(&mut self, delta: &StateDelta) {
        for (k, v) in &delta.updates {
            self.entries.insert(k.clone(), v.clone());
        }
        for k in &delta.removals {
            self.entries.remove(k);
        }
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for State {
    fn default() -> Self {
        Self::new()
    }
}

impl SerPi for State {
    fn ser_pi(&self) -> Vec<u8> {
        // BTreeMap is already sorted by key, so iteration is canonical.
        let pairs: Vec<(Vec<u8>, Vec<u8>)> = self.entries.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        canonical_cbor_bytes(&pairs)
    }
}

/// A state delta: the "eraser" produced by an instrument.
/// Only refines or updates state. No hidden side channels.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateDelta {
    /// Key-value pairs to set/update.
    pub updates: BTreeMap<Vec<u8>, Vec<u8>>,
    /// Keys to remove.
    pub removals: Vec<Vec<u8>>,
}

impl StateDelta {
    pub fn empty() -> Self {
        StateDelta {
            updates: BTreeMap::new(),
            removals: Vec::new(),
        }
    }

    pub fn with_update(mut self, key: Vec<u8>, value: Vec<u8>) -> Self {
        self.updates.insert(key, value);
        self
    }

    pub fn with_removal(mut self, key: Vec<u8>) -> Self {
        self.removals.push(key);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.updates.is_empty() && self.removals.is_empty()
    }
}

impl SerPi for StateDelta {
    fn ser_pi(&self) -> Vec<u8> {
        let updates: Vec<(Vec<u8>, Vec<u8>)> = self.updates.iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        let mut buf = Vec::new();
        buf.extend_from_slice(&canonical_cbor_bytes(&updates));
        let mut sorted_removals = self.removals.clone();
        sorted_removals.sort();
        buf.extend_from_slice(&canonical_cbor_bytes(&sorted_removals));
        canonical_cbor_bytes(&buf)
    }
}
