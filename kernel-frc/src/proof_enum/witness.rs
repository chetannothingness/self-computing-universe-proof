//! Universal witness enumerator — the REAL engine.
//!
//! Enumerates ALL finite byte strings in canonical order: (length, then lexicographic).
//! Every finite byte string appears exactly once. Therefore every finite proof term
//! (which IS a finite byte string) appears at some rank.
//!
//! This is NOT a tactic vocabulary. This is NOT a smart search.
//! This is the exhaustive walk over ALL of D* (the free monoid on 256 symbols).
//! The Lean kernel does the filtering: Check(S, cert) = PASS or FAIL.
//!
//! Completeness guarantee: surjectivity of enumeration over finite strings.
//! Soundness guarantee: Lean's CIC type theory.

/// Universal witness enumerator over all finite byte strings.
///
/// Enumeration order: empty string, then all length-1, then all length-2, etc.
/// Within each length: lexicographic order (equivalent to base-256 counting).
///
/// Rank 0 = []
/// Rank 1 = [0x00]
/// Rank 2 = [0x01]
/// ...
/// Rank 256 = [0xFF]
/// Rank 257 = [0x00, 0x00]
/// Rank 258 = [0x00, 0x01]
/// ...
pub struct WitnessEnumerator {
    current: Vec<u8>,
    rank: u64,
}

impl WitnessEnumerator {
    /// Start enumeration from the empty string.
    pub fn new() -> Self {
        Self {
            current: vec![],
            rank: 0,
        }
    }

    /// Current rank (how many witnesses have been yielded so far).
    pub fn rank(&self) -> u64 {
        self.rank
    }
}

impl Iterator for WitnessEnumerator {
    type Item = (u64, Vec<u8>);

    fn next(&mut self) -> Option<Self::Item> {
        let result = (self.rank, self.current.clone());
        self.rank += 1;

        // Advance to next byte string in (length, lexicographic) order.
        // Treat current as a big-endian base-256 counter.
        // When all bytes overflow (carry propagates past MSB), increase length.
        let mut carry = true;
        for byte in self.current.iter_mut().rev() {
            if carry {
                if *byte == 255 {
                    *byte = 0;
                    carry = true;
                } else {
                    *byte += 1;
                    carry = false;
                    break;
                }
            }
        }
        if carry {
            // All bytes overflowed → move to next length (all zeros).
            self.current = vec![0u8; self.current.len() + 1];
        }

        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_is_empty() {
        let mut e = WitnessEnumerator::new();
        let (rank, bytes) = e.next().unwrap();
        assert_eq!(rank, 0);
        assert_eq!(bytes, Vec::<u8>::new());
    }

    #[test]
    fn second_is_zero_byte() {
        let mut e = WitnessEnumerator::new();
        e.next(); // skip empty
        let (rank, bytes) = e.next().unwrap();
        assert_eq!(rank, 1);
        assert_eq!(bytes, vec![0x00]);
    }

    #[test]
    fn rank_256_is_0xff() {
        let mut e = WitnessEnumerator::new();
        let mut last = (0, vec![]);
        for _ in 0..=256 {
            last = e.next().unwrap();
        }
        assert_eq!(last.0, 256);
        assert_eq!(last.1, vec![0xFF]);
    }

    #[test]
    fn rank_257_is_length_2() {
        let mut e = WitnessEnumerator::new();
        let mut last = (0, vec![]);
        for _ in 0..=257 {
            last = e.next().unwrap();
        }
        assert_eq!(last.0, 257);
        assert_eq!(last.1, vec![0x00, 0x00]);
    }

    #[test]
    fn rank_258_is_0x0001() {
        let mut e = WitnessEnumerator::new();
        let mut last = (0, vec![]);
        for _ in 0..=258 {
            last = e.next().unwrap();
        }
        assert_eq!(last.0, 258);
        assert_eq!(last.1, vec![0x00, 0x01]);
    }

    #[test]
    fn all_length_1_strings_appear() {
        let mut e = WitnessEnumerator::new();
        e.next(); // skip empty (rank 0)
        let length_1: Vec<Vec<u8>> = (0..256).map(|_| e.next().unwrap().1).collect();
        // Should be [0x00], [0x01], ..., [0xFF]
        for i in 0..256u16 {
            assert_eq!(length_1[i as usize], vec![i as u8]);
        }
    }

    #[test]
    fn enumerator_is_infinite() {
        let mut e = WitnessEnumerator::new();
        // Take 100000 elements — should never return None
        for _ in 0..100_000 {
            assert!(e.next().is_some());
        }
    }

    #[test]
    fn no_duplicates_in_first_1000() {
        let e = WitnessEnumerator::new();
        let witnesses: Vec<Vec<u8>> = e.take(1000).map(|(_, b)| b).collect();
        for i in 0..witnesses.len() {
            for j in (i + 1)..witnesses.len() {
                assert_ne!(witnesses[i], witnesses[j], "duplicate at ranks {} and {}", i, j);
            }
        }
    }
}
