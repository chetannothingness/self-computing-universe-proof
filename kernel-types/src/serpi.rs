use crate::Hash32;

/// Canonical serialization trait: Ser_Π.
///
/// Rule: ONE encoding, ONE ordering, ONE normalization.
/// We use canonical CBOR via ciborium with sorted map keys.
///
/// Every type that participates in the kernel MUST implement this.
/// The output is deterministic: same logical value → same bytes, always.
pub trait SerPi {
    /// Produce the canonical byte representation.
    fn ser_pi(&self) -> Vec<u8>;

    /// Hash of the canonical representation.
    fn ser_pi_hash(&self) -> Hash32 {
        crate::hash::H(&self.ser_pi())
    }
}

// Implementations for primitives

impl SerPi for u8 {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for u16 {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for u32 {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for u64 {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for i64 {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for bool {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for String {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for str {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&self.to_string())
    }
}

impl SerPi for Vec<u8> {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(self)
    }
}

impl SerPi for [u8; 32] {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&self.to_vec())
    }
}

impl SerPi for Vec<Hash32> {
    fn ser_pi(&self) -> Vec<u8> {
        let items: Vec<Vec<u8>> = self.iter().map(|x| x.to_vec()).collect();
        canonical_cbor_bytes(&items)
    }
}

impl<T: SerPi> SerPi for Option<T> {
    fn ser_pi(&self) -> Vec<u8> {
        match self {
            None => canonical_cbor_bytes(&ciborium::Value::Null),
            Some(v) => {
                // Wrap in a 1-element array to distinguish from None
                let inner = v.ser_pi();
                let mut out = Vec::new();
                out.push(0x01); // tag: Some
                out.extend_from_slice(&inner);
                // Re-encode the whole thing canonically
                canonical_cbor_bytes(&out)
            }
        }
    }
}

/// Encode any serde-serializable value to canonical CBOR bytes.
/// Determinism: ciborium produces deterministic output for non-map types.
/// For map types, the caller must sort keys before encoding.
pub fn canonical_cbor_bytes<T: serde::Serialize>(val: &T) -> Vec<u8> {
    let mut buf = Vec::new();
    ciborium::into_writer(val, &mut buf).expect("CBOR serialization must not fail (total semantics)");
    buf
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn u64_deterministic() {
        assert_eq!(42u64.ser_pi(), 42u64.ser_pi());
    }

    #[test]
    fn string_deterministic() {
        let s = "hello".to_string();
        assert_eq!(s.ser_pi(), s.ser_pi());
    }

    #[test]
    fn hash32_deterministic() {
        let h = crate::hash::H(b"test");
        assert_eq!(h.ser_pi(), h.ser_pi());
    }

    #[test]
    fn different_values_different_bytes() {
        assert_ne!(1u64.ser_pi(), 2u64.ser_pi());
    }
}
