//! L2 witness-content layer types.
//!
//! These encode the *witness content* — the actual satisfying assignment,
//! the refutation structure, the exact solution value — not just the answer status.
//! All implement SerPi. Zero floats.

use kernel_types::{Hash32, SerPi};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};
use crate::types::Rational;

/// SAT witness: moons encoding the satisfying assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SatWitnessMoons {
    pub parent_galaxy_name: String,
    pub qid_hex: String,
    pub moons: Vec<WitnessMoon>,
    pub clause_rings: Vec<ClauseRing>,
    pub contract_hash: Hash32,
}

impl SerPi for SatWitnessMoons {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.parent_galaxy_name.ser_pi());
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        // Length prefix: prevents collision between different partitions of
        // moons vs clause_rings (e.g., 3 moons + 1 ring != 1 moon + 3 rings).
        buf.extend_from_slice(&(self.moons.len() as u64).ser_pi());
        for m in &self.moons {
            buf.extend_from_slice(&m.ser_pi());
        }
        buf.extend_from_slice(&(self.clause_rings.len() as u64).ser_pi());
        for r in &self.clause_rings {
            buf.extend_from_slice(&r.ser_pi());
        }
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// One moon per variable in the SAT assignment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessMoon {
    pub moon_index: u32,
    pub variable_index: u32,
    pub bit_value: bool,
    /// +45000 for true, -45000 for false (milli-degrees).
    pub inclination_milli_deg: i64,
    /// 0 for true, 2 for false.
    pub phase_quadrant: u8,
    /// Clause group this moon belongs to.
    pub parent_ring_index: u32,
}

impl SerPi for WitnessMoon {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(
            "WitnessMoon",
            self.moon_index,
            self.variable_index,
            self.bit_value,
            self.inclination_milli_deg,
            self.phase_quadrant,
            self.parent_ring_index,
        ))
    }
}

/// One ring per clause in the SAT formula.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClauseRing {
    pub clause_index: u32,
    pub literal_count: u32,
    pub is_satisfied: bool,
    /// (clause_index + 1) * 1000 milli-AU.
    pub orbital_radius_milli_au: i64,
}

impl SerPi for ClauseRing {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(
            "ClauseRing",
            self.clause_index,
            self.literal_count,
            self.is_satisfied,
            self.orbital_radius_milli_au,
        ))
    }
}

/// UNSAT witness: proof-step stars in a globular cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnsatWitnessCluster {
    pub parent_galaxy_name: String,
    pub qid_hex: String,
    pub cluster_name: String,
    pub proof_steps: Vec<ProofStepStar>,
    /// Index of the central dense core (final contradiction).
    pub contradiction_step_index: u32,
    pub contract_hash: Hash32,
}

impl SerPi for UnsatWitnessCluster {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.parent_galaxy_name.ser_pi());
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.cluster_name.ser_pi());
        // Length prefix: number of proof steps encoded before the steps themselves.
        buf.extend_from_slice(&(self.proof_steps.len() as u64).ser_pi());
        for s in &self.proof_steps {
            buf.extend_from_slice(&s.ser_pi());
        }
        buf.extend_from_slice(&(self.contradiction_step_index as u64).ser_pi());
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// One star per clause in the UNSAT proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStepStar {
    pub step_index: u32,
    /// (clauses.len() - step_index) * 100 milli-parsecs.
    pub radial_distance_milli_pc: i64,
    /// Subcluster grouping: clause sharing most variables.
    pub parent_step_index: Option<u32>,
    /// Spectral class from step type.
    pub spectral_class: u8,
}

impl SerPi for ProofStepStar {
    fn ser_pi(&self) -> Vec<u8> {
        // Canonical Option encoding: (has_value: bool, value: u64).
        // No sentinel -1 — the tag bit is the honest truth about presence.
        let (has_parent, parent_val) = match self.parent_step_index {
            Some(p) => (true, p as u64),
            None => (false, 0u64),
        };
        canonical_cbor_bytes(&(
            "ProofStepStar",
            self.step_index,
            self.radial_distance_milli_pc,
            has_parent,
            parent_val,
            self.spectral_class,
        ))
    }
}

