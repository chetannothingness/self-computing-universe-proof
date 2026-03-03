use kernel_types::provenance::WebSelector;

/// Apply a selector to a response body, extracting the relevant bytes.
/// Total: always returns a result. On invalid selector, returns empty bytes.
pub fn apply_selector(body: &[u8], selector: &WebSelector) -> Vec<u8> {
    match selector {
        WebSelector::FullBody => body.to_vec(),
        WebSelector::ByteRange(lo, hi) => {
            let lo = *lo as usize;
            let hi = (*hi as usize).min(body.len());
            if lo >= body.len() || lo >= hi {
                Vec::new()
            } else {
                body[lo..hi].to_vec()
            }
        }
        WebSelector::CssSelector(sel) => {
            // Simplified: search for the selector as a substring marker.
            // Full CSS selection would require an HTML parser (not in scope for kernel).
            // The kernel witnesses the hash of the selection, not the DOM.
            let marker = format!("<{}", sel);
            if let Some(pos) = body.windows(marker.len())
                .position(|w| w == marker.as_bytes())
            {
                body[pos..].to_vec()
            } else {
                Vec::new()
            }
        }
        WebSelector::XPath(path) => {
            // Simplified: treat as byte search marker.
            // Full XPath would require XML parsing.
            let marker = path.as_bytes();
            if let Some(pos) = body.windows(marker.len())
                .position(|w| w == marker)
            {
                body[pos..].to_vec()
            } else {
                Vec::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_body_returns_all() {
        let body = b"hello world";
        let result = apply_selector(body, &WebSelector::FullBody);
        assert_eq!(result, body);
    }

    #[test]
    fn byte_range_valid() {
        let body = b"0123456789";
        let result = apply_selector(body, &WebSelector::ByteRange(2, 5));
        assert_eq!(result, b"234");
    }

    #[test]
    fn byte_range_out_of_bounds() {
        let body = b"short";
        let result = apply_selector(body, &WebSelector::ByteRange(100, 200));
        assert!(result.is_empty());
    }

    #[test]
    fn css_selector_found() {
        let body = b"<html><div class='x'>content</div></html>";
        let result = apply_selector(body, &WebSelector::CssSelector("div".into()));
        assert!(!result.is_empty());
    }

    #[test]
    fn css_selector_not_found() {
        let body = b"<html><p>text</p></html>";
        let result = apply_selector(body, &WebSelector::CssSelector("table".into()));
        assert!(result.is_empty());
    }
}
