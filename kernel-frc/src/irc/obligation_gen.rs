// Generate Base/Step/Link obligations from an invariant and transition system.

use crate::frc_types::{
    Invariant, InvariantKind, IrcObligation, ObligationKind, ObligationStatus,
    TransitionSystem,
};

/// Generate the three IRC obligations (Base, Step, Link) for a given invariant.
pub fn generate_obligations(
    invariant: &Invariant,
    ts: &TransitionSystem,
) -> (IrcObligation, IrcObligation, IrcObligation) {
    let base_stmt = generate_base_statement(invariant, ts);
    let step_stmt = generate_step_statement(invariant, ts);
    let link_stmt = generate_link_statement(invariant, ts);

    let base = IrcObligation::new(
        ObligationKind::Base,
        base_stmt,
        ObligationStatus::Gap {
            reason: "not yet attempted".to_string(),
            attempted_methods: vec![],
        },
    );

    let step = IrcObligation::new(
        ObligationKind::Step,
        step_stmt,
        ObligationStatus::Gap {
            reason: "not yet attempted".to_string(),
            attempted_methods: vec![],
        },
    );

    let link = IrcObligation::new(
        ObligationKind::Link,
        link_stmt,
        ObligationStatus::Gap {
            reason: "not yet attempted".to_string(),
            attempted_methods: vec![],
        },
    );

    (base, step, link)
}

fn generate_base_statement(invariant: &Invariant, _ts: &TransitionSystem) -> String {
    match invariant.kind {
        InvariantKind::Prefix => {
            // Prefix: I(0) = ∀m ≤ 0, P(m), i.e., P(0) or vacuously true
            format!("I(0) — base case of prefix invariant: {}", invariant.description)
        }
        InvariantKind::Bounding => {
            format!("I(0) — bounding invariant holds at 0: {}", invariant.description)
        }
        InvariantKind::Modular => {
            format!("I(0) — modular invariant holds at 0: {}", invariant.description)
        }
        InvariantKind::Structural => {
            format!("I(0) — structural invariant holds at initial state: {}", invariant.description)
        }
        InvariantKind::Composite => {
            format!("I(0) — composite invariant base: {}", invariant.description)
        }
        InvariantKind::Specialized => {
            format!("I(0) — specialized invariant base: {}", invariant.description)
        }
        InvariantKind::SECDerived => {
            format!("I(0) — SEC-derived invariant base: {}", invariant.description)
        }
    }
}

fn generate_step_statement(invariant: &Invariant, ts: &TransitionSystem) -> String {
    match invariant.kind {
        InvariantKind::Prefix => {
            // Step for prefix: ∀n, I(n) → I(n+1), i.e., P(n+1) given ∀m≤n, P(m)
            format!(
                "∀n, I(n) → I(n+1) — given ∀m ≤ n, P(m), show P(n+1) where P(n) = {}",
                ts.property_desc
            )
        }
        InvariantKind::Bounding => {
            format!(
                "∀n, I(n) → I(n+1) — bound preserved under transition {}: {}",
                ts.transition_desc, invariant.description
            )
        }
        InvariantKind::Modular => {
            format!(
                "∀n, I(n) → I(n+1) — modular property preserved: {}",
                invariant.description
            )
        }
        InvariantKind::Structural => {
            format!(
                "∀n, I(n) → I(n+1) — state membership preserved under {}: {}",
                ts.transition_desc, invariant.description
            )
        }
        InvariantKind::Composite => {
            format!(
                "∀n, I(n) → I(n+1) — composite invariant step: {}",
                invariant.description
            )
        }
        InvariantKind::Specialized => {
            format!(
                "∀n, I(n) → I(n+1) — specialized step: {}",
                invariant.description
            )
        }
        InvariantKind::SECDerived => {
            format!(
                "∀n, I(n) → I(n+1) — SEC-derived step: {}",
                invariant.description
            )
        }
    }
}

fn generate_link_statement(invariant: &Invariant, ts: &TransitionSystem) -> String {
    match invariant.kind {
        InvariantKind::Prefix => {
            // Link for prefix is trivial: I(n) = ∀m≤n, P(m) implies P(n)
            format!(
                "∀n, I(n) → P(n) — trivial for prefix: ∀m ≤ n, P(m) implies P(n) where P(n) = {}",
                ts.property_desc
            )
        }
        _ => {
            format!(
                "∀n, I(n) → P(n) — link invariant to property: {} implies {}",
                invariant.description, ts.property_desc
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frc_types::Invariant;
    use crate::irc::transition_system::build_transition_system;

    #[test]
    fn obligations_well_formed() {
        let ts = build_transition_system("goldbach");
        let inv = Invariant::new(
            InvariantKind::Prefix,
            "∀m ≤ n, isSumOfTwoPrimes(m)".to_string(),
            "def I (n : Nat) : Prop := ∀ m, m ≤ n → isSumOfTwoPrimes m".to_string(),
        );

        let (base, step, link) = generate_obligations(&inv, &ts);

        assert_eq!(base.kind, ObligationKind::Base);
        assert_eq!(step.kind, ObligationKind::Step);
        assert_eq!(link.kind, ObligationKind::Link);

        assert!(!base.statement.is_empty());
        assert!(!step.statement.is_empty());
        assert!(!link.statement.is_empty());

        // All start as gaps
        assert!(!base.is_discharged());
        assert!(!step.is_discharged());
        assert!(!link.is_discharged());
    }

    #[test]
    fn obligations_deterministic() {
        let ts = build_transition_system("lagrange");
        let inv = Invariant::new(
            InvariantKind::Prefix,
            "test".to_string(),
            "test".to_string(),
        );
        let (b1, s1, l1) = generate_obligations(&inv, &ts);
        let (b2, s2, l2) = generate_obligations(&inv, &ts);
        assert_eq!(b1.obligation_hash, b2.obligation_hash);
        assert_eq!(s1.obligation_hash, s2.obligation_hash);
        assert_eq!(l1.obligation_hash, l2.obligation_hash);
    }
}
