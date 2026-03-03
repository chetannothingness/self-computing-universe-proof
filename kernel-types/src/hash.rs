use crate::Hash32;

/// The ONE hash function used everywhere in the kernel.
/// blake3, pinned forever. No negotiation.
pub fn H(data: &[u8]) -> Hash32 {
    *blake3::hash(data).as_bytes()
}

/// Chain hash: H(prev || current_event_bytes).
/// This is the TraceHead / LedgerHead update rule.
pub fn chain(prev: &Hash32, event_bytes: &[u8]) -> Hash32 {
    let mut input = Vec::with_capacity(32 + event_bytes.len());
    input.extend_from_slice(prev);
    input.extend_from_slice(event_bytes);
    H(&input)
}

/// Merkle root of a list of hashes.
/// Empty list → HASH_ZERO. Single element → itself.
/// Odd list → last element is paired with itself.
pub fn merkle_root(hashes: &[Hash32]) -> Hash32 {
    if hashes.is_empty() {
        return crate::HASH_ZERO;
    }
    if hashes.len() == 1 {
        return hashes[0];
    }
    let mut level = hashes.to_vec();
    while level.len() > 1 {
        let mut next = Vec::with_capacity((level.len() + 1) / 2);
        for pair in level.chunks(2) {
            if pair.len() == 2 {
                next.push(chain(&pair[0], &pair[1]));
            } else {
                next.push(chain(&pair[0], &pair[0]));
            }
        }
        level = next;
    }
    level[0]
}

/// Hex encode a Hash32 for display.
pub fn hex(h: &Hash32) -> String {
    h.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Decode a hex string back into a Hash32.
/// Returns None if the string is not exactly 64 hex characters.
pub fn from_hex(s: &str) -> Option<Hash32> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_hash() {
        let a = H(b"hello");
        let b = H(b"hello");
        assert_eq!(a, b);
        assert_ne!(a, H(b"world"));
    }

    #[test]
    fn chain_deterministic() {
        let prev = H(b"genesis");
        let c1 = chain(&prev, b"event1");
        let c2 = chain(&prev, b"event1");
        assert_eq!(c1, c2);
    }

    #[test]
    fn merkle_empty() {
        assert_eq!(merkle_root(&[]), crate::HASH_ZERO);
    }

    #[test]
    fn merkle_single() {
        let h = H(b"x");
        assert_eq!(merkle_root(&[h]), h);
    }

    #[test]
    fn merkle_deterministic() {
        let a = H(b"a");
        let b = H(b"b");
        let c = H(b"c");
        let r1 = merkle_root(&[a, b, c]);
        let r2 = merkle_root(&[a, b, c]);
        assert_eq!(r1, r2);
    }
}
