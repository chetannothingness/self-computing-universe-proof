use kernel_types::{Hash32, HASH_ZERO, SerPi};
use kernel_types::serpi::canonical_cbor_bytes;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// Canonical host star identity. Priority order:
/// 1. gaia_dr3_id (preferred — NASA has integrated Gaia DR3 IDs)
/// 2. HIP/HD/Gliese canonicalized name
/// 3. (ra_mas, dec_mas, name_hash) fallback
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum HostKey {
    GaiaDR3(u64),
    Catalog { catalog: String, id: String },
    Positional { ra_mas: i64, dec_mas: i64, name_hash: Hash32 },
}

impl SerPi for HostKey {
    fn ser_pi(&self) -> Vec<u8> {
        match self {
            HostKey::GaiaDR3(id) => canonical_cbor_bytes(&("GaiaDR3", *id)),
            HostKey::Catalog { catalog, id } => {
                canonical_cbor_bytes(&("Catalog", catalog.as_str(), id.as_str()))
            }
            HostKey::Positional { ra_mas, dec_mas, name_hash } => {
                canonical_cbor_bytes(&("Positional", *ra_mas, *dec_mas, &name_hash.to_vec()))
            }
        }
    }
}

/// A host star from the NASA Exoplanet Archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoHost {
    pub canonical_key: HostKey,
    pub display_name: String,
    pub ra_mas: i64,
    pub dec_mas: i64,
    pub dist_mpc: i64,
    pub spectral_type: String,
    pub mag_v_milli: i64,
    pub gaia_dr3_id: Option<u64>,
    pub source_hash: Hash32,
}

impl SerPi for ExoHost {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.canonical_key.ser_pi());
        buf.extend_from_slice(&self.display_name.ser_pi());
        buf.extend_from_slice(&self.ra_mas.ser_pi());
        buf.extend_from_slice(&self.dec_mas.ser_pi());
        buf.extend_from_slice(&self.dist_mpc.ser_pi());
        buf.extend_from_slice(&self.spectral_type.ser_pi());
        buf.extend_from_slice(&self.mag_v_milli.ser_pi());
        buf.extend_from_slice(&self.gaia_dr3_id.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.source_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// An exoplanet.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExoPlanet {
    pub host_key: HostKey,
    pub planet_letter: String,
    pub display_name: String,
    pub period_micro_days: Option<i64>,
    pub semi_major_milli_au: Option<i64>,
    pub eccentricity_milli: Option<i64>,
    pub inclination_milli_deg: Option<i64>,
    pub mass_micro_jupiter: Option<i64>,
    pub radius_milli_jupiter: Option<i64>,
    pub discovery_method: String,
    pub discovery_year: Option<i64>,
    pub status: PlanetStatus,
    pub source_hash: Hash32,
}

impl SerPi for ExoPlanet {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.host_key.ser_pi());
        buf.extend_from_slice(&self.planet_letter.ser_pi());
        buf.extend_from_slice(&self.display_name.ser_pi());
        buf.extend_from_slice(&self.period_micro_days.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.semi_major_milli_au.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.eccentricity_milli.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.inclination_milli_deg.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.mass_micro_jupiter.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.radius_milli_jupiter.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.discovery_method.ser_pi());
        buf.extend_from_slice(&self.discovery_year.unwrap_or(0).ser_pi());
        buf.extend_from_slice(&self.status.ser_pi());
        buf.extend_from_slice(&self.source_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanetStatus {
    Confirmed,
    Refuted,
    Controversial,
}

impl SerPi for PlanetStatus {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self {
            PlanetStatus::Confirmed => 0,
            PlanetStatus::Refuted => 1,
            PlanetStatus::Controversial => 2,
        };
        canonical_cbor_bytes(&("PlanetStatus", tag))
    }
}

