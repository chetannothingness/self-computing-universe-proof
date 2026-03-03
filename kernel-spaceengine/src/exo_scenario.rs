use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::{Event, EventKind, Ledger};
use crate::exo_types::*;

/// A generated exoplanet scenario script.
#[derive(Debug, Clone)]
pub struct ExoScenarioScript {
    pub bytes: Vec<u8>,
    pub script_hash: Hash32,
}

impl SerPi for ExoScenarioScript {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(&self.bytes, &self.script_hash.to_vec()))
    }
}

/// Generates weekly proof scenario scripts for real-universe exoplanet data.
pub struct ExoScenarioGenerator;

impl ExoScenarioGenerator {
    /// Generate weekly proof scenario that auto-selects newest additions.
    /// Deterministic: sorted by discovery year (newest first), then by canonical name.
    pub fn generate(
        catalog: &RealUniverseCatalog,
        build_hash: &Hash32,
        merkle_root: &Hash32,
        ledger: &mut Ledger,
    ) -> ExoScenarioScript {
        let build_hex = &hash::hex(build_hash)[..16];
        let merkle_hex = &hash::hex(merkle_root)[..16];

        let mut script = String::new();
        script.push_str("SaveVars\n");
        script.push_str(&format!(
            "Log \"TOE_REAL: BuildHash={} Merkle={}\"\n",
            build_hex, merkle_hex
        ));
        script.push_str(&format!(
            "Print \"TOE_REAL BuildHash={}\" {{ Time 20 PosX 0.02 PosY 0.02 }}\n\n",
            build_hex
        ));

        // Collect all planets with their host info for sorting.
        let mut all_planets: Vec<(&ExoPlanet, &str)> = Vec::new();
        for (host_key, planet_list) in &catalog.planets {
            let host_name = catalog.hosts.get(host_key)
                .map(|h| h.display_name.as_str())
                .unwrap_or("Unknown");
            for planet in planet_list {
                all_planets.push((planet, host_name));
            }
        }

        // Sort by discovery year descending, then display_name ascending (deterministic).
        all_planets.sort_by(|a, b| {
            let year_a = a.0.discovery_year.unwrap_or(0);
            let year_b = b.0.discovery_year.unwrap_or(0);
            year_b.cmp(&year_a)
                .then_with(|| a.0.display_name.cmp(&b.0.display_name))
        });

        // Take top 20 (configurable) for the weekly proof tour.
        let tour_count = 20.min(all_planets.len());
        let tour = &all_planets[..tour_count];

        script.push_str("// --- Auto-selected newest systems (deterministic by discovery date) ---\n\n");

        for (idx, (_planet, host_name)) in tour.iter().enumerate() {
            script.push_str(&format!("Select \"{}\"\n", host_name));
            script.push_str("Goto { Time 6 DistRad 4 }\n");
            script.push_str("Wait 6\n");
            script.push_str(&format!(
                "Print \"MERKLE:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex
            ));
            script.push_str(&format!(
                "Screenshot {{ Name \"toe_weekly_{:02}_\" Format \"png\" }}\n\n",
                idx + 1
            ));
        }

        // Top 10 cinematic highlights: systems with multiple planets.
        let mut multi_planet_hosts: Vec<(&str, usize)> = Vec::new();
        for (host_key, planet_list) in &catalog.planets {
            if planet_list.len() >= 2 {
                let name = catalog.hosts.get(host_key)
                    .map(|h| h.display_name.as_str())
                    .unwrap_or("Unknown");
                multi_planet_hosts.push((name, planet_list.len()));
            }
        }
        multi_planet_hosts.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));

        if !multi_planet_hosts.is_empty() {
            script.push_str("// --- Top cinematic highlights (multi-planet systems) ---\n\n");
            for (name, count) in multi_planet_hosts.iter().take(10) {
                script.push_str(&format!("// {} ({} planets)\n", name, count));
                script.push_str(&format!("Select \"{}\"\n", name));
                script.push_str("Goto { Time 8 DistRad 6 }\n");
                script.push_str("Wait 8\n");
                script.push_str(&format!(
                    "Print \"MERKLE:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                    merkle_hex
                ));
                script.push_str(&format!(
                    "Screenshot {{ Name \"toe_highlight_{}\" Format \"png\" }}\n\n",
                    name.replace(' ', "_")
                ));
            }
        }

        script.push_str("RestoreVars\n");

        let bytes = script.into_bytes();
        let script_hash = hash::H(&bytes);

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&("ExoScenarioScript", &script_hash.to_vec()));
        ledger.commit(Event::new(
            EventKind::SpaceEngineScenarioEmit,
            &payload,
            vec![],
            1,
            1,
        ));

        ExoScenarioScript { bytes, script_hash }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn make_test_exo_catalog() -> RealUniverseCatalog {
        let key1 = HostKey::GaiaDR3(111);
        let key2 = HostKey::GaiaDR3(222);

        let mut hosts = BTreeMap::new();
        hosts.insert(key1.clone(), ExoHost {
            canonical_key: key1.clone(),
            display_name: "NewStar-2025".into(),
            ra_mas: 100000, dec_mas: 50000, dist_mpc: 100000,
            spectral_type: "G2V".into(), mag_v_milli: 10000,
            gaia_dr3_id: Some(111), source_hash: HASH_ZERO,
        });
        hosts.insert(key2.clone(), ExoHost {
            canonical_key: key2.clone(),
            display_name: "OldStar-2020".into(),
            ra_mas: 200000, dec_mas: -30000, dist_mpc: 200000,
            spectral_type: "K0III".into(), mag_v_milli: 8000,
            gaia_dr3_id: Some(222), source_hash: HASH_ZERO,
        });

        let mut planets = BTreeMap::new();
        planets.insert(key1.clone(), vec![ExoPlanet {
            host_key: key1.clone(),
            planet_letter: "b".into(),
            display_name: "NewStar-2025 b".into(),
            period_micro_days: Some(5000000),
            semi_major_milli_au: Some(50),
            eccentricity_milli: Some(10),
            inclination_milli_deg: Some(85000),
            mass_micro_jupiter: Some(1000000),
            radius_milli_jupiter: Some(1000),
            discovery_method: "Transit".into(),
            discovery_year: Some(2025),
            status: PlanetStatus::Confirmed,
            source_hash: HASH_ZERO,
        }]);
        planets.insert(key2.clone(), vec![ExoPlanet {
            host_key: key2.clone(),
            planet_letter: "b".into(),
            display_name: "OldStar-2020 b".into(),
            period_micro_days: Some(10000000),
            semi_major_milli_au: Some(100),
            eccentricity_milli: None,
            inclination_milli_deg: None,
            mass_micro_jupiter: None,
            radius_milli_jupiter: None,
            discovery_method: "RV".into(),
            discovery_year: Some(2020),
            status: PlanetStatus::Confirmed,
            source_hash: HASH_ZERO,
        }]);

        RealUniverseCatalog {
            hosts, planets, refuted: vec![],
            fetch_hash: HASH_ZERO, normalized_hash: HASH_ZERO,
            merkle_root: HASH_ZERO, host_count: 2, planet_count: 2,
        }
    }

    #[test]
    fn exo_scenario_deterministic() {
        let cat = make_test_exo_catalog();
        let build = hash::H(b"exo_build");
        let merkle = hash::H(b"exo_merkle");
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let s1 = ExoScenarioGenerator::generate(&cat, &build, &merkle, &mut l1);
        let s2 = ExoScenarioGenerator::generate(&cat, &build, &merkle, &mut l2);
        assert_eq!(s1.script_hash, s2.script_hash);
    }

    #[test]
    fn exo_scenario_selects_newest() {
        let cat = make_test_exo_catalog();
        let mut ledger = Ledger::new();
        let s = ExoScenarioGenerator::generate(&cat, &HASH_ZERO, &HASH_ZERO, &mut ledger);
        let text = String::from_utf8_lossy(&s.bytes);
        // NewStar-2025 (year 2025) should appear before OldStar-2020 (year 2020)
        let pos_new = text.find("NewStar-2025").unwrap();
        let pos_old = text.find("OldStar-2020").unwrap();
        assert!(pos_new < pos_old, "Newest system should appear first");
    }

    #[test]
    fn exo_scenario_has_log_line() {
        let cat = make_test_exo_catalog();
        let build = hash::H(b"log_test");
        let mut ledger = Ledger::new();
        let s = ExoScenarioGenerator::generate(&cat, &build, &HASH_ZERO, &mut ledger);
        let text = String::from_utf8_lossy(&s.bytes);
        assert!(text.contains("TOE_REAL: BuildHash="));
        assert!(text.contains("Merkle="));
    }
}
