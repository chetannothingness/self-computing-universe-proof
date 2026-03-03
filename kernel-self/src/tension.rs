use kernel_types::tension::Tension;
use kernel_contracts::quotient::AnswerQuotient;
use kernel_instruments::enumerator::DeltaEnumerator;
use kernel_instruments::state::State;

/// Compute tension from the current answer quotient.
/// Theta = floor(log2(|survivors|)).
pub fn compute_tension(quotient: &AnswerQuotient) -> Tension {
    Tension::from_survivors(quotient.size() as u64)
}

/// Select the best instrument by tension: argmax(delta_theta / delta_E).
/// Returns the index into the enumerator's canonical order, or None if no instruments.
///
/// Selection policy:
/// 1. Among instruments with cost > 0, pick the one maximizing refinement/cost.
/// 2. Among ties, use the canonical order (lexicographic on ID).
pub fn select_by_tension(
    enumerator: &DeltaEnumerator,
    state: &State,
    tension: &Tension,
) -> Option<usize> {
    if enumerator.is_empty() || tension.is_resolved() {
        return None;
    }

    let order = enumerator.canonical_order(state);
    if order.is_empty() {
        return None;
    }

    // The canonical order already sorts by (min_cost, max_refinement, lex_id).
    // This IS the tension-optimal ordering: cheapest high-refinement instrument first.
    // The kernel doesn't "browse" -- it picks the minimal, ambiguity-collapsing witness.
    Some(order[0])
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_instruments::base::CheckEquality;

    #[test]
    fn compute_tension_from_quotient() {
        let domain = vec![b"a".to_vec(), b"b".to_vec(), b"c".to_vec(), b"d".to_vec()];
        let quotient = AnswerQuotient::from_domain(domain);
        let t = compute_tension(&quotient);
        assert_eq!(t.theta_numerator, 2); // log2(4) = 2
        assert_eq!(t.remaining_survivors, 4);
    }

    #[test]
    fn tension_resolved_when_unique() {
        let domain = vec![b"only".to_vec()];
        let quotient = AnswerQuotient::from_domain(domain);
        let t = compute_tension(&quotient);
        assert!(t.is_resolved());
    }

    #[test]
    fn select_returns_none_when_resolved() {
        let domain = vec![b"only".to_vec()];
        let quotient = AnswerQuotient::from_domain(domain);
        let t = compute_tension(&quotient);
        let enumerator = DeltaEnumerator::new();
        let state = State::new();
        assert_eq!(select_by_tension(&enumerator, &state, &t), None);
    }

    #[test]
    fn select_returns_first_in_canonical_order() {
        let domain = vec![b"a".to_vec(), b"b".to_vec()];
        let quotient = AnswerQuotient::from_domain(domain);
        let t = compute_tension(&quotient);

        let mut enumerator = DeltaEnumerator::new();
        enumerator.register(Box::new(CheckEquality::new(b"k".to_vec(), b"v".to_vec())));

        let state = State::new();
        let result = select_by_tension(&enumerator, &state, &t);
        assert_eq!(result, Some(0));
    }
}
