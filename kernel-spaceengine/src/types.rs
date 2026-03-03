use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use std::fmt;

/// Rational number: num/den. Denominator always > 0. Reduced by GCD.
/// NO FLOATS — all coordinates and physical parameters use integer or rational arithmetic.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rational {
    pub num: i64,
    pub den: u64,
}

impl Rational {
    pub fn new(num: i64, den: u64) -> Self {
        assert!(den > 0, "Rational denominator must be > 0");
        let g = gcd(num.unsigned_abs(), den);
        Rational {
            num: if g > 0 { num / g as i64 } else { num },
            den: if g > 0 { den / g } else { den },
        }
    }

    pub fn integer(n: i64) -> Self {
        Rational { num: n, den: 1 }
    }

    /// Write as a decimal string using integer arithmetic only.
    /// For SpaceEngine .sc files: e.g., Rational{num: 1234, den: 1000} → "1.234"
    pub fn to_sc_decimal(&self, decimal_places: u32) -> String {
        let scale = 10i64.pow(decimal_places);
        let scaled = (self.num * scale) / self.den as i64;
        let sign = if scaled < 0 { "-" } else { "" };
        let abs_scaled = scaled.unsigned_abs();
        let int_part = abs_scaled / scale as u64;
        let frac_part = abs_scaled % scale as u64;
        format!("{}{}.{:0>width$}", sign, int_part, frac_part, width = decimal_places as usize)
    }
}

impl SerPi for Rational {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&("Rational", self.num, self.den))
    }
}

fn gcd(a: u64, b: u64) -> u64 {
    if b == 0 { a } else { gcd(b, a % b) }
}

/// Celestial object kinds — one per EvalSpec type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CelestialKind {
    Star,
    Planet,
    Galaxy,
    Nebula,
    Cluster,
    DarkObject,
}

impl SerPi for CelestialKind {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            CelestialKind::Star => 0,
            CelestialKind::Planet => 1,
            CelestialKind::Galaxy => 2,
            CelestialKind::Nebula => 3,
            CelestialKind::Cluster => 4,
            CelestialKind::DarkObject => 5,
        };
        canonical_cbor_bytes(&("CelestialKind", tag))
    }
}

/// Galaxy morphology: SAT=Spiral, UNSAT=Elliptical.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GalaxyMorphology {
    Spiral,
    Elliptical,
}

impl SerPi for GalaxyMorphology {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            GalaxyMorphology::Spiral => 0,
            GalaxyMorphology::Elliptical => 1,
        };
        canonical_cbor_bytes(&("GalaxyMorphology", tag))
    }
}

/// Spectral class lookup from index.
pub fn spectral_class_name(idx: u8) -> &'static str {
    match idx % 7 {
        0 => "O", 1 => "B", 2 => "A", 3 => "F",
        4 => "G", 5 => "K", 6 => "M", _ => "M",
    }
}

/// Extract coordinates from QID bytes (deterministic).
/// Bytes 0-7 → x, 8-15 → y, 16-23 → z as i64::from_le_bytes.
pub fn coords_from_qid(qid: &Hash32) -> (i64, i64, i64) {
    let x = i64::from_le_bytes(qid[0..8].try_into().unwrap());
    let y = i64::from_le_bytes(qid[8..16].try_into().unwrap());
    let z = i64::from_le_bytes(qid[16..24].try_into().unwrap());
    (x, y, z)
}

/// ArithFind → StarSystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarSystem {
    pub qid_hex: String,
    pub name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    pub spectral_class: u8,
    pub luminosity: Rational,
    pub planet_orbits: Vec<Rational>,
    pub contract_hash: Hash32,
}

impl SerPi for StarSystem {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&self.spectral_class.ser_pi());
        buf.extend_from_slice(&self.luminosity.ser_pi());
        for orbit in &self.planet_orbits {
            buf.extend_from_slice(&orbit.ser_pi());
        }
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// BoolCnf → Galaxy (SAT=Spiral, UNSAT=Elliptical)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Galaxy {
    pub qid_hex: String,
    pub name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    pub arm_count: u32,
    pub radius_kpc: Rational,
    pub morphology: GalaxyMorphology,
    pub contract_hash: Hash32,
}

impl SerPi for Galaxy {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&(self.arm_count as u64).ser_pi());
        buf.extend_from_slice(&self.radius_kpc.ser_pi());
        buf.extend_from_slice(&self.morphology.ser_pi());
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Table → Nebula
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Nebula {
    pub qid_hex: String,
    pub name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    pub radius_ly: Rational,
    pub density: Rational,
    pub contract_hash: Hash32,
}

impl SerPi for Nebula {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&self.radius_ly.ser_pi());
        buf.extend_from_slice(&self.density.ser_pi());
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// FormalProof → DarkObject (inadmissible = invisible but present)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DarkObject {
    pub qid_hex: String,
    pub name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    pub mass_estimate: Rational,
    pub reason_inadmissible: String,
    pub contract_hash: Hash32,
}

