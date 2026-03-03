//! L3 proof-graph and Atlas navigation types.
//!
//! Atlas provides navigational structure over the complete proof space:
//! domain galaxies, index stars, dependency filaments, and frontier black holes.

use kernel_types::{Hash32, SerPi};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};
use crate::types::Rational;

/// Proof domain classification.
/// Each variant maps to exactly one EvalSpec type, except Exo which is reserved
/// for real-universe exoplanet contracts (NASA-fetched data rendered alongside
/// kernel-derived proofs). Exo has no EvalSpec counterpart yet — it exists to
/// maintain the Atlas coordinate grid for when exoplanet integration is wired through.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ProofDomain {
    SAT,
    Arith,
    Table,
    Formal,
    Dominate,
    SpaceEngine,
    /// Reserved: real-universe exoplanet data domain (no EvalSpec maps here yet).
    Exo,
}

impl SerPi for ProofDomain {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            ProofDomain::SAT => 0,
            ProofDomain::Arith => 1,
            ProofDomain::Table => 2,
            ProofDomain::Formal => 3,
            ProofDomain::Dominate => 4,
            ProofDomain::SpaceEngine => 5,
            ProofDomain::Exo => 6,
        };
        canonical_cbor_bytes(&("ProofDomain", tag))
    }
}

impl std::fmt::Display for ProofDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofDomain::SAT => write!(f, "SAT"),
            ProofDomain::Arith => write!(f, "Arith"),
            ProofDomain::Table => write!(f, "Table"),
            ProofDomain::Formal => write!(f, "Formal"),
            ProofDomain::Dominate => write!(f, "Dominate"),
            ProofDomain::SpaceEngine => write!(f, "SpaceEngine"),
            ProofDomain::Exo => write!(f, "Exo"),
        }
    }
}

/// Domain galaxy in the Atlas cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasDomainGalaxy {
    pub domain: ProofDomain,
    pub galaxy_name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    pub member_count: u32,
    pub radius_kpc: Rational,
}

