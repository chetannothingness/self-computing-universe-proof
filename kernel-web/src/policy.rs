use kernel_types::Hash32;
use kernel_types::hash::H;
use kernel_types::provenance::{WebRequest, WebProvenance, HttpMethod};
#[cfg(test)]
use kernel_types::provenance::RetrievalPolicy;
use kernel_types::HASH_ZERO;
use serde::{Serialize, Deserialize};

/// Response from executing a web request.
#[derive(Debug, Clone)]
pub struct WebResponse {
    /// HTTP status code.
    pub status_code: u16,
    /// The response body bytes.
    pub body: Vec<u8>,
    /// Hash of the response headers.
    pub headers_hash: Hash32,
    /// Provenance record.
    pub provenance: WebProvenance,
}

/// Error from executing a web request.
/// TOTAL: errors are values, not panics.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WebError {
    Timeout { url: String, timeout_ms: u64 },
    ConnectionFailed { url: String, reason: String },
    TooManyRedirects { url: String, count: u64 },
    InvalidUrl { url: String },
}

impl std::fmt::Display for WebError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WebError::Timeout { url, timeout_ms } =>
                write!(f, "Timeout after {}ms: {}", timeout_ms, url),
            WebError::ConnectionFailed { url, reason } =>
                write!(f, "Connection failed: {} ({})", url, reason),
            WebError::TooManyRedirects { url, count } =>
                write!(f, "Too many redirects ({}): {}", count, url),
            WebError::InvalidUrl { url } =>
                write!(f, "Invalid URL: {}", url),
        }
    }
}

/// Execute a web request. Returns the response or a typed error.
/// This is the I/O boundary -- the ONLY place where network calls happen.
pub fn execute_request(request: &WebRequest) -> Result<WebResponse, WebError> {
    let timeout = std::time::Duration::from_millis(request.policy.timeout_ms);

    let client = reqwest::blocking::Client::builder()
        .timeout(timeout)
        .redirect(reqwest::redirect::Policy::limited(request.policy.max_redirects as usize))
        .user_agent(&request.policy.user_agent)
        .build()
        .map_err(|e| WebError::ConnectionFailed {
            url: request.url.clone(),
            reason: format!("{}", e),
        })?;

    let resp = match request.method {
        HttpMethod::Get => client.get(&request.url).send(),
        HttpMethod::Head => client.head(&request.url).send(),
    };

    let resp = resp.map_err(|e| {
        if e.is_timeout() {
            WebError::Timeout {
                url: request.url.clone(),
                timeout_ms: request.policy.timeout_ms,
            }
        } else if e.is_redirect() {
            WebError::TooManyRedirects {
                url: request.url.clone(),
                count: request.policy.max_redirects,
            }
        } else {
            WebError::ConnectionFailed {
                url: request.url.clone(),
                reason: format!("{}", e),
            }
        }
    })?;

    let status_code = resp.status().as_u16();

    // Hash the headers deterministically.
    let mut header_bytes = Vec::new();
    let mut header_keys: Vec<String> = resp.headers().keys()
        .map(|k| k.as_str().to_string())
        .collect();
    header_keys.sort();
    for key in &header_keys {
        if let Some(val) = resp.headers().get(key.as_str()) {
            header_bytes.extend_from_slice(key.as_bytes());
            header_bytes.push(b':');
            header_bytes.extend_from_slice(val.as_bytes());
            header_bytes.push(b'\n');
        }
    }
    let headers_hash = H(&header_bytes);

    let body = resp.bytes().map_err(|e| WebError::ConnectionFailed {
        url: request.url.clone(),
        reason: format!("body read failed: {}", e),
    })?.to_vec();

    let provenance = WebProvenance {
        resolver: "system".into(),
        tls_fingerprint: HASH_ZERO, // would need TLS introspection for real fingerprint
        redirects_hash: H(request.url.as_bytes()),
    };

    Ok(WebResponse {
        status_code,
        body,
        headers_hash,
        provenance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_error_display() {
        let e = WebError::Timeout { url: "https://x.com".into(), timeout_ms: 5000 };
        let s = format!("{}", e);
        assert!(s.contains("Timeout"));
        assert!(s.contains("5000"));
    }

    #[test]
    fn invalid_url_is_error() {
        let req = WebRequest {
            url: "not-a-url".into(),
            method: HttpMethod::Get,
            selector: kernel_types::provenance::WebSelector::FullBody,
            policy: RetrievalPolicy::default(),
        };
        let result = execute_request(&req);
        assert!(result.is_err());
    }
}
