pub mod bounded_counterexample;
pub mod finite_search;
pub mod effective_compactness;
pub mod proof_mining;
pub mod algebraic_decision;
pub mod certified_numerics;

pub use bounded_counterexample::BoundedCounterexampleSchema;
pub use finite_search::FiniteSearchSchema;
pub use effective_compactness::EffectiveCompactnessSchema;
pub use proof_mining::ProofMiningSchema;
pub use algebraic_decision::AlgebraicDecisionSchema;
pub use certified_numerics::CertifiedNumericsSchema;

use crate::schema::Schema;

/// Build the complete schema library in Π-canonical order.
pub fn build_schema_library() -> Vec<Box<dyn Schema>> {
    vec![
        Box::new(BoundedCounterexampleSchema),
        Box::new(FiniteSearchSchema),
        Box::new(EffectiveCompactnessSchema),
        Box::new(ProofMiningSchema),
        Box::new(AlgebraicDecisionSchema),
        Box::new(CertifiedNumericsSchema),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frc_types::SchemaId;

    #[test]
    fn schema_library_has_six_schemas() {
        let lib = build_schema_library();
        assert_eq!(lib.len(), 6);
    }

    #[test]
    fn schema_library_canonical_order() {
        let lib = build_schema_library();
        assert_eq!(lib[0].id(), SchemaId::BoundedCounterexample);
        assert_eq!(lib[1].id(), SchemaId::FiniteSearch);
        assert_eq!(lib[2].id(), SchemaId::EffectiveCompactness);
        assert_eq!(lib[3].id(), SchemaId::ProofMining);
        assert_eq!(lib[4].id(), SchemaId::AlgebraicDecision);
        assert_eq!(lib[5].id(), SchemaId::CertifiedNumerics);
    }

    #[test]
    fn schema_costs_monotone() {
        let lib = build_schema_library();
        for i in 1..lib.len() {
            assert!(lib[i].cost() >= lib[i - 1].cost(),
                "schema {} cost ({}) < schema {} cost ({})",
                i, lib[i].cost(), i - 1, lib[i - 1].cost());
        }
    }
}
