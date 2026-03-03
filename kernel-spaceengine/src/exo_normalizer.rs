use kernel_types::{HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::{Event, EventKind, Ledger};
use crate::exo_types::*;
use std::collections::BTreeMap;

/// Deterministic normalization pipeline for NASA Exoplanet Archive data.
pub struct ExoNormalizer;

impl ExoNormalizer {
    /// Parse raw NASA Exoplanet Archive CSV into typed records.
    /// Deterministic: same bytes → same parsed records.
    pub fn parse_archive_data(raw_bytes: &[u8]) -> Result<Vec<RawExoRecord>, String> {
        let text = std::str::from_utf8(raw_bytes)
            .map_err(|e| format!("Invalid UTF-8: {}", e))?;

        let mut records = Vec::new();
        let mut lines = text.lines();

        // Find header line (skip comment lines starting with #).
        let mut header_fields = Vec::new();
        for line in &mut lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            header_fields = trimmed.split(',').map(|s| s.trim().to_lowercase()).collect();
            break;
        }

        if header_fields.is_empty() {
            return Ok(records);
        }

        // Build column index map.
        let col = |name: &str| -> Option<usize> {
            header_fields.iter().position(|h| h == name)
        };

        let i_hostname = col("hostname");
        let i_letter = col("pl_letter").or_else(|| col("pl_name"));
        let i_ra = col("ra").or_else(|| col("ra_str"));
        let i_dec = col("dec").or_else(|| col("dec_str"));
        let i_dist = col("sy_dist").or_else(|| col("st_dist"));
        let i_sptype = col("st_spectype").or_else(|| col("st_sptype"));
        let i_vmag = col("sy_vmag").or_else(|| col("st_vmag"));
        let i_gaia = col("gaia_id").or_else(|| col("gaia_dr3")).or_else(|| col("gaia_dr3_id"));
        let i_hip = col("hip_name").or_else(|| col("hip_id"));
        let i_hd = col("hd_name").or_else(|| col("hd_id"));
        let i_period = col("pl_orbper");
        let i_sma = col("pl_orbsmax");
        let i_ecc = col("pl_orbeccen");
        let i_inc = col("pl_orbincl");
        let i_mass = col("pl_bmassj").or_else(|| col("pl_massj"));
        let i_radius = col("pl_radj");
        let i_method = col("discoverymethod").or_else(|| col("pl_discmethod"));
        let i_year = col("disc_year").or_else(|| col("pl_disc_year"));
        let i_disp = col("disposition").or_else(|| col("pl_controv_flag"));

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            let fields: Vec<&str> = parse_csv_line(trimmed);

            let get = |idx: Option<usize>| -> Option<String> {
                idx.and_then(|i| fields.get(i))
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
            };

            let hostname = match get(i_hostname) {
                Some(h) => h,
                None => continue,
            };

            let raw_bytes_hash = hash::H(trimmed.as_bytes());

            records.push(RawExoRecord {
                hostname: hostname.clone(),
                planet_letter: get(i_letter).unwrap_or_else(|| "b".into()),
                ra_str: get(i_ra).unwrap_or_default(),
                dec_str: get(i_dec).unwrap_or_default(),
                dist_pc: get(i_dist),
                spectral_type: get(i_sptype),
                mag_v: get(i_vmag),
                gaia_dr3_id: get(i_gaia),
                hip_id: get(i_hip),
                hd_id: get(i_hd),
                period_days: get(i_period),
                semi_major_au: get(i_sma),
                eccentricity: get(i_ecc),
                inclination_deg: get(i_inc),
                mass_jupiter: get(i_mass),
                radius_jupiter: get(i_radius),
                discovery_method: get(i_method),
                discovery_year: get(i_year),
                disposition: get(i_disp),
                raw_bytes_hash,
            });
        }

        Ok(records)
    }

    /// Assign canonical HostKey to each record.
    /// Priority: gaia_dr3_id > HIP/HD > positional.
    pub fn canonicalize_hosts(records: &[RawExoRecord]) -> BTreeMap<HostKey, ExoHost> {
        let mut hosts = BTreeMap::new();

        for record in records {
            let key = Self::host_key_for_record(record);

            hosts.entry(key.clone()).or_insert_with(|| {
                let ra_mas = parse_decimal_to_milli(&record.ra_str, 3_600_000);
                let dec_mas = parse_decimal_to_milli(&record.dec_str, 3_600_000);
                let dist_mpc = record.dist_pc.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1000))
                    .unwrap_or(0);
                let mag_v_milli = record.mag_v.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1000))
                    .unwrap_or(0);
                let gaia_dr3_id = record.gaia_dr3_id.as_ref()
                    .and_then(|s| {
                        let trimmed = s.trim().trim_start_matches("Gaia DR3 ");
                        trimmed.parse::<u64>().ok().filter(|&id| id > 0)
                    });

                ExoHost {
                    canonical_key: key,
                    display_name: record.hostname.clone(),
                    ra_mas,
                    dec_mas,
                    dist_mpc,
                    spectral_type: record.spectral_type.clone().unwrap_or_default(),
                    mag_v_milli,
                    gaia_dr3_id,
                    source_hash: record.raw_bytes_hash,
                }
            });
        }

        hosts
    }

    /// Build the canonical HostKey for a raw record.
    fn host_key_for_record(record: &RawExoRecord) -> HostKey {
        // Priority 1: Gaia DR3 ID (NASA formats as raw number or "Gaia DR3 <id>")
        if let Some(ref gaia_str) = record.gaia_dr3_id {
            let id_str = gaia_str.trim()
                .trim_start_matches("Gaia DR3 ");
            if let Ok(id) = id_str.parse::<u64>() {
                if id > 0 {
                    return HostKey::GaiaDR3(id);
                }
            }
        }

        // Priority 2: HIP or HD catalog ID
        if let Some(ref hip) = record.hip_id {
            let id = hip.trim().to_uppercase();
            if !id.is_empty() {
                return HostKey::Catalog { catalog: "HIP".into(), id };
            }
        }
        if let Some(ref hd) = record.hd_id {
            let id = hd.trim().to_uppercase();
            if !id.is_empty() {
                return HostKey::Catalog { catalog: "HD".into(), id };
            }
        }

        // Priority 3: Positional fallback
        let ra_mas = parse_decimal_to_milli(&record.ra_str, 3_600_000);
        let dec_mas = parse_decimal_to_milli(&record.dec_str, 3_600_000);
        // Round to 1-arcsecond grid (1000 mas)
        let ra_grid = (ra_mas / 1000) * 1000;
        let dec_grid = (dec_mas / 1000) * 1000;
        let name_hash = hash::H(record.hostname.as_bytes());
        HostKey::Positional { ra_mas: ra_grid, dec_mas: dec_grid, name_hash }
    }

    /// Merge duplicate hosts and attach planets under canonical host.
    pub fn merge_hosts(
        hosts: &mut BTreeMap<HostKey, ExoHost>,
        planets: &mut BTreeMap<HostKey, Vec<ExoPlanet>>,
    ) {
        // Already using BTreeMap keyed by HostKey, so duplicates are
        // naturally merged during canonicalize_hosts. Planets are grouped
        // by host_key in build_planets. Nothing extra needed — the
        // BTreeMap deduplication IS the merge.
        let _ = (hosts, planets);
    }

    /// Build planet list from raw records, keyed by canonical host.
    pub fn build_planets(records: &[RawExoRecord]) -> (BTreeMap<HostKey, Vec<ExoPlanet>>, Vec<ExoPlanet>) {
        let mut planets: BTreeMap<HostKey, Vec<ExoPlanet>> = BTreeMap::new();
        let mut refuted = Vec::new();

        for record in records {
            let key = Self::host_key_for_record(record);
            let status = match record.disposition.as_deref() {
                Some("REFUTED") | Some("FALSE POSITIVE") => PlanetStatus::Refuted,
                Some("CONTROVERSIAL") => PlanetStatus::Controversial,
                _ => PlanetStatus::Confirmed,
            };

            let planet = ExoPlanet {
                host_key: key.clone(),
                planet_letter: record.planet_letter.clone(),
                display_name: format!("{} {}", record.hostname, record.planet_letter),
                period_micro_days: record.period_days.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1_000_000)),
                semi_major_milli_au: record.semi_major_au.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1000)),
                eccentricity_milli: record.eccentricity.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1000)),
                inclination_milli_deg: record.inclination_deg.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1000)),
                mass_micro_jupiter: record.mass_jupiter.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1_000_000)),
                radius_milli_jupiter: record.radius_jupiter.as_ref()
                    .map(|s| parse_decimal_to_milli(s, 1000)),
                discovery_method: record.discovery_method.clone().unwrap_or_default(),
                discovery_year: record.discovery_year.as_ref()
                    .and_then(|s| s.parse().ok()),
                status: status.clone(),
                source_hash: record.raw_bytes_hash,
            };

            if status == PlanetStatus::Refuted {
                refuted.push(planet);
            } else {
                planets.entry(key).or_default().push(planet);
            }
        }

        (planets, refuted)
    }

    /// Apply refutations: remove refuted planets from confirmed list.
    pub fn apply_refutations(
        planets: &mut BTreeMap<HostKey, Vec<ExoPlanet>>,
        refuted: &mut Vec<ExoPlanet>,
    ) {
        for host_planets in planets.values_mut() {
            let before = host_planets.len();
            host_planets.retain(|p| p.status != PlanetStatus::Refuted);
            let removed = before - host_planets.len();
            if removed > 0 {
                // Already tracked in refuted list during build_planets.
            }
        }
        // Remove empty host entries.
        planets.retain(|_, v| !v.is_empty());
        let _ = refuted;
    }

    /// Full normalization pipeline: parse → canonicalize → build planets → merge → refute.
    /// Emits ExoplanetNormalize ledger event.
    pub fn normalize(raw_bytes: &[u8], ledger: &mut Ledger) -> Result<RealUniverseCatalog, String> {
        let fetch_hash = hash::H(raw_bytes);
        let records = Self::parse_archive_data(raw_bytes)?;
        let mut hosts = Self::canonicalize_hosts(&records);
        let (mut planets, mut refuted) = Self::build_planets(&records);
        Self::merge_hosts(&mut hosts, &mut planets);
        Self::apply_refutations(&mut planets, &mut refuted);

        let host_count = hosts.len();
        let planet_count: usize = planets.values().map(|v| v.len()).sum();

        // Compute normalized hash over the canonical tables.
        let mut norm_buf = Vec::new();
        for (key, host) in &hosts {
            norm_buf.extend_from_slice(&key.ser_pi());
            norm_buf.extend_from_slice(&host.ser_pi());
        }
        for (key, pl_list) in &planets {
            norm_buf.extend_from_slice(&key.ser_pi());
            for p in pl_list {
                norm_buf.extend_from_slice(&p.ser_pi());
            }
        }
        let normalized_hash = hash::H(&norm_buf);

        // Merkle root will be computed when catalog files are emitted.
        let merkle_root = HASH_ZERO;

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&(
            "ExoplanetNormalize",
            host_count as u64,
            planet_count as u64,
            &normalized_hash.to_vec(),
        ));
        ledger.commit(Event::new(
            EventKind::ExoplanetNormalize,
            &payload,
            vec![],
            1,
            1,
        ));

        Ok(RealUniverseCatalog {
            hosts,
            planets,
            refuted,
            fetch_hash,
            normalized_hash,
            merkle_root,
            host_count,
            planet_count,
        })
    }
}

