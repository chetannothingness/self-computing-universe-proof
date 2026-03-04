use kernel_types::{Hash32, SerPi};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};

/// AGI domain kinds — one per capability being proved.
///
/// Each domain has a deterministic simulator-judge.
/// All are completable (finite experiment budget, finite solution space).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AgiDomainKind {
    SynthPhysics,
    AlienChemistry,
    CustomMath,
    CompanySandbox,
    BioMedSandbox,
    CausalReasoning,
    ModelDiscovery,
    MaterialsDesign,
    AlgoDiscovery,
    PhysicalReasoning,
    SocialReasoning,
    MultiStepPlanning,
}

impl AgiDomainKind {
    /// Human-readable name for this domain.
    pub fn name(&self) -> &'static str {
        match self {
            AgiDomainKind::SynthPhysics => "SynthPhysics",
            AgiDomainKind::AlienChemistry => "AlienChemistry",
            AgiDomainKind::CustomMath => "CustomMath",
            AgiDomainKind::CompanySandbox => "CompanySandbox",
            AgiDomainKind::BioMedSandbox => "BioMedSandbox",
            AgiDomainKind::CausalReasoning => "CausalReasoning",
            AgiDomainKind::ModelDiscovery => "ModelDiscovery",
            AgiDomainKind::MaterialsDesign => "MaterialsDesign",
            AgiDomainKind::AlgoDiscovery => "AlgoDiscovery",
            AgiDomainKind::PhysicalReasoning => "PhysicalReasoning",
            AgiDomainKind::SocialReasoning => "SocialReasoning",
            AgiDomainKind::MultiStepPlanning => "MultiStepPlanning",
        }
    }

    /// Phase number this domain belongs to.
    pub fn phase(&self) -> u8 {
        match self {
            AgiDomainKind::SynthPhysics
            | AgiDomainKind::AlienChemistry
            | AgiDomainKind::CustomMath => 2,
            AgiDomainKind::CompanySandbox
            | AgiDomainKind::BioMedSandbox => 3,
            AgiDomainKind::CausalReasoning => 6,
            AgiDomainKind::ModelDiscovery
            | AgiDomainKind::MaterialsDesign
            | AgiDomainKind::AlgoDiscovery => 7,
            AgiDomainKind::PhysicalReasoning
            | AgiDomainKind::SocialReasoning
            | AgiDomainKind::MultiStepPlanning => 8,
        }
    }

    /// Tag byte for canonical serialization.
    fn tag(&self) -> u8 {
        match self {
            AgiDomainKind::SynthPhysics => 0,
            AgiDomainKind::AlienChemistry => 1,
            AgiDomainKind::CustomMath => 2,
            AgiDomainKind::CompanySandbox => 3,
            AgiDomainKind::BioMedSandbox => 4,
            AgiDomainKind::CausalReasoning => 5,
            AgiDomainKind::ModelDiscovery => 6,
            AgiDomainKind::MaterialsDesign => 7,
            AgiDomainKind::AlgoDiscovery => 8,
            AgiDomainKind::PhysicalReasoning => 9,
            AgiDomainKind::SocialReasoning => 10,
            AgiDomainKind::MultiStepPlanning => 11,
        }
    }
}

impl SerPi for AgiDomainKind {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&("AgiDomainKind", self.tag()))
    }
}

/// AGI domain evaluation specification.
///
/// This is the data needed to construct an AgiDomain EvalSpec variant.
/// We keep it in the agi-proof crate rather than modifying kernel-contracts
/// directly, using the existing EvalSpec::Table variant as the carrier
/// (the goal spec and judge hash are encoded in the table entries).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiDomainSpec {
    pub domain: AgiDomainKind,
    pub world_seed: [u8; 32],
    pub goal_spec: Vec<u8>,
    pub judge_hash: Hash32,
    pub max_experiments: u64,
}

impl SerPi for AgiDomainSpec {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.domain.ser_pi());
        buf.extend_from_slice(&self.world_seed.ser_pi());
        buf.extend_from_slice(&self.goal_spec.ser_pi());
        buf.extend_from_slice(&self.judge_hash.ser_pi());
        buf.extend_from_slice(&self.max_experiments.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

impl AgiDomainSpec {
    /// Derive the completion bound B*(Q) for this domain.
    ///
    /// AGI domain tasks ARE completable: finite experiment budget,
    /// finite solution space, deterministic simulator.
    pub fn b_star(&self) -> u64 {
        self.max_experiments * 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_kind_serpi_deterministic() {
        let d1 = AgiDomainKind::SynthPhysics;
        let d2 = AgiDomainKind::SynthPhysics;
        assert_eq!(d1.ser_pi(), d2.ser_pi());
    }

    #[test]
    fn domain_kind_different_tags() {
        let physics = AgiDomainKind::SynthPhysics;
        let chem = AgiDomainKind::AlienChemistry;
        assert_ne!(physics.ser_pi(), chem.ser_pi());
    }

    #[test]
    fn domain_spec_serpi_deterministic() {
        let spec = AgiDomainSpec {
            domain: AgiDomainKind::SynthPhysics,
            world_seed: [42u8; 32],
            goal_spec: vec![1, 2, 3],
            judge_hash: [0u8; 32],
            max_experiments: 100,
        };
        let s1 = spec.ser_pi();
        let s2 = spec.ser_pi();
        assert_eq!(s1, s2);
    }

    #[test]
    fn b_star_derivation() {
        let spec = AgiDomainSpec {
            domain: AgiDomainKind::CustomMath,
            world_seed: [0u8; 32],
            goal_spec: vec![],
            judge_hash: [0u8; 32],
            max_experiments: 50,
        };
        assert_eq!(spec.b_star(), 500);
    }

    #[test]
    fn all_domains_have_phases() {
        let domains = vec![
            AgiDomainKind::SynthPhysics,
            AgiDomainKind::AlienChemistry,
            AgiDomainKind::CustomMath,
            AgiDomainKind::CompanySandbox,
            AgiDomainKind::BioMedSandbox,
            AgiDomainKind::CausalReasoning,
            AgiDomainKind::ModelDiscovery,
            AgiDomainKind::MaterialsDesign,
            AgiDomainKind::AlgoDiscovery,
            AgiDomainKind::PhysicalReasoning,
            AgiDomainKind::SocialReasoning,
            AgiDomainKind::MultiStepPlanning,
        ];
        for d in &domains {
            assert!(d.phase() >= 2 && d.phase() <= 8);
        }
        // All 12 domains have unique tags
        let mut tags: Vec<u8> = domains.iter().map(|d| d.tag()).collect();
        tags.sort();
        tags.dedup();
        assert_eq!(tags.len(), 12);
    }
}
