use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::{Event, EventKind, Ledger};
use crate::types::KernelCatalog;

/// A generated SpaceEngine scenario script.
#[derive(Debug, Clone)]
pub struct ScenarioScript {
    pub bytes: Vec<u8>,
    pub script_hash: Hash32,
}

impl SerPi for ScenarioScript {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(&self.bytes, &self.script_hash.to_vec()))
    }
}

/// Generates .se scenario scripts for proof tours.
pub struct ScenarioGenerator;

impl ScenarioGenerator {
    /// Generate a scenario script for a kernel-derived catalog.
    pub fn generate(
        catalog: &KernelCatalog,
        build_hash: &Hash32,
        merkle_root: &Hash32,
        ledger: &mut Ledger,
    ) -> ScenarioScript {
        let build_hex = &hash::hex(build_hash)[..16];
        let merkle_hex = &hash::hex(merkle_root)[..16];

        let mut script = String::new();
        script.push_str("SaveVars\n");
        script.push_str(&format!(
            "Log \"KernelTOE: BuildHash={} Merkle={}\"\n",
            build_hex, merkle_hex
        ));
        script.push_str(&format!(
            "Print \"KernelTOE BuildHash={}\" {{ Time 20 PosX 0.02 PosY 0.02 }}\n\n",
            build_hex
        ));

        let mut obj_idx = 1u32;

        // Tour stars
        for star in &catalog.stars {
            script.push_str(&format!("Select \"{}\"\n", star.name));
            script.push_str("Goto { Time 6 DistRad 4 }\n");
            script.push_str("Wait 6\n");
            script.push_str(&format!(
                "Print \"MERKLE:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex
            ));
            script.push_str(&format!(
                "Screenshot {{ Name \"toe_proof_{:02}_\" Format \"png\" }}\n\n",
                obj_idx
            ));
            obj_idx += 1;
        }

        // Tour galaxies
        for gal in &catalog.galaxies {
            script.push_str(&format!("Select \"{}\"\n", gal.name));
            script.push_str("Goto { Time 6 DistRad 6 }\n");
            script.push_str("Wait 6\n");
            script.push_str(&format!(
                "Print \"MERKLE:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex
            ));
            script.push_str(&format!(
                "Screenshot {{ Name \"toe_proof_{:02}_\" Format \"png\" }}\n\n",
                obj_idx
            ));
            obj_idx += 1;
        }

        // Tour nebulae
        for neb in &catalog.nebulae {
            script.push_str(&format!("Select \"{}\"\n", neb.name));
            script.push_str("Goto { Time 6 DistRad 5 }\n");
            script.push_str("Wait 6\n");
            script.push_str(&format!(
                "Print \"MERKLE:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex
            ));
            script.push_str(&format!(
                "Screenshot {{ Name \"toe_proof_{:02}_\" Format \"png\" }}\n\n",
                obj_idx
            ));
            obj_idx += 1;
        }

        // Tour clusters
        for cl in &catalog.clusters {
            script.push_str(&format!("Select \"{}\"\n", cl.name));
            script.push_str("Goto { Time 6 DistRad 5 }\n");
            script.push_str("Wait 6\n");
            script.push_str(&format!(
                "Print \"MERKLE:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex
            ));
            script.push_str(&format!(
                "Screenshot {{ Name \"toe_proof_{:02}_\" Format \"png\" }}\n\n",
                obj_idx
            ));
            obj_idx += 1;
        }

        // Dark objects: note their presence but don't tour (invisible).
        for dark in &catalog.dark_objects {
            script.push_str(&format!("// DarkObject \"{}\" (inadmissible — invisible)\n", dark.name));
        }

        script.push_str("\nRestoreVars\n");

        let bytes = script.into_bytes();
        let script_hash = hash::H(&bytes);

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&("ScenarioScript", &script_hash.to_vec()));
        ledger.commit(Event::new(
            EventKind::SpaceEngineScenarioEmit,
            &payload,
            vec![],
            1,
            1,
        ));

        ScenarioScript { bytes, script_hash }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;
    use crate::types::*;

    fn make_test_catalog() -> KernelCatalog {
        let qid = hash::H(b"test_scenario");
        KernelCatalog {
            stars: vec![StarSystem {
                qid_hex: hash::hex(&qid),
                name: "KS-test".into(),
                coord_x: 1, coord_y: 2, coord_z: 3,
                spectral_class: 4,
                luminosity: Rational::integer(1),
                planet_orbits: vec![],
                contract_hash: qid,
            }],
            galaxies: vec![Galaxy {
                qid_hex: hash::hex(&qid),
                name: "KG-test".into(),
                coord_x: 4, coord_y: 5, coord_z: 6,
                arm_count: 2,
                radius_kpc: Rational::integer(10),
                morphology: GalaxyMorphology::Spiral,
                contract_hash: qid,
            }],
            nebulae: vec![],
            dark_objects: vec![],
            clusters: vec![],
            merkle_root: HASH_ZERO,
            kernel_build_hash: HASH_ZERO,
        }
    }

    #[test]
    fn scenario_deterministic() {
        let cat = make_test_catalog();
        let build = hash::H(b"build");
        let merkle = hash::H(b"merkle");
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let s1 = ScenarioGenerator::generate(&cat, &build, &merkle, &mut l1);
        let s2 = ScenarioGenerator::generate(&cat, &build, &merkle, &mut l2);
        assert_eq!(s1.script_hash, s2.script_hash);
    }

    #[test]
    fn scenario_contains_all_objects() {
        let cat = make_test_catalog();
        let mut ledger = Ledger::new();
        let s = ScenarioGenerator::generate(&cat, &HASH_ZERO, &HASH_ZERO, &mut ledger);
        let text = String::from_utf8_lossy(&s.bytes);
        assert!(text.contains("KS-test"));
        assert!(text.contains("KG-test"));
    }

    #[test]
    fn scenario_has_hash_overlays() {
        let cat = make_test_catalog();
        let build = hash::H(b"overlay_test");
        let mut ledger = Ledger::new();
        let s = ScenarioGenerator::generate(&cat, &build, &HASH_ZERO, &mut ledger);
        let text = String::from_utf8_lossy(&s.bytes);
        assert!(text.contains("MERKLE:"));
        assert!(text.contains("BuildHash="));
    }

    #[test]
    fn scenario_hash_nonzero() {
        let cat = make_test_catalog();
        let mut ledger = Ledger::new();
        let s = ScenarioGenerator::generate(&cat, &HASH_ZERO, &HASH_ZERO, &mut ledger);
        assert_ne!(s.script_hash, HASH_ZERO);
    }
}