/// Parse a CSV line respecting quoted fields.
fn parse_csv_line(line: &str) -> Vec<&str> {
    let mut fields = Vec::new();
    let mut start = 0;
    let mut in_quotes = false;
    let bytes = line.as_bytes();

    for i in 0..bytes.len() {
        if bytes[i] == b'"' {
            in_quotes = !in_quotes;
        } else if bytes[i] == b',' && !in_quotes {
            fields.push(&line[start..i]);
            start = i + 1;
        }
    }
    fields.push(&line[start..]);
    fields
}

/// Parse a decimal string into integer with given scale.
/// E.g., parse_decimal_to_milli("1.234", 1000) → 1234
/// Uses pure integer arithmetic — no floats.
fn parse_decimal_to_milli(s: &str, scale: i64) -> i64 {
    let s = s.trim();
    if s.is_empty() { return 0; }

    let negative = s.starts_with('-');
    let s = s.trim_start_matches('-').trim_start_matches('+');

    let parts: Vec<&str> = s.splitn(2, '.').collect();
    let int_part: i64 = parts[0].parse().unwrap_or(0);

    let frac_value = if parts.len() > 1 {
        let frac_str = parts[1];
        let frac_digits = frac_str.len() as u32;
        let frac_raw: i64 = frac_str.parse().unwrap_or(0);
        let frac_scale = 10i64.pow(frac_digits);
        (frac_raw * scale) / frac_scale
    } else {
        0
    };

    let result = int_part * scale + frac_value;
    if negative { -result } else { result }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonicalize_gaia_preferred() {
        let record = RawExoRecord {
            hostname: "TOI-6894".into(),
            planet_letter: "b".into(),
            ra_str: "285.123".into(),
            dec_str: "-12.456".into(),
            dist_pc: Some("150.0".into()),
            spectral_type: Some("K2V".into()),
            mag_v: Some("11.2".into()),
            gaia_dr3_id: Some("Gaia DR3 12345678".into()),
            hip_id: Some("HIP 99999".into()),
            hd_id: None,
            period_days: Some("3.456".into()),
            semi_major_au: Some("0.045".into()),
            eccentricity: Some("0.012".into()),
            inclination_deg: Some("87.5".into()),
            mass_jupiter: Some("0.850".into()),
            radius_jupiter: Some("1.1".into()),
            discovery_method: Some("Transit".into()),
            discovery_year: Some("2024".into()),
            disposition: None,
            raw_bytes_hash: HASH_ZERO,
        };
        let hosts = ExoNormalizer::canonicalize_hosts(&[record]);
        assert_eq!(hosts.len(), 1);
        let key = hosts.keys().next().unwrap();
        assert!(matches!(key, HostKey::GaiaDR3(12345678)));
    }

    #[test]
    fn canonicalize_hip_fallback() {
        let record = RawExoRecord {
            hostname: "Test Star".into(),
            planet_letter: "b".into(),
            ra_str: "10.0".into(),
            dec_str: "20.0".into(),
            dist_pc: None, spectral_type: None, mag_v: None,
            gaia_dr3_id: None,
            hip_id: Some("HIP 12345".into()),
            hd_id: None,
            period_days: None, semi_major_au: None, eccentricity: None,
            inclination_deg: None, mass_jupiter: None, radius_jupiter: None,
            discovery_method: None, discovery_year: None, disposition: None,
            raw_bytes_hash: HASH_ZERO,
        };
        let hosts = ExoNormalizer::canonicalize_hosts(&[record]);
        let key = hosts.keys().next().unwrap();
        assert!(matches!(key, HostKey::Catalog { catalog, .. } if catalog == "HIP"));
    }

    #[test]
    fn canonicalize_positional_fallback() {
        let record = RawExoRecord {
            hostname: "Unknown Star".into(),
            planet_letter: "b".into(),
            ra_str: "100.0".into(),
            dec_str: "-30.0".into(),
            dist_pc: None, spectral_type: None, mag_v: None,
            gaia_dr3_id: None, hip_id: None, hd_id: None,
            period_days: None, semi_major_au: None, eccentricity: None,
            inclination_deg: None, mass_jupiter: None, radius_jupiter: None,
            discovery_method: None, discovery_year: None, disposition: None,
            raw_bytes_hash: HASH_ZERO,
        };
        let hosts = ExoNormalizer::canonicalize_hosts(&[record]);
        let key = hosts.keys().next().unwrap();
        assert!(matches!(key, HostKey::Positional { .. }));
    }

    #[test]
    fn merge_duplicate_hosts() {
        let records = vec![
            RawExoRecord {
                hostname: "Same Star".into(),
                planet_letter: "b".into(),
                ra_str: "10.0".into(), dec_str: "20.0".into(),
                dist_pc: None, spectral_type: None, mag_v: None,
                gaia_dr3_id: Some("Gaia DR3 99999".into()),
                hip_id: None, hd_id: None,
                period_days: Some("5.0".into()), semi_major_au: None, eccentricity: None,
                inclination_deg: None, mass_jupiter: None, radius_jupiter: None,
                discovery_method: None, discovery_year: None, disposition: None,
                raw_bytes_hash: hash::H(b"row1"),
            },
            RawExoRecord {
                hostname: "Same Star".into(),
                planet_letter: "c".into(),
                ra_str: "10.0".into(), dec_str: "20.0".into(),
                dist_pc: None, spectral_type: None, mag_v: None,
                gaia_dr3_id: Some("Gaia DR3 99999".into()),
                hip_id: None, hd_id: None,
                period_days: Some("10.0".into()), semi_major_au: None, eccentricity: None,
                inclination_deg: None, mass_jupiter: None, radius_jupiter: None,
                discovery_method: None, discovery_year: None, disposition: None,
                raw_bytes_hash: hash::H(b"row2"),
            },
        ];
        let hosts = ExoNormalizer::canonicalize_hosts(&records);
        // Two records with same Gaia ID → one host
        assert_eq!(hosts.len(), 1);
        let (planets, _) = ExoNormalizer::build_planets(&records);
        // Both planets under same host
        assert_eq!(planets.len(), 1);
        let pl = planets.values().next().unwrap();
        assert_eq!(pl.len(), 2);
    }

    #[test]
    fn refutation_removes_planet() {
        let records = vec![
            RawExoRecord {
                hostname: "Refuted Star".into(),
                planet_letter: "b".into(),
                ra_str: "10.0".into(), dec_str: "20.0".into(),
                dist_pc: None, spectral_type: None, mag_v: None,
                gaia_dr3_id: Some("Gaia DR3 88888".into()),
                hip_id: None, hd_id: None,
                period_days: None, semi_major_au: None, eccentricity: None,
                inclination_deg: None, mass_jupiter: None, radius_jupiter: None,
                discovery_method: None, discovery_year: None,
                disposition: Some("REFUTED".into()),
                raw_bytes_hash: HASH_ZERO,
            },
        ];
        let (planets, refuted) = ExoNormalizer::build_planets(&records);
        assert!(planets.is_empty());
        assert_eq!(refuted.len(), 1);
        assert_eq!(refuted[0].status, PlanetStatus::Refuted);
    }

    #[test]
    fn normalize_pipeline_deterministic() {
        let csv = "hostname,pl_letter,ra,dec,gaia_id,pl_orbper\nTOI-1234,b,100.0,-20.0,Gaia DR3 55555,3.5\n";
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let cat1 = ExoNormalizer::normalize(csv.as_bytes(), &mut l1).unwrap();
        let cat2 = ExoNormalizer::normalize(csv.as_bytes(), &mut l2).unwrap();
        assert_eq!(cat1.normalized_hash, cat2.normalized_hash);
        assert_eq!(cat1.host_count, 1);
        assert_eq!(cat1.planet_count, 1);
    }

    #[test]
    fn parse_decimal_to_milli_correct() {
        assert_eq!(parse_decimal_to_milli("1.234", 1000), 1234);
        assert_eq!(parse_decimal_to_milli("-5.5", 1000), -5500);
        assert_eq!(parse_decimal_to_milli("100", 1000), 100000);
        assert_eq!(parse_decimal_to_milli("0.001", 1000), 1);
    }
}
