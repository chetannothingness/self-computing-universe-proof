use kernel_types::{Hash32, SerPi, hash};
use kernel_types::provenance::{WebRequest, HttpMethod, WebSelector, RetrievalPolicy};
use kernel_ledger::{Event, EventKind};
use kernel_instruments::instrument::{Instrument, InstrumentResult, InstrumentOutcome};
use kernel_instruments::state::{State, StateDelta};
use kernel_instruments::budget::Budget;
use crate::selector::apply_selector;
use crate::policy::{execute_request, WebError};

/// Web instrument: witnesses external web content into the ledger as hashes.
/// The kernel doesn't "trust" web bytes. It witnesses them as hashes with provenance.
pub struct WebInstrument {
    request: WebRequest,
    id: Hash32,
}

impl WebInstrument {
    pub fn new(url: String) -> Self {
        let request = WebRequest {
            url: url.clone(),
            method: HttpMethod::Get,
            selector: WebSelector::FullBody,
            policy: RetrievalPolicy::default(),
        };
        let id = hash::H(&request.ser_pi());
        WebInstrument { request, id }
    }

    pub fn with_request(request: WebRequest) -> Self {
        let id = hash::H(&request.ser_pi());
        WebInstrument { request, id }
    }
}

impl Instrument for WebInstrument {
    fn id(&self) -> Hash32 {
        self.id
    }

    fn cost(&self) -> u64 {
        100 // web retrieval is expensive relative to local computation
    }

    fn name(&self) -> &str {
        "WebInstrument"
    }

    fn apply(&self, _state: &State, budget: &Budget) -> InstrumentResult {
        // Check budget before expensive I/O.
        if !budget.can_afford(self.cost(), 0) {
            return self.error_result(b"BUDGET_EXCEEDED");
        }

        // Execute the request (I/O boundary).
        match execute_request(&self.request) {
            Ok(response) => {
                // Apply selector to response body.
                let selected = apply_selector(&response.body, &self.request.selector);
                let content_hash = hash::H(&selected);
                let url_hash = hash::H(self.request.url.as_bytes());

                let outcome = InstrumentOutcome {
                    value: content_hash.to_vec(),
                    shrink: 1, // web observation refines by witnessing external state
                };

                // Build state delta: store the web observation.
                let delta = StateDelta::empty()
                    .with_update(
                        format!("web:{}:hash", hash::hex(&url_hash)).into_bytes(),
                        content_hash.to_vec(),
                    )
                    .with_update(
                        format!("web:{}:status", hash::hex(&url_hash)).into_bytes(),
                        response.status_code.to_string().into_bytes(),
                    )
                    .with_update(
                        format!("web:{}:provenance", hash::hex(&url_hash)).into_bytes(),
                        response.provenance.ser_pi(),
                    );

                let event = Event::new(
                    EventKind::WebRetrieve,
                    &outcome.ser_pi(),
                    vec![],
                    self.cost(),
                    1,
                );

                InstrumentResult {
                    outcome,
                    delta,
                    cost: self.cost(),
                    events: vec![event],
                }
            }
            Err(e) => {
                // TOTAL: error is a valid outcome, not a panic.
                let error_tag = match &e {
                    WebError::Timeout { .. } => b"ERROR:TIMEOUT".to_vec(),
                    WebError::ConnectionFailed { .. } => b"ERROR:CONNECTION".to_vec(),
                    WebError::TooManyRedirects { .. } => b"ERROR:REDIRECTS".to_vec(),
                    WebError::InvalidUrl { .. } => b"ERROR:INVALID_URL".to_vec(),
                };
                self.error_result(&error_tag)
            }
        }
    }

    fn expected_refinement(&self, _state: &State) -> u64 {
        1 // web observation refines by 1 (witnesses one external fact)
    }
}

impl WebInstrument {
    fn error_result(&self, error_tag: &[u8]) -> InstrumentResult {
        let outcome = InstrumentOutcome {
            value: error_tag.to_vec(),
            shrink: 0,
        };
        let event = Event::new(
            EventKind::WebRetrieve,
            &outcome.ser_pi(),
            vec![],
            self.cost(),
            0,
        );
        InstrumentResult {
            outcome,
            delta: StateDelta::empty(),
            cost: self.cost(),
            events: vec![event],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_instrument_id_deterministic() {
        let a = WebInstrument::new("https://example.com".into());
        let b = WebInstrument::new("https://example.com".into());
        assert_eq!(a.id(), b.id());
    }

    #[test]
    fn web_instrument_id_differs_by_url() {
        let a = WebInstrument::new("https://example.com".into());
        let b = WebInstrument::new("https://other.com".into());
        assert_ne!(a.id(), b.id());
    }

    #[test]
    fn web_instrument_serpi_canonical() {
        let req = WebRequest {
            url: "https://example.com".into(),
            method: HttpMethod::Get,
            selector: WebSelector::FullBody,
            policy: RetrievalPolicy::default(),
        };
        let bytes1 = req.ser_pi();
        let bytes2 = req.ser_pi();
        assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn web_instrument_cost_correct() {
        let inst = WebInstrument::new("https://example.com".into());
        assert_eq!(inst.cost(), 100);
    }

    #[test]
    fn invalid_url_returns_error_outcome() {
        let inst = WebInstrument::new("not-a-url".into());
        let state = State::new();
        let budget = Budget::default_test();
        let result = inst.apply(&state, &budget);
        // Should return an error-tagged outcome, not panic.
        let val_str = String::from_utf8_lossy(&result.outcome.value);
        assert!(val_str.starts_with("ERROR:"));
        assert_eq!(result.outcome.shrink, 0);
    }
}
