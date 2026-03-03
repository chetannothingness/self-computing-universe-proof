use kernel_types::{Hash32, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::{Event, EventKind, Ledger};
use crate::exo_types::*;
use crate::types::write_integer_as_decimal;
use std::collections::BTreeMap;

/// Emits real-universe exoplanet catalogs in SpaceEngine format.
pub struct ExoCatalogEmitter;

impl ExoCatalogEmitter {
    /// Emit host stars as CSV (SpaceEngine supports CSV for speed/scale).
    /// addons/TOE_REAL/catalogs/stars/TOE_ExoHosts.csv
    pub fn emit_hosts_csv(hosts: &BTreeMap<HostKey, ExoHost>) -> Vec<u8> {
        let mut csv = String::new();
        csv.push_str("Name,RA,Dec,Dist,SpType,AppMagV\n");

        for host in hosts.values() {
            let ra = write_integer_as_decimal(host.ra_mas, 3);
            let dec = write_integer_as_decimal(host.dec_mas, 3);
            let dist = write_integer_as_decimal(host.dist_mpc, 3);
            let vmag = write_integer_as_decimal(host.mag_v_milli, 3);

            csv.push_str(&format!(
                "\"{}\",{},{},{},\"{}\",{}\n",
                host.display_name,
                ra, dec, dist,
                host.spectral_type,
                vmag,
            ));
        }

        csv.into_bytes()
    }

    /// Emit planets as .sc (SpaceEngine's native object format).
    /// addons/TOE_REAL/catalogs/planets/TOE_ExoPlanets.sc
    pub fn emit_planets_sc(planets: &BTreeMap<HostKey, Vec<ExoPlanet>>, hosts: &BTreeMap<HostKey, ExoHost>) -> Vec<u8> {
        let mut sc = String::new();

        for (host_key, planet_list) in planets {
            let parent_name = hosts.get(host_key)
                .map(|h| h.display_name.as_str())
                .unwrap_or("Unknown");

            for planet in planet_list {
                sc.push_str(&format!("Planet \"{}\"\n{{\n", planet.display_name));
                sc.push_str(&format!("    ParentBody \"{}\"\n", parent_name));

                if let Some(mass) = planet.mass_micro_jupiter {
                    sc.push_str(&format!("    Mass       {}\n",
                        write_integer_as_decimal(mass, 6)));
                }

                // Orbit block
                let has_orbit = planet.period_micro_days.is_some()
                    || planet.semi_major_milli_au.is_some()
                    || planet.eccentricity_milli.is_some()
                    || planet.inclination_milli_deg.is_some();

                if has_orbit {
                    sc.push_str("    Orbit\n    {\n");
                    if let Some(period) = planet.period_micro_days {
                        sc.push_str(&format!("        Period        {}\n",
                            write_integer_as_decimal(period, 6)));
                    }
                    if let Some(sma) = planet.semi_major_milli_au {
                        sc.push_str(&format!("        SemiMajorAxis {}\n",
                            write_integer_as_decimal(sma, 3)));
                    }
                    if let Some(ecc) = planet.eccentricity_milli {
                        sc.push_str(&format!("        Eccentricity  {}\n",
                            write_integer_as_decimal(ecc, 3)));
                    }
                    if let Some(inc) = planet.inclination_milli_deg {
                        sc.push_str(&format!("        Inclination   {}\n",
                            write_integer_as_decimal(inc, 3)));
                    }
                    sc.push_str("    }\n");
                }

                if let Some(radius) = planet.radius_milli_jupiter {
                    sc.push_str(&format!("    Radius    {}\n",
                        write_integer_as_decimal(radius, 3)));
                }

                sc.push_str("}\n\n");
            }
        }

        sc.into_bytes()
    }

    /// Emit all files as BTreeMap<filename, bytes>.
    pub fn emit_all(catalog: &RealUniverseCatalog) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();

        let hosts_csv = Self::emit_hosts_csv(&catalog.hosts);
        files.insert("catalogs/stars/TOE_ExoHosts.csv".into(), hosts_csv);

        let planets_sc = Self::emit_planets_sc(&catalog.planets, &catalog.hosts);
        files.insert("catalogs/planets/TOE_ExoPlanets.sc".into(), planets_sc);

        files
    }

    /// Emit all files and record ledger event.
    pub fn emit_with_ledger(catalog: &RealUniverseCatalog, ledger: &mut Ledger) -> BTreeMap<String, Vec<u8>> {
        let files = Self::emit_all(catalog);

        let file_hashes: Vec<Hash32> = files.iter()
            .map(|(name, bytes)| {
                let mut buf = Vec::new();
                buf.extend_from_slice(name.as_bytes());
                buf.extend_from_slice(bytes);
                hash::H(&buf)
            })
            .collect();
        let merkle_root = hash::merkle_root(&file_hashes);

        let payload = canonical_cbor_bytes(&(
            "ExoplanetCatalogEmit",
            files.len() as u64,
            &merkle_root.to_vec(),
        ));
        ledger.commit(Event::new(
            EventKind::ExoplanetCatalogEmit,
            &payload,
            vec![],
            1,
            1,
        ));

        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;

    fn make_test_catalog() -> RealUniverseCatalog {
        let key = HostKey::GaiaDR3(12345);
        let mut hosts = BTreeMap::new();
        hosts.insert(key.clone(), ExoHost {
            canonical_key: key.clone(),
            display_name: "TOI-6894".into(),
            ra_mas: 285123,
            dec_mas: -12456,
            dist_mpc: 150000,
            spectral_type: "K2V".into(),
            mag_v_milli: 11200,
            gaia_dr3_id: Some(12345),
            source_hash: HASH_ZERO,
        });

        let mut planets = BTreeMap::new();
        planets.insert(key.clone(), vec![ExoPlanet {
            host_key: key,
            planet_letter: "b".into(),
            display_name: "TOI-6894 b".into(),
            period_micro_days: Some(3456000),
            semi_major_milli_au: Some(45),
            eccentricity_milli: Some(12),
            inclination_milli_deg: Some(87500),
            mass_micro_jupiter: Some(850000),
            radius_milli_jupiter: Some(1100),
            discovery_method: "Transit".into(),
            discovery_year: Some(2024),
            status: PlanetStatus::Confirmed,
            source_hash: HASH_ZERO,
        }]);

        RealUniverseCatalog {
            hosts,
            planets,
            refuted: vec![],
            fetch_hash: HASH_ZERO,
            normalized_hash: HASH_ZERO,
            merkle_root: HASH_ZERO,
            host_count: 1,
            planet_count: 1,
        }
    }

    #[test]
    fn hosts_csv_deterministic() {
        let cat = make_test_catalog();
        let csv1 = ExoCatalogEmitter::emit_hosts_csv(&cat.hosts);
        let csv2 = ExoCatalogEmitter::emit_hosts_csv(&cat.hosts);
        assert_eq!(csv1, csv2);
    }

    #[test]
    fn planets_sc_deterministic() {
        let cat = make_test_catalog();
        let sc1 = ExoCatalogEmitter::emit_planets_sc(&cat.planets, &cat.hosts);
        let sc2 = ExoCatalogEmitter::emit_planets_sc(&cat.planets, &cat.hosts);
        assert_eq!(sc1, sc2);
    }

    #[test]
    fn csv_format_no_floats() {
        let cat = make_test_catalog();
        let csv = ExoCatalogEmitter::emit_hosts_csv(&cat.hosts);
        let text = String::from_utf8(csv).unwrap();
        assert!(text.contains("TOI-6894"));
        // Verify the CSV has integer-derived decimal values, not float notation
        assert!(!text.contains("e+"));
        assert!(!text.contains("E+"));
        assert!(!text.contains("NaN"));
        assert!(!text.contains("inf"));
    }

    #[test]
    fn sc_format_valid_syntax() {
        let cat = make_test_catalog();
        let sc = ExoCatalogEmitter::emit_planets_sc(&cat.planets, &cat.hosts);
        let text = String::from_utf8(sc).unwrap();
        assert!(text.contains("Planet \"TOI-6894 b\""));
        assert!(text.contains("ParentBody \"TOI-6894\""));
        assert!(text.contains("Orbit"));
        assert!(text.contains("Period"));
        assert!(text.contains("SemiMajorAxis"));
    }
}
