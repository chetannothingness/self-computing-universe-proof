use crate::Hash32;
use crate::serpi::{SerPi, canonical_cbor_bytes};
use serde::{Serialize, Deserialize};

/// Web provenance: cryptographic receipt of a web retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebProvenance {
    /// DNS resolver used.
    pub resolver: String,
    /// TLS certificate fingerprint hash.
    pub tls_fingerprint: Hash32,
    /// Hash of the redirect chain.
    pub redirects_hash: Hash32,
}

impl SerPi for WebProvenance {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.resolver.ser_pi());
        buf.extend_from_slice(&self.tls_fingerprint.ser_pi());
        buf.extend_from_slice(&self.redirects_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Policy governing a web retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalPolicy {
    pub timeout_ms: u64,
    pub max_redirects: u64,
    pub user_agent: String,
    pub cache_mode: CacheMode,
}

impl SerPi for RetrievalPolicy {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.timeout_ms.ser_pi());
        buf.extend_from_slice(&self.max_redirects.ser_pi());
        buf.extend_from_slice(&self.user_agent.ser_pi());
        buf.extend_from_slice(&self.cache_mode.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

impl Default for RetrievalPolicy {
    fn default() -> Self {
        RetrievalPolicy {
            timeout_ms: 10_000,
            max_redirects: 5,
            user_agent: "kernel-web/0.1".into(),
            cache_mode: CacheMode::ContentHash,
        }
    }
}

/// Caching strategy for web retrievals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CacheMode {
    /// Never cache.
    NoCache,
    /// Cache keyed by content hash.
    ContentHash,
    /// Cache keyed by time bucket (seconds).
    TimeBucket(u64),
}

impl SerPi for CacheMode {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            CacheMode::NoCache => 0,
            CacheMode::ContentHash => 1,
            CacheMode::TimeBucket(_) => 2,
        };
        let mut buf = Vec::new();
        buf.push(tag);
        if let CacheMode::TimeBucket(secs) = self {
            buf.extend_from_slice(&secs.ser_pi());
        }
        canonical_cbor_bytes(&buf)
    }
}

/// Selector for extracting content from a web response body.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebSelector {
    CssSelector(String),
    XPath(String),
    ByteRange(u64, u64),
    FullBody,
}

impl SerPi for WebSelector {
    fn ser_pi(&self) -> Vec<u8> {
        match self {
            WebSelector::CssSelector(s) => canonical_cbor_bytes(&("Css", s.as_str())),
            WebSelector::XPath(s) => canonical_cbor_bytes(&("XPath", s.as_str())),
            WebSelector::ByteRange(lo, hi) => canonical_cbor_bytes(&("ByteRange", lo, hi)),
            WebSelector::FullBody => canonical_cbor_bytes(&("FullBody", 0u8)),
        }
    }
}

/// HTTP method (restricted to safe methods).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HttpMethod {
    Get,
    Head,
}

impl SerPi for HttpMethod {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            HttpMethod::Get => 0,
            HttpMethod::Head => 1,
        };
        canonical_cbor_bytes(&tag)
    }
}

/// A web retrieval request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebRequest {
    pub url: String,
    pub method: HttpMethod,
    pub selector: WebSelector,
    pub policy: RetrievalPolicy,
}

impl SerPi for WebRequest {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.url.ser_pi());
        buf.extend_from_slice(&self.method.ser_pi());
        buf.extend_from_slice(&self.selector.ser_pi());
        buf.extend_from_slice(&self.policy.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HASH_ZERO;

    #[test]
    fn provenance_deterministic() {
        let p = WebProvenance {
            resolver: "8.8.8.8".into(),
            tls_fingerprint: HASH_ZERO,
            redirects_hash: HASH_ZERO,
        };
        assert_eq!(p.ser_pi(), p.ser_pi());
    }

    #[test]
    fn policy_deterministic() {
        let p = RetrievalPolicy::default();
        assert_eq!(p.ser_pi(), p.ser_pi());
    }

    #[test]
    fn cache_mode_variants_differ() {
        let a = CacheMode::NoCache;
        let b = CacheMode::ContentHash;
        let c = CacheMode::TimeBucket(60);
        assert_ne!(a.ser_pi(), b.ser_pi());
        assert_ne!(b.ser_pi(), c.ser_pi());
    }

    #[test]
    fn web_request_deterministic() {
        let r = WebRequest {
            url: "https://example.com".into(),
            method: HttpMethod::Get,
            selector: WebSelector::FullBody,
            policy: RetrievalPolicy::default(),
        };
        assert_eq!(r.ser_pi(), r.ser_pi());
    }

    #[test]
    fn selector_variants_differ() {
        let a = WebSelector::FullBody;
        let b = WebSelector::CssSelector("div".into());
        assert_ne!(a.ser_pi(), b.ser_pi());
    }
}