/// ArithFind witness: planet with exact orbital period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArithWitnessPlanet {
    pub parent_star_name: String,
    pub qid_hex: String,
    /// The x that satisfies P(x) = target.
    pub witness_value: i64,
    /// x as orbital period.
    pub period_exact: Rational,
    /// (2x+1)/2 — impossible for correct integer x.
    pub decoy_orbit: Rational,
    /// Always true when proof is correct.
    pub decoy_valid: bool,
    pub contract_hash: Hash32,
}

impl SerPi for ArithWitnessPlanet {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.parent_star_name.ser_pi());
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.witness_value.ser_pi());
        buf.extend_from_slice(&self.period_exact.ser_pi());
        buf.extend_from_slice(&self.decoy_orbit.ser_pi());
        buf.extend_from_slice(&self.decoy_valid.ser_pi());
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Dark object lensing proxy: visible surrogate for invisible dark objects.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LensingProxy {
    pub qid_hex: String,
    pub proxy_name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    /// From H(dark_object.ser_pi())[0..8].
    pub lensing_mass: Rational,
    pub einstein_radius_milli_arcsec: i64,
    pub dark_object_hash: Hash32,
}

impl SerPi for LensingProxy {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.proxy_name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&self.lensing_mass.ser_pi());
        buf.extend_from_slice(&self.einstein_radius_milli_arcsec.ser_pi());
        buf.extend_from_slice(&self.dark_object_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;

    fn make_test_moon(idx: u32, bit: bool) -> WitnessMoon {
        WitnessMoon {
            moon_index: idx,
            variable_index: idx,
            bit_value: bit,
            inclination_milli_deg: if bit { 45_000 } else { -45_000 },
            phase_quadrant: if bit { 0 } else { 2 },
            parent_ring_index: 0,
        }
    }

    #[test]
    fn sat_witness_moons_serpi_deterministic() {
        let w1 = SatWitnessMoons {
            parent_galaxy_name: "KG-test".into(),
            qid_hex: "abcd1234".into(),
            moons: vec![make_test_moon(0, true)],
            clause_rings: vec![ClauseRing {
                clause_index: 0, literal_count: 1, is_satisfied: true,
                orbital_radius_milli_au: 1000,
            }],
            contract_hash: HASH_ZERO,
        };
        let w2 = w1.clone();
        assert_eq!(w1.ser_pi(), w2.ser_pi());
    }

    #[test]
    fn witness_moon_serpi_deterministic() {
        let m1 = make_test_moon(0, true);
        let m2 = make_test_moon(0, true);
        assert_eq!(m1.ser_pi(), m2.ser_pi());
        let m3 = make_test_moon(0, false);
        assert_ne!(m1.ser_pi(), m3.ser_pi());
    }

    #[test]
    fn clause_ring_serpi_deterministic() {
        let r1 = ClauseRing {
            clause_index: 0, literal_count: 2, is_satisfied: true,
            orbital_radius_milli_au: 1000,
        };
        let r2 = r1.clone();
        assert_eq!(r1.ser_pi(), r2.ser_pi());
    }

    #[test]
    fn unsat_witness_cluster_serpi_deterministic() {
        let u1 = UnsatWitnessCluster {
            parent_galaxy_name: "KG-test".into(),
            qid_hex: "abcd1234".into(),
            cluster_name: "KC-test".into(),
            proof_steps: vec![ProofStepStar {
                step_index: 0, radial_distance_milli_pc: 100,
                parent_step_index: None, spectral_class: 6,
            }],
            contradiction_step_index: 0,
            contract_hash: HASH_ZERO,
        };
        let u2 = u1.clone();
        assert_eq!(u1.ser_pi(), u2.ser_pi());
    }

    #[test]
    fn proof_step_star_serpi_deterministic() {
        let s1 = ProofStepStar {
            step_index: 0, radial_distance_milli_pc: 200,
            parent_step_index: Some(1), spectral_class: 3,
        };
        let s2 = s1.clone();
        assert_eq!(s1.ser_pi(), s2.ser_pi());
    }

    #[test]
    fn arith_witness_planet_serpi_deterministic() {
        let a1 = ArithWitnessPlanet {
            parent_star_name: "KS-test".into(),
            qid_hex: "abcd1234".into(),
            witness_value: 5,
            period_exact: Rational::integer(5),
            decoy_orbit: Rational::new(11, 2),
            decoy_valid: true,
            contract_hash: HASH_ZERO,
        };
        let a2 = a1.clone();
        assert_eq!(a1.ser_pi(), a2.ser_pi());
    }

    #[test]
    fn lensing_proxy_serpi_deterministic() {
        let lp1 = LensingProxy {
            qid_hex: "abcd1234".into(),
            proxy_name: "KLens-test".into(),
            coord_x: 100, coord_y: 200, coord_z: 300,
            lensing_mass: Rational::new(5000, 1000),
            einstein_radius_milli_arcsec: 500,
            dark_object_hash: HASH_ZERO,
        };
        let lp2 = lp1.clone();
        assert_eq!(lp1.ser_pi(), lp2.ser_pi());
    }

    #[test]
    fn clause_ring_satisfaction_logic() {
        let satisfied = ClauseRing {
            clause_index: 0, literal_count: 3, is_satisfied: true,
            orbital_radius_milli_au: 1000,
        };
        let unsatisfied = ClauseRing {
            clause_index: 1, literal_count: 2, is_satisfied: false,
            orbital_radius_milli_au: 2000,
        };
        assert!(satisfied.is_satisfied);
        assert!(!unsatisfied.is_satisfied);
        assert_ne!(satisfied.ser_pi(), unsatisfied.ser_pi());
    }

    #[test]
    fn proof_step_radial_ordering() {
        // Steps further from center have larger radial distance.
        let step0 = ProofStepStar {
            step_index: 0, radial_distance_milli_pc: 300,
            parent_step_index: None, spectral_class: 6,
        };
        let step1 = ProofStepStar {
            step_index: 1, radial_distance_milli_pc: 200,
            parent_step_index: None, spectral_class: 6,
        };
        let step2 = ProofStepStar {
            step_index: 2, radial_distance_milli_pc: 100,
            parent_step_index: None, spectral_class: 6,
        };
        // Earlier steps are further out (radial = (total - index) * 100).
        assert!(step0.radial_distance_milli_pc > step1.radial_distance_milli_pc);
        assert!(step1.radial_distance_milli_pc > step2.radial_distance_milli_pc);
    }

    #[test]
    fn decoy_differs_from_witness() {
        let planet = ArithWitnessPlanet {
            parent_star_name: "KS-test".into(),
            qid_hex: "abcd1234".into(),
            witness_value: 5,
            period_exact: Rational::integer(5),
            decoy_orbit: Rational::new(11, 2),
            decoy_valid: true,
            contract_hash: HASH_ZERO,
        };
        // Decoy orbit must differ from witness period.
        assert_ne!(planet.period_exact.num, planet.decoy_orbit.num);
    }

    #[test]
    fn bit_encoding_correctness() {
        let moon_true = make_test_moon(0, true);
        let moon_false = make_test_moon(1, false);
        assert_eq!(moon_true.inclination_milli_deg, 45_000);
        assert_eq!(moon_false.inclination_milli_deg, -45_000);
        assert_eq!(moon_true.phase_quadrant, 0);
        assert_eq!(moon_false.phase_quadrant, 2);
    }

    #[test]
    fn witness_moon_inclination_sign() {
        let moon_t = make_test_moon(0, true);
        let moon_f = make_test_moon(0, false);
        assert!(moon_t.inclination_milli_deg > 0);
        assert!(moon_f.inclination_milli_deg < 0);
    }

    #[test]
    fn empty_moons_edge_case() {
        let w = SatWitnessMoons {
            parent_galaxy_name: "KG-empty".into(),
            qid_hex: "00000000".into(),
            moons: vec![],
            clause_rings: vec![],
            contract_hash: HASH_ZERO,
        };
        let bytes = w.ser_pi();
        assert!(!bytes.is_empty());
        // Determinism on empty.
        assert_eq!(bytes, w.ser_pi());
    }

    #[test]
    fn clause_ring_orbital_radius() {
        for i in 0..5u32 {
            let ring = ClauseRing {
                clause_index: i,
                literal_count: 2,
                is_satisfied: true,
                orbital_radius_milli_au: (i as i64 + 1) * 1000,
            };
            assert_eq!(ring.orbital_radius_milli_au, (i as i64 + 1) * 1000);
        }
    }
}