/// The real-universe catalog after normalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealUniverseCatalog {
    pub hosts: BTreeMap<HostKey, ExoHost>,
    pub planets: BTreeMap<HostKey, Vec<ExoPlanet>>,
    pub refuted: Vec<ExoPlanet>,
    pub fetch_hash: Hash32,
    pub normalized_hash: Hash32,
    pub merkle_root: Hash32,
    pub host_count: usize,
    pub planet_count: usize,
}

impl SerPi for RealUniverseCatalog {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        for (key, host) in &self.hosts {
            buf.extend_from_slice(&key.ser_pi());
            buf.extend_from_slice(&host.ser_pi());
        }
        for (key, planets) in &self.planets {
            buf.extend_from_slice(&key.ser_pi());
            for p in planets {
                buf.extend_from_slice(&p.ser_pi());
            }
        }
        for r in &self.refuted {
            buf.extend_from_slice(&r.ser_pi());
        }
        buf.extend_from_slice(&self.fetch_hash.ser_pi());
        buf.extend_from_slice(&self.normalized_hash.ser_pi());
        buf.extend_from_slice(&self.merkle_root.ser_pi());
        buf.extend_from_slice(&(self.host_count as u64).ser_pi());
        buf.extend_from_slice(&(self.planet_count as u64).ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Raw record parsed from NASA Exoplanet Archive before canonicalization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawExoRecord {
    pub hostname: String,
    pub planet_letter: String,
    pub ra_str: String,
    pub dec_str: String,
    pub dist_pc: Option<String>,
    pub spectral_type: Option<String>,
    pub mag_v: Option<String>,
    pub gaia_dr3_id: Option<String>,
    pub hip_id: Option<String>,
    pub hd_id: Option<String>,
    pub period_days: Option<String>,
    pub semi_major_au: Option<String>,
    pub eccentricity: Option<String>,
    pub inclination_deg: Option<String>,
    pub mass_jupiter: Option<String>,
    pub radius_jupiter: Option<String>,
    pub discovery_method: Option<String>,
    pub discovery_year: Option<String>,
    pub disposition: Option<String>,
    pub raw_bytes_hash: Hash32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_key_ordering_gaia_first() {
        let gaia = HostKey::GaiaDR3(12345);
        let catalog = HostKey::Catalog { catalog: "HIP".into(), id: "12345".into() };
        let positional = HostKey::Positional { ra_mas: 0, dec_mas: 0, name_hash: HASH_ZERO };
        // GaiaDR3 < Catalog < Positional (enum variant ordering)
        assert!(gaia < catalog);
        assert!(catalog < positional);
    }

    #[test]
    fn host_key_serpi_deterministic() {
        let key1 = HostKey::GaiaDR3(999);
        let key2 = HostKey::GaiaDR3(999);
        assert_eq!(key1.ser_pi(), key2.ser_pi());
        let key3 = HostKey::GaiaDR3(1000);
        assert_ne!(key1.ser_pi(), key3.ser_pi());
    }

    #[test]
    fn planet_status_serpi_differ() {
        assert_ne!(PlanetStatus::Confirmed.ser_pi(), PlanetStatus::Refuted.ser_pi());
        assert_ne!(PlanetStatus::Confirmed.ser_pi(), PlanetStatus::Controversial.ser_pi());
        assert_ne!(PlanetStatus::Refuted.ser_pi(), PlanetStatus::Controversial.ser_pi());
    }

    #[test]
    fn real_universe_catalog_serpi() {
        let cat = RealUniverseCatalog {
            hosts: BTreeMap::new(),
            planets: BTreeMap::new(),
            refuted: vec![],
            fetch_hash: HASH_ZERO,
            normalized_hash: HASH_ZERO,
            merkle_root: HASH_ZERO,
            host_count: 0,
            planet_count: 0,
        };
        let bytes1 = cat.ser_pi();
        let bytes2 = cat.ser_pi();
        assert_eq!(bytes1, bytes2);
    }
}