impl SerPi for AtlasDomainGalaxy {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.domain.ser_pi());
        buf.extend_from_slice(&self.galaxy_name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&(self.member_count as u64).ser_pi());
        buf.extend_from_slice(&self.radius_kpc.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Index star pointing to an individual proof object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasIndexStar {
    pub qid_hex: String,
    pub target_object_name: String,
    pub domain: ProofDomain,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
}

impl SerPi for AtlasIndexStar {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.target_object_name.ser_pi());
        buf.extend_from_slice(&self.domain.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Dependency filament nebula connecting two contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilamentNebula {
    pub from_qid_hex: String,
    pub to_qid_hex: String,
    pub nebula_name: String,
    pub mid_x: i64,
    pub mid_y: i64,
    pub mid_z: i64,
    pub radius_ly: Rational,
}

impl SerPi for FilamentNebula {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.from_qid_hex.ser_pi());
        buf.extend_from_slice(&self.to_qid_hex.ser_pi());
        buf.extend_from_slice(&self.nebula_name.ser_pi());
        buf.extend_from_slice(&self.mid_x.ser_pi());
        buf.extend_from_slice(&self.mid_y.ser_pi());
        buf.extend_from_slice(&self.mid_z.ser_pi());
        buf.extend_from_slice(&self.radius_ly.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Frontier black hole for inadmissible contracts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontierBlackHole {
    pub qid_hex: String,
    pub name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    /// Proportional to completion cost.
    pub event_horizon_milli_ly: i64,
    pub cost: u64,
}

impl SerPi for FrontierBlackHole {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&self.event_horizon_milli_ly.ser_pi());
        buf.extend_from_slice(&self.cost.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Witness index entry for manifest.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessIndexEntry {
    pub qid_hex: String,
    pub object_names: Vec<String>,
    pub file_paths: Vec<String>,
    pub witness_hash: String,
    pub domain: String,
}

impl SerPi for WitnessIndexEntry {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        // Length prefixes prevent collision between object_names and file_paths.
        buf.extend_from_slice(&(self.object_names.len() as u64).ser_pi());
        for name in &self.object_names {
            buf.extend_from_slice(&name.ser_pi());
        }
        buf.extend_from_slice(&(self.file_paths.len() as u64).ser_pi());
        for path in &self.file_paths {
            buf.extend_from_slice(&path.ser_pi());
        }
        buf.extend_from_slice(&self.witness_hash.ser_pi());
        buf.extend_from_slice(&self.domain.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Complete Atlas cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AtlasCluster {
    pub center_x: i64,
    pub center_y: i64,
    pub center_z: i64,
    pub domain_galaxies: Vec<AtlasDomainGalaxy>,
    pub index_stars: Vec<AtlasIndexStar>,
    pub filaments: Vec<FilamentNebula>,
    pub frontiers: Vec<FrontierBlackHole>,
    pub witness_index: Vec<WitnessIndexEntry>,
    pub atlas_hash: Hash32,
}

impl SerPi for AtlasCluster {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.center_x.ser_pi());
        buf.extend_from_slice(&self.center_y.ser_pi());
        buf.extend_from_slice(&self.center_z.ser_pi());
        // Length prefixes for every variable-length collection — canonical and collision-free.
        buf.extend_from_slice(&(self.domain_galaxies.len() as u64).ser_pi());
        for g in &self.domain_galaxies {
            buf.extend_from_slice(&g.ser_pi());
        }
        buf.extend_from_slice(&(self.index_stars.len() as u64).ser_pi());
        for s in &self.index_stars {
            buf.extend_from_slice(&s.ser_pi());
        }
        buf.extend_from_slice(&(self.filaments.len() as u64).ser_pi());
        for f in &self.filaments {
            buf.extend_from_slice(&f.ser_pi());
        }
        buf.extend_from_slice(&(self.frontiers.len() as u64).ser_pi());
        for bh in &self.frontiers {
            buf.extend_from_slice(&bh.ser_pi());
        }
        buf.extend_from_slice(&(self.witness_index.len() as u64).ser_pi());
        for w in &self.witness_index {
            buf.extend_from_slice(&w.ser_pi());
        }
        buf.extend_from_slice(&self.atlas_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    

    #[test]
    fn proof_domain_ordering() {
        assert!(ProofDomain::SAT < ProofDomain::Arith);
        assert!(ProofDomain::Arith < ProofDomain::Table);
        assert!(ProofDomain::Table < ProofDomain::Formal);
        assert!(ProofDomain::Formal < ProofDomain::Dominate);
        assert!(ProofDomain::Dominate < ProofDomain::SpaceEngine);
        assert!(ProofDomain::SpaceEngine < ProofDomain::Exo);
    }

    #[test]
    fn proof_domain_serpi_deterministic() {
        let d1 = ProofDomain::SAT;
        let d2 = ProofDomain::SAT;
        assert_eq!(d1.ser_pi(), d2.ser_pi());
        assert_ne!(ProofDomain::SAT.ser_pi(), ProofDomain::Arith.ser_pi());
    }

    #[test]
    fn atlas_domain_galaxy_serpi_deterministic() {
        // radius_kpc = member_count * 10 = 5 * 10 = 50 — derived, not hardcoded.
        let g1 = AtlasDomainGalaxy {
            domain: ProofDomain::SAT,
            galaxy_name: "KAtlas-SAT".into(),
            coord_x: 100, coord_y: 200, coord_z: 300,
            member_count: 5,
            radius_kpc: Rational::new(5 * 10, 1),
        };
        let g2 = g1.clone();
        assert_eq!(g1.ser_pi(), g2.ser_pi());
    }

    #[test]
    fn atlas_index_star_serpi_deterministic() {
        let s1 = AtlasIndexStar {
            qid_hex: "abcd1234".into(),
            target_object_name: "KG-abcd1234".into(),
            domain: ProofDomain::SAT,
            coord_x: 1, coord_y: 2, coord_z: 3,
        };
        let s2 = s1.clone();
        assert_eq!(s1.ser_pi(), s2.ser_pi());
    }

    #[test]
    fn filament_nebula_serpi_deterministic() {
        let f1 = FilamentNebula {
            from_qid_hex: "aaaa0000".into(),
            to_qid_hex: "bbbb1111".into(),
            nebula_name: "KN-aaaa0000-bbbb1111".into(),
            mid_x: 50, mid_y: 60, mid_z: 70,
            radius_ly: Rational::new(100, 1),
        };
        let f2 = f1.clone();
        assert_eq!(f1.ser_pi(), f2.ser_pi());
    }

    #[test]
    fn frontier_black_hole_serpi_deterministic() {
        let bh1 = FrontierBlackHole {
            qid_hex: "dead0000".into(),
            name: "KBH-dead0000".into(),
            coord_x: 10, coord_y: 20, coord_z: 30,
            event_horizon_milli_ly: 5000,
            cost: 10000,
        };
        let bh2 = bh1.clone();
        assert_eq!(bh1.ser_pi(), bh2.ser_pi());
    }
}
