use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use std::collections::BTreeMap;

/// Monotone cache trait: append-only, hash-chained, Pi-canonical.
pub trait MonotoneCache: Send + Sync {
    /// Get a value by key.
    fn get(&self, key: &[u8]) -> Option<&Vec<u8>>;

    /// Insert a value. Returns false if key already exists (append-only).
    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> bool;

    /// Current head hash (chain of all entries).
    fn head(&self) -> Hash32;

    /// Number of entries.
    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Generic monotone cache backed by BTreeMap.
pub struct GenericCache {
    entries: BTreeMap<Vec<u8>, Vec<u8>>,
    head: Hash32,
    name: String,
}

impl GenericCache {
    pub fn new(name: &str) -> Self {
        GenericCache {
            entries: BTreeMap::new(),
            head: kernel_types::HASH_ZERO,
            name: name.into(),
        }
    }
}

impl MonotoneCache for GenericCache {
    fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.entries.get(key)
    }

    fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> bool {
        if self.entries.contains_key(&key) {
            return false; // append-only: no overwrites
        }
        let mut entry_buf = Vec::new();
        entry_buf.extend_from_slice(&key);
        entry_buf.extend_from_slice(&value);
        self.head = hash::chain(&self.head, &entry_buf);
        self.entries.insert(key, value);
        true
    }

    fn head(&self) -> Hash32 {
        self.head
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Build graph index: maps build artifacts to their dependencies.
pub type BuildGraphIndex = GenericCache;

/// Symbol index: maps symbol names to locations.
pub type SymbolIndex = GenericCache;

/// Test blame map: maps failing tests to likely causes.
pub type TestBlameMap = GenericCache;

/// Patch motifs: reusable patch patterns.
pub type PatchMotifs = GenericCache;

/// Reproducer cache: maps bug descriptions to reproducers.
pub type ReproducerCache = GenericCache;

/// Proof cache: maps proof obligations to proof terms.
pub type ProofCache = GenericCache;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monotone_cache_append_only() {
        let mut cache = GenericCache::new("test");
        assert!(cache.insert(b"key1".to_vec(), b"val1".to_vec()));
        assert!(!cache.insert(b"key1".to_vec(), b"val2".to_vec())); // duplicate rejected
        assert_eq!(cache.get(b"key1"), Some(&b"val1".to_vec()));
    }

    #[test]
    fn monotone_cache_head_changes() {
        let mut cache = GenericCache::new("test");
        let h0 = cache.head();
        cache.insert(b"a".to_vec(), b"1".to_vec());
        let h1 = cache.head();
        assert_ne!(h0, h1);
        cache.insert(b"b".to_vec(), b"2".to_vec());
        let h2 = cache.head();
        assert_ne!(h1, h2);
    }

    #[test]
    fn monotone_cache_deterministic() {
        let mut c1 = GenericCache::new("test");
        let mut c2 = GenericCache::new("test");
        c1.insert(b"x".to_vec(), b"1".to_vec());
        c2.insert(b"x".to_vec(), b"1".to_vec());
        assert_eq!(c1.head(), c2.head());
    }
}