impl SerPi for DarkObject {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&self.mass_estimate.ser_pi());
        buf.extend_from_slice(&self.reason_inadmissible.ser_pi());
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Dominate → StarCluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarCluster {
    pub qid_hex: String,
    pub name: String,
    pub coord_x: i64,
    pub coord_y: i64,
    pub coord_z: i64,
    pub member_count: u32,
    pub radius_pc: Rational,
    pub contract_hash: Hash32,
}

impl SerPi for StarCluster {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.qid_hex.ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.coord_x.ser_pi());
        buf.extend_from_slice(&self.coord_y.ser_pi());
        buf.extend_from_slice(&self.coord_z.ser_pi());
        buf.extend_from_slice(&(self.member_count as u64).ser_pi());
        buf.extend_from_slice(&self.radius_pc.ser_pi());
        buf.extend_from_slice(&self.contract_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Full kernel-derived catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelCatalog {
    pub stars: Vec<StarSystem>,
    pub galaxies: Vec<Galaxy>,
    pub nebulae: Vec<Nebula>,
    pub dark_objects: Vec<DarkObject>,
    pub clusters: Vec<StarCluster>,
    pub merkle_root: Hash32,
    pub kernel_build_hash: Hash32,
}

impl SerPi for KernelCatalog {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for s in &self.stars { buf.extend_from_slice(&s.ser_pi()); }
        for g in &self.galaxies { buf.extend_from_slice(&g.ser_pi()); }
        for n in &self.nebulae { buf.extend_from_slice(&n.ser_pi()); }
        for d in &self.dark_objects { buf.extend_from_slice(&d.ser_pi()); }
        for c in &self.clusters { buf.extend_from_slice(&c.ser_pi()); }
        buf.extend_from_slice(&self.merkle_root.ser_pi());
        buf.extend_from_slice(&self.kernel_build_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Write an integer as a decimal with given decimal places (pure integer arithmetic, zero floats).
/// E.g., write_integer_as_decimal(12345, 3) → "12.345"
pub fn write_integer_as_decimal(value: i64, decimal_places: u32) -> String {
    let scale = 10i64.pow(decimal_places);
    let sign = if value < 0 { "-" } else { "" };
    let abs_val = value.unsigned_abs();
    let int_part = abs_val / scale as u64;
    let frac_part = abs_val % scale as u64;
    format!("{}{}.{:0>width$}", sign, int_part, frac_part, width = decimal_places as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rational_reduction() {
        let r = Rational::new(6, 4);
        assert_eq!(r.num, 3);
        assert_eq!(r.den, 2);
    }

    #[test]
    fn rational_integer() {
        let r = Rational::integer(42);
        assert_eq!(r.num, 42);
        assert_eq!(r.den, 1);
    }

    #[test]
    fn rational_serpi_deterministic() {
        let r1 = Rational::new(3, 7);
        let r2 = Rational::new(3, 7);
        assert_eq!(r1.ser_pi(), r2.ser_pi());
        // Different value → different bytes
        let r3 = Rational::new(4, 7);
        assert_ne!(r1.ser_pi(), r3.ser_pi());
    }

    #[test]
    fn celestial_kind_tags_differ() {
        let kinds = vec![
            CelestialKind::Star, CelestialKind::Planet, CelestialKind::Galaxy,
            CelestialKind::Nebula, CelestialKind::Cluster, CelestialKind::DarkObject,
        ];
        for i in 0..kinds.len() {
            for j in (i+1)..kinds.len() {
                assert_ne!(kinds[i].ser_pi(), kinds[j].ser_pi(),
                    "{:?} and {:?} must have different SerPi", kinds[i], kinds[j]);
            }
        }
    }

    #[test]
    fn star_system_serpi() {
        let qid = hash::H(b"test_star");
        let star = StarSystem {
            qid_hex: hash::hex(&qid),
            name: "KS-test".into(),
            coord_x: 100, coord_y: 200, coord_z: 300,
            spectral_class: 4,
            luminosity: Rational::new(1, 1),
            planet_orbits: vec![Rational::new(1, 10), Rational::new(5, 10)],
            contract_hash: qid,
        };
        let bytes1 = star.ser_pi();
        let bytes2 = star.ser_pi();
        assert_eq!(bytes1, bytes2);
        assert!(!bytes1.is_empty());
    }

    #[test]
    fn galaxy_morphology_differ() {
        assert_ne!(
            GalaxyMorphology::Spiral.ser_pi(),
            GalaxyMorphology::Elliptical.ser_pi()
        );
    }

    #[test]
    fn write_integer_decimal_correct() {
        assert_eq!(write_integer_as_decimal(12345, 3), "12.345");
        assert_eq!(write_integer_as_decimal(-500, 3), "-0.500");
        assert_eq!(write_integer_as_decimal(0, 3), "0.000");
    }

    #[test]
    fn coords_from_qid_deterministic() {
        let qid = hash::H(b"coords_test");
        let (x1, y1, z1) = coords_from_qid(&qid);
        let (x2, y2, z2) = coords_from_qid(&qid);
        assert_eq!((x1, y1, z1), (x2, y2, z2));
    }
}
