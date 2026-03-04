//! Full per-QID .sc/.se/.json emission for the 4-layer visualization stack.
//!
//! Produces the complete file tree: per-QID catalogs, Atlas domain catalogs,
//! witness moon/planet files, proof-step clusters, dependency filaments,
//! lensing proxy stars, domain tour scripts, per-QID deep dive scripts,
//! and manifest/merkle/witness_index JSONs.

use kernel_types::{Hash32, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::{Contract, EvalSpec};
use kernel_ledger::{Event, EventKind, Ledger};
use crate::types::*;
use crate::witness_types::*;
use crate::atlas_types::*;
use std::collections::BTreeMap;

/// Emits the complete enhanced .pak file tree.
pub struct EnhancedEmitter;

impl EnhancedEmitter {
    /// Emit complete enhanced file tree.
    pub fn emit_all(
        catalog: &KernelCatalog,
        contracts: &[Contract],
        _outputs: &[SolveOutput],
        atlas: &AtlasCluster,
        sat_witnesses: &[SatWitnessMoons],
        unsat_witnesses: &[UnsatWitnessCluster],
        arith_witnesses: &[ArithWitnessPlanet],
        lensing_proxies: &[LensingProxy],
        build_hash: &Hash32,
        merkle_root: &Hash32,
        ledger: &mut Ledger,
    ) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();
        let build_hex16 = &hash::hex(build_hash)[..16];
        let merkle_hex16 = &hash::hex(merkle_root)[..16];

        // ── Per-QID galaxy files ──
        for gal in &catalog.galaxies {
            let prefix = &gal.qid_hex[..8.min(gal.qid_hex.len())];
            let sc = Self::emit_galaxy_sc(gal);
            files.insert(format!("catalogs/galaxies/KG-{}.sc", prefix), sc.into_bytes());
        }

        // ── AtlasDomains.sc ──
        {
            let mut sc = String::new();
            for dg in &atlas.domain_galaxies {
                sc.push_str(&format!("Galaxy \"{}\"\n{{\n", dg.galaxy_name));
                sc.push_str(&format!("    RA  {}\n", write_integer_as_decimal(dg.coord_x % 360_000, 3)));
                sc.push_str(&format!("    Dec {}\n", write_integer_as_decimal(dg.coord_y % 90_000, 3)));
                sc.push_str(&format!("    Dist {}\n", write_integer_as_decimal(dg.coord_z.unsigned_abs() as i64 % 10_000_000, 3)));
                sc.push_str("    Type \"S\"\n");
                sc.push_str(&format!("    Radius {}\n", dg.radius_kpc.to_sc_decimal(3)));
                sc.push_str("}\n\n");
            }
            files.insert("catalogs/galaxies/AtlasDomains.sc".into(), sc.into_bytes());
        }

        // ── Per-QID star files ──
        for star in &catalog.stars {
            let prefix = &star.qid_hex[..8.min(star.qid_hex.len())];
            let sc = Self::emit_star_sc(star);
            files.insert(format!("catalogs/stars/KS-{}.sc", prefix), sc.into_bytes());
        }

        // ── UNSAT proof-step star files ──
        for uw in unsat_witnesses {
            let prefix = &uw.qid_hex[..8.min(uw.qid_hex.len())];
            let sc = Self::emit_unsat_steps_sc(uw);
            files.insert(format!("catalogs/stars/KS-{}-Steps.sc", prefix), sc.into_bytes());
        }

        // ── Lensing proxy stars ──
        for lp in lensing_proxies {
            let prefix = &lp.qid_hex[..8.min(lp.qid_hex.len())];
            let sc = Self::emit_lensing_sc(lp);
            files.insert(format!("catalogs/stars/KLens-{}.sc", prefix), sc.into_bytes());
        }

        // ── SAT witness moon files ──
        for sw in sat_witnesses {
            let prefix = &sw.qid_hex[..8.min(sw.qid_hex.len())];
            let sc = Self::emit_sat_moons_sc(sw);
            files.insert(format!("catalogs/planets/KP-{}.sc", prefix), sc.into_bytes());
        }

        // ── ArithFind witness planet files ──
        for aw in arith_witnesses {
            let prefix = &aw.qid_hex[..8.min(aw.qid_hex.len())];
            let sc = Self::emit_arith_planet_sc(aw);
            files.insert(format!("catalogs/planets/KP-{}.sc", prefix), sc.into_bytes());
        }

        // ── UNSAT cluster files ──
        for uw in unsat_witnesses {
            let prefix = &uw.qid_hex[..8.min(uw.qid_hex.len())];
            let sc = Self::emit_unsat_cluster_sc(uw);
            files.insert(format!("catalogs/clusters/KC-{}.sc", prefix), sc.into_bytes());
        }

        // ── Atlas cluster file ──
        {
            let sc = Self::emit_atlas_cluster_sc(atlas);
            files.insert("catalogs/clusters/KC-Atlas.sc".into(), sc.into_bytes());
        }

        // ── Nebula files (existing Table contracts) ──
        for neb in &catalog.nebulae {
            let prefix = &neb.qid_hex[..8.min(neb.qid_hex.len())];
            let sc = Self::emit_nebula_sc(neb);
            files.insert(format!("catalogs/nebulae/KN-{}.sc", prefix), sc.into_bytes());
        }

        // ── Dependency filament nebulae ──
        for fil in &atlas.filaments {
            let sc = Self::emit_filament_sc(fil);
            files.insert(format!("catalogs/nebulae/{}.sc", fil.nebula_name), sc.into_bytes());
        }

        // ── Frontier black holes ──
        for bh in &atlas.frontiers {
            let prefix = &bh.qid_hex[..8.min(bh.qid_hex.len())];
            let sc = Self::emit_frontier_sc(bh);
            files.insert(format!("catalogs/stars/KBH-{}.sc", prefix), sc.into_bytes());
        }

        // ── Atlas tour script ──
        {
            let se = Self::emit_atlas_tour(atlas, build_hex16, merkle_hex16);
            files.insert("scripts/toe_proof_atlas.se".into(), se.into_bytes());
        }

        // ── Domain tour scripts ──
        // SAT domain tour
        {
            let se = Self::emit_domain_tour_sat(&catalog.galaxies, build_hex16, merkle_hex16);
            files.insert("scripts/toe_proof_SAT.se".into(), se.into_bytes());
        }
        // Arith domain tour
        {
            let se = Self::emit_domain_tour_arith(&catalog.stars, build_hex16, merkle_hex16);
            files.insert("scripts/toe_proof_Arith.se".into(), se.into_bytes());
        }
        // Table domain tour
        {
            let se = Self::emit_domain_tour_table(&catalog.nebulae, build_hex16, merkle_hex16);
            files.insert("scripts/toe_proof_Table.se".into(), se.into_bytes());
        }
        // Formal domain tour
        {
            let se = Self::emit_domain_tour_formal(&catalog.dark_objects, lensing_proxies, build_hex16, merkle_hex16);
            files.insert("scripts/toe_proof_Formal.se".into(), se.into_bytes());
        }
        // Dominate domain tour
        {
            let se = Self::emit_domain_tour_dominate(&catalog.clusters, build_hex16, merkle_hex16);
            files.insert("scripts/toe_proof_Dominate.se".into(), se.into_bytes());
        }
        // SpaceEngine domain tour (meta)
        {
            let mut se = String::new();
            se.push_str("SaveVars\n");
            se.push_str(&format!("Log \"KernelTOE-SpaceEngine: BuildHash={}\"\n", build_hex16));
            se.push_str("Print \"SpaceEngine Domain (meta-contracts)\" { Time 15 PosX 0.02 PosY 0.02 }\n");
            se.push_str("RestoreVars\n");
            files.insert("scripts/toe_proof_SpaceEngine.se".into(), se.into_bytes());
        }

        // ── Per-QID deep dive scripts ──
        for contract in contracts {
            let qid_hex = hash::hex(&contract.qid);
            let prefix = &qid_hex[..8.min(qid_hex.len())];
            let se = Self::emit_qid_deep_dive(
                contract, &qid_hex, prefix,
                sat_witnesses, unsat_witnesses, arith_witnesses,
                build_hex16, merkle_hex16,
            );
            files.insert(format!("scripts/toe_proof_{}.se", prefix), se.into_bytes());
        }

        // ── Witness index JSON ──
        let witness_index = Self::build_witness_index(
            contracts, &files,
            sat_witnesses, unsat_witnesses, arith_witnesses, lensing_proxies,
        );
        let witness_json = serde_json::to_vec_pretty(&witness_index).unwrap();
        files.insert("proof/witness_index.json".into(), witness_json);

        // ── Enhanced manifest JSON ──
        let manifest = Self::build_enhanced_manifest(
            catalog, atlas, sat_witnesses, unsat_witnesses, arith_witnesses,
            lensing_proxies, build_hash, merkle_root,
        );
        let manifest_json = serde_json::to_vec_pretty(&manifest).unwrap();
        files.insert("proof/manifest.json".into(), manifest_json);

        // ── Merkle JSON ──
        let file_hashes: Vec<(String, String)> = files.iter()
            .map(|(k, v)| (k.clone(), hash::hex(&hash::H(v))))
            .collect();
        let merkle_json = serde_json::to_vec_pretty(&file_hashes).unwrap();
        files.insert("proof/merkle.json".into(), merkle_json);

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&(
            "EnhancedVerify",
            files.len() as u64,
            witness_index.len() as u64,
        ));
        ledger.commit(Event::new(
            EventKind::EnhancedVerify,
            &payload,
            vec![],
            1,
            1,
        ));

        files
    }

    // ── Individual .sc emitters ──

    fn emit_galaxy_sc(gal: &Galaxy) -> String {
        let morph = match gal.morphology {
            GalaxyMorphology::Spiral => "S",
            GalaxyMorphology::Elliptical => "E",
        };
        format!(
            "Galaxy \"{}\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    Type \"{}\"\n    Radius {}\n}}\n",
            gal.name,
            write_integer_as_decimal(gal.coord_x % 360_000, 3),
            write_integer_as_decimal(gal.coord_y % 90_000, 3),
            write_integer_as_decimal(gal.coord_z.unsigned_abs() as i64 % 10_000_000, 3),
            morph,
            gal.radius_kpc.to_sc_decimal(3),
        )
    }

    fn emit_star_sc(star: &StarSystem) -> String {
        format!(
            "Star \"{}\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    Class \"{}\"\n    Lum {}\n}}\n",
            star.name,
            write_integer_as_decimal(star.coord_x % 360_000, 3),
            write_integer_as_decimal(star.coord_y % 90_000, 3),
            write_integer_as_decimal(star.coord_z.unsigned_abs() as i64 % 100_000, 3),
            spectral_class_name(star.spectral_class),
            star.luminosity.to_sc_decimal(3),
        )
    }

    fn emit_sat_moons_sc(sw: &SatWitnessMoons) -> String {
        let mut sc = String::new();
        for moon in &sw.moons {
            let incl = write_integer_as_decimal(moon.inclination_milli_deg, 3);
            sc.push_str(&format!(
                "Planet \"{parent}-Moon-{idx:03}\"\n{{\n    ParentBody \"{parent}\"\n    Class \"Selena\"\n    Orbit\n    {{\n        SemiMajorAxis 0.{semi:03}\n        Inclination {incl}\n        Period 0.{period:03}\n    }}\n}}\n\n",
                parent = sw.parent_galaxy_name,
                idx = moon.moon_index,
                semi = (moon.moon_index + 1),
                incl = incl,
                period = (moon.moon_index + 1),
            ));
        }
        sc
    }

    fn emit_arith_planet_sc(aw: &ArithWitnessPlanet) -> String {
        let period = aw.period_exact.to_sc_decimal(3);
        let semi = Rational::new(
            (aw.witness_value.unsigned_abs() as i64 * 1000 + 500) / 1000,
            1,
        );
        let decoy_period = aw.decoy_orbit.to_sc_decimal(3);
        let decoy_semi = Rational::new(
            ((aw.witness_value.unsigned_abs() as i64 * 2 + 1) * 500 + 500) / 1000,
            1,
        );
        format!(
            "Planet \"{parent}-Witness\"\n{{\n    ParentBody \"{parent}\"\n    Orbit\n    {{\n        Period {period}\n        SemiMajorAxis {semi}\n    }}\n}}\n\nPlanet \"{parent}-Decoy\"\n{{\n    ParentBody \"{parent}\"\n    Orbit\n    {{\n        Period {decoy_period}\n        SemiMajorAxis {decoy_semi}\n    }}\n}}\n",
            parent = aw.parent_star_name,
            period = period,
            semi = semi.to_sc_decimal(3),
            decoy_period = decoy_period,
            decoy_semi = decoy_semi.to_sc_decimal(3),
        )
    }

    fn emit_unsat_steps_sc(uw: &UnsatWitnessCluster) -> String {
        let mut sc = String::new();
        for step in &uw.proof_steps {
            sc.push_str(&format!(
                "Star \"{name}-Step-{idx:03}\"\n{{\n    RA  {ra}\n    Dec {dec}\n    Dist {dist}\n    Class \"{cls}\"\n}}\n\n",
                name = uw.cluster_name,
                idx = step.step_index,
                ra = write_integer_as_decimal(step.radial_distance_milli_pc % 360_000, 3),
                dec = write_integer_as_decimal(step.radial_distance_milli_pc % 90_000, 3),
                dist = write_integer_as_decimal(step.radial_distance_milli_pc, 3),
                cls = spectral_class_name(step.spectral_class),
            ));
        }
        sc
    }

    fn emit_unsat_cluster_sc(uw: &UnsatWitnessCluster) -> String {
        format!(
            "Cluster \"{}\"\n{{\n    NStars {}\n}}\n",
            uw.cluster_name,
            uw.proof_steps.len(),
        )
    }

    fn emit_lensing_sc(lp: &LensingProxy) -> String {
        format!(
            "Star \"{}\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    Class \"DA\"\n    Lum 0.001\n}}\n",
            lp.proxy_name,
            write_integer_as_decimal(lp.coord_x % 360_000, 3),
            write_integer_as_decimal(lp.coord_y % 90_000, 3),
            write_integer_as_decimal(lp.coord_z.unsigned_abs() as i64 % 100_000, 3),
        )
    }

    fn emit_nebula_sc(neb: &Nebula) -> String {
        format!(
            "Nebula \"{}\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    Radius {}\n}}\n",
            neb.name,
            write_integer_as_decimal(neb.coord_x % 360_000, 3),
            write_integer_as_decimal(neb.coord_y % 90_000, 3),
            write_integer_as_decimal(neb.coord_z.unsigned_abs() as i64 % 100_000, 3),
            neb.radius_ly.to_sc_decimal(3),
        )
    }

    fn emit_filament_sc(fil: &FilamentNebula) -> String {
        format!(
            "Nebula \"{}\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    Radius {}\n}}\n",
            fil.nebula_name,
            write_integer_as_decimal(fil.mid_x % 360_000, 3),
            write_integer_as_decimal(fil.mid_y % 90_000, 3),
            write_integer_as_decimal(fil.mid_z.unsigned_abs() as i64 % 100_000, 3),
            fil.radius_ly.to_sc_decimal(3),
        )
    }

    fn emit_frontier_sc(bh: &FrontierBlackHole) -> String {
        format!(
            "Star \"{}\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    Class \"X\"\n    Lum 0.000\n}}\n",
            bh.name,
            write_integer_as_decimal(bh.coord_x % 360_000, 3),
            write_integer_as_decimal(bh.coord_y % 90_000, 3),
            write_integer_as_decimal(bh.coord_z.unsigned_abs() as i64 % 100_000, 3),
        )
    }

    fn emit_atlas_cluster_sc(atlas: &AtlasCluster) -> String {
        format!(
            "Cluster \"KC-Atlas\"\n{{\n    RA  {}\n    Dec {}\n    Dist {}\n    NStars {}\n}}\n",
            write_integer_as_decimal(atlas.center_x % 360_000, 3),
            write_integer_as_decimal(atlas.center_y % 90_000, 3),
            write_integer_as_decimal(atlas.center_z.unsigned_abs() as i64 % 100_000, 3),
            atlas.index_stars.len(),
        )
    }

    // ── Scenario script emitters ──

    fn emit_atlas_tour(atlas: &AtlasCluster, build_hex16: &str, merkle_hex16: &str) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!(
            "Log \"KernelTOE-Full: BuildHash={} Merkle={} DarkObjects={} Filaments={}\"\n",
            build_hex16, merkle_hex16,
            atlas.frontiers.len(),
            atlas.filaments.len(),
        ));
        se.push_str(&format!(
            "Print \"KernelTOE-Full BuildHash={}\" {{ Time 20 PosX 0.02 PosY 0.02 }}\n\n",
            build_hex16,
        ));

        for dg in &atlas.domain_galaxies {
            se.push_str(&format!("Select \"{}\"\n", dg.galaxy_name));
            se.push_str("Goto { Time 8 DistRad 20 }\n");
            se.push_str("Wait 4\n");
            se.push_str(&format!(
                "Print \"Atlas: {} Domain ({} proofs)\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                dg.domain, dg.member_count,
            ));
            se.push_str(&format!(
                "Screenshot {{ Name \"toe_atlas_{}_\" Format \"png\" }}\n\n",
                dg.domain,
            ));
        }

        se.push_str("RestoreVars\n");
        se
    }

    fn emit_domain_tour_sat(galaxies: &[Galaxy], _build_hex16: &str, merkle_hex16: &str) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!("Log \"KernelTOE-SAT: {} proofs\"\n", galaxies.len()));
        se.push_str("Print \"SAT Domain Tour\" { Time 15 PosX 0.02 PosY 0.02 }\n\n");

        for gal in galaxies {
            let prefix = &gal.qid_hex[..8.min(gal.qid_hex.len())];
            let status = match gal.morphology {
                GalaxyMorphology::Spiral => "UNIQUE(SAT)",
                GalaxyMorphology::Elliptical => "UNSAT",
            };
            se.push_str(&format!("Select \"{}\"\n", gal.name));
            se.push_str("Goto { Time 6 DistRad 4 }\n");
            se.push_str("Wait 6\n");
            se.push_str(&format!(
                "Print \"MERKLE:{} Status:{}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex16, status,
            ));
            se.push_str(&format!(
                "Screenshot {{ Name \"toe_SAT_{}_\" Format \"png\" }}\n\n",
                prefix,
            ));
        }

        se.push_str("RestoreVars\n");
        se
    }

    fn emit_domain_tour_arith(stars: &[StarSystem], _build_hex16: &str, merkle_hex16: &str) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!("Log \"KernelTOE-Arith: {} proofs\"\n", stars.len()));
        se.push_str("Print \"Arith Domain Tour\" { Time 15 PosX 0.02 PosY 0.02 }\n\n");

        for star in stars {
            let prefix = &star.qid_hex[..8.min(star.qid_hex.len())];
            se.push_str(&format!("Select \"{}\"\n", star.name));
            se.push_str("Goto { Time 6 DistRad 4 }\n");
            se.push_str("Wait 6\n");
            se.push_str(&format!(
                "Print \"MERKLE:{} Status:UNIQUE(Arith)\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex16,
            ));
            se.push_str(&format!(
                "Screenshot {{ Name \"toe_Arith_{}_\" Format \"png\" }}\n\n",
                prefix,
            ));
        }

        se.push_str("RestoreVars\n");
        se
    }

    fn emit_domain_tour_table(nebulae: &[Nebula], _build_hex16: &str, merkle_hex16: &str) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!("Log \"KernelTOE-Table: {} proofs\"\n", nebulae.len()));
        se.push_str("Print \"Table Domain Tour\" { Time 15 PosX 0.02 PosY 0.02 }\n\n");

        for neb in nebulae {
            let prefix = &neb.qid_hex[..8.min(neb.qid_hex.len())];
            se.push_str(&format!("Select \"{}\"\n", neb.name));
            se.push_str("Goto { Time 6 DistRad 5 }\n");
            se.push_str("Wait 6\n");
            se.push_str(&format!(
                "Print \"MERKLE:{} Status:UNIQUE(Table)\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex16,
            ));
            se.push_str(&format!(
                "Screenshot {{ Name \"toe_Table_{}_\" Format \"png\" }}\n\n",
                prefix,
            ));
        }

        se.push_str("RestoreVars\n");
        se
    }

    fn emit_domain_tour_formal(dark_objects: &[DarkObject], lensing_proxies: &[LensingProxy], _build_hex16: &str, _merkle_hex16: &str) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!("Log \"KernelTOE-Formal: {} dark objects, {} lensing proxies\"\n",
            dark_objects.len(), lensing_proxies.len()));
        se.push_str("Print \"Formal Domain Tour (Dark Objects + Lensing)\" { Time 15 PosX 0.02 PosY 0.02 }\n\n");

        for lp in lensing_proxies {
            let prefix = &lp.qid_hex[..8.min(lp.qid_hex.len())];
            se.push_str(&format!("Select \"{}\"\n", lp.proxy_name));
            se.push_str("Goto { Time 6 DistRad 4 }\n");
            se.push_str("Wait 6\n");
            se.push_str(&format!(
                "Print \"Lensing proxy for DarkObject (mass={})\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                lp.lensing_mass.to_sc_decimal(3),
            ));
            se.push_str(&format!(
                "Screenshot {{ Name \"toe_Formal_{}_\" Format \"png\" }}\n\n",
                prefix,
            ));
        }

        // Note dark objects (invisible).
        for dark in dark_objects {
            se.push_str(&format!("// DarkObject \"{}\" (inadmissible - invisible)\n", dark.name));
        }

        se.push_str("RestoreVars\n");
        se
    }

    fn emit_domain_tour_dominate(clusters: &[StarCluster], _build_hex16: &str, merkle_hex16: &str) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!("Log \"KernelTOE-Dominate: {} proofs\"\n", clusters.len()));
        se.push_str("Print \"Dominate Domain Tour\" { Time 15 PosX 0.02 PosY 0.02 }\n\n");

        for cl in clusters {
            let prefix = &cl.qid_hex[..8.min(cl.qid_hex.len())];
            se.push_str(&format!("Select \"{}\"\n", cl.name));
            se.push_str("Goto { Time 6 DistRad 5 }\n");
            se.push_str("Wait 6\n");
            se.push_str(&format!(
                "Print \"MERKLE:{} Status:UNIQUE(Dominate)\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                merkle_hex16,
            ));
            se.push_str(&format!(
                "Screenshot {{ Name \"toe_Dominate_{}_\" Format \"png\" }}\n\n",
                prefix,
            ));
        }

        se.push_str("RestoreVars\n");
        se
    }

    fn emit_qid_deep_dive(
        contract: &Contract,
        qid_hex: &str,
        prefix: &str,
        sat_witnesses: &[SatWitnessMoons],
        unsat_witnesses: &[UnsatWitnessCluster],
        arith_witnesses: &[ArithWitnessPlanet],
        build_hex16: &str,
        _merkle_hex16: &str,
    ) -> String {
        let mut se = String::new();
        se.push_str("SaveVars\n");
        se.push_str(&format!("Log \"KernelTOE-Proof: QID={}\"\n", qid_hex));
        se.push_str(&format!(
            "Print \"Proof Deep Dive: {}\" {{ Time 15 PosX 0.02 PosY 0.02 }}\n\n",
            contract.description,
        ));

        // Navigate to the primary object.
        match &contract.eval {
            EvalSpec::BoolCnf { .. } => {
                let gal_name = format!("KG-{}", prefix);
                se.push_str(&format!("Select \"{}\"\n", gal_name));
                se.push_str("Goto { Time 6 DistRad 4 }\n");
                se.push_str("Wait 6\n");
                se.push_str(&format!(
                    "Print \"Contract: {}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                    contract.description,
                ));
                se.push_str(&format!(
                    "Screenshot {{ Name \"toe_proof_{}_overview_\" Format \"png\" }}\n\n",
                    prefix,
                ));

                // Zoom into witness moons if SAT.
                if let Some(sw) = sat_witnesses.iter().find(|w| w.qid_hex == qid_hex) {
                    for moon in &sw.moons {
                        let val = if moon.bit_value { "TRUE" } else { "FALSE" };
                        let incl = if moon.bit_value { "+45deg" } else { "-45deg" };
                        se.push_str(&format!(
                            "Select \"{parent}-Moon-{idx:03}\"\n",
                            parent = sw.parent_galaxy_name, idx = moon.moon_index,
                        ));
                        se.push_str("Goto { Time 4 DistRad 2 }\n");
                        se.push_str("Wait 4\n");
                        se.push_str(&format!(
                            "Print \"Var{}={} (incl={})\" {{ Time 8 PosX 0.02 PosY 0.06 }}\n",
                            moon.variable_index, val, incl,
                        ));
                        se.push_str(&format!(
                            "Screenshot {{ Name \"toe_proof_{}_moon{}_\" Format \"png\" }}\n\n",
                            prefix, moon.moon_index,
                        ));
                    }
                }

                // Or zoom into cluster if UNSAT.
                if let Some(uw) = unsat_witnesses.iter().find(|w| w.qid_hex == qid_hex) {
                    se.push_str(&format!("Select \"{}\"\n", uw.cluster_name));
                    se.push_str("Goto { Time 4 DistRad 3 }\n");
                    se.push_str("Wait 4\n");
                    se.push_str(&format!(
                        "Print \"UNSAT proof cluster ({} steps)\" {{ Time 8 PosX 0.02 PosY 0.06 }}\n",
                        uw.proof_steps.len(),
                    ));
                    se.push_str(&format!(
                        "Screenshot {{ Name \"toe_proof_{}_cluster_\" Format \"png\" }}\n\n",
                        prefix,
                    ));
                }
            }
            EvalSpec::ArithFind { .. } => {
                let star_name = format!("KS-{}", prefix);
                se.push_str(&format!("Select \"{}\"\n", star_name));
                se.push_str("Goto { Time 6 DistRad 4 }\n");
                se.push_str("Wait 6\n");
                se.push_str(&format!(
                    "Print \"Contract: {}\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                    contract.description,
                ));
                se.push_str(&format!(
                    "Screenshot {{ Name \"toe_proof_{}_overview_\" Format \"png\" }}\n\n",
                    prefix,
                ));

                if let Some(aw) = arith_witnesses.iter().find(|w| w.qid_hex == qid_hex) {
                    se.push_str(&format!("Select \"{}-Witness\"\n", aw.parent_star_name));
                    se.push_str("Goto { Time 4 DistRad 2 }\n");
                    se.push_str("Wait 4\n");
                    se.push_str(&format!(
                        "Print \"Witness: x={} period={}\" {{ Time 8 PosX 0.02 PosY 0.06 }}\n",
                        aw.witness_value, aw.period_exact.to_sc_decimal(3),
                    ));
                    se.push_str(&format!(
                        "Screenshot {{ Name \"toe_proof_{}_witness_\" Format \"png\" }}\n\n",
                        prefix,
                    ));
                }
            }
            _ => {
                // Generic overview for other types.
                se.push_str(&format!(
                    "Print \"Contract: {} (BuildHash={})\" {{ Time 10 PosX 0.02 PosY 0.06 }}\n",
                    contract.description, build_hex16,
                ));
            }
        }

        se.push_str("RestoreVars\n");
        se
    }

    // ── Metadata builders ──

    fn build_witness_index(
        contracts: &[Contract],
        files: &BTreeMap<String, Vec<u8>>,
        sat_witnesses: &[SatWitnessMoons],
        unsat_witnesses: &[UnsatWitnessCluster],
        arith_witnesses: &[ArithWitnessPlanet],
        lensing_proxies: &[LensingProxy],
    ) -> Vec<WitnessIndexEntry> {
        let mut index = Vec::new();

        for contract in contracts {
            let qid_hex = hash::hex(&contract.qid);
            let prefix = &qid_hex[..8.min(qid_hex.len())];
            let mut object_names = Vec::new();
            let mut file_paths = Vec::new();
            let domain = crate::atlas_builder::AtlasBuilder::classify_domain(&contract.eval);

            // Find all files for this QID prefix.
            for key in files.keys() {
                if key.contains(prefix) {
                    file_paths.push(key.clone());
                }
            }

            // Collect object names.
            match &contract.eval {
                EvalSpec::BoolCnf { .. } => {
                    object_names.push(format!("KG-{}", prefix));
                    if let Some(sw) = sat_witnesses.iter().find(|w| w.qid_hex == qid_hex) {
                        for moon in &sw.moons {
                            object_names.push(format!("{}-Moon-{:03}", sw.parent_galaxy_name, moon.moon_index));
                        }
                    }
                    if let Some(uw) = unsat_witnesses.iter().find(|w| w.qid_hex == qid_hex) {
                        object_names.push(uw.cluster_name.clone());
                    }
                }
                EvalSpec::ArithFind { .. } => {
                    object_names.push(format!("KS-{}", prefix));
                    if let Some(aw) = arith_witnesses.iter().find(|w| w.qid_hex == qid_hex) {
                        object_names.push(format!("{}-Witness", aw.parent_star_name));
                        object_names.push(format!("{}-Decoy", aw.parent_star_name));
                    }
                }
                EvalSpec::Table(_) => {
                    object_names.push(format!("KN-{}", prefix));
                }
                EvalSpec::FormalProof { .. } => {
                    object_names.push(format!("KD-{}", prefix));
                    if let Some(lp) = lensing_proxies.iter().find(|l| l.qid_hex == qid_hex) {
                        object_names.push(lp.proxy_name.clone());
                    }
                }
                EvalSpec::Dominate { .. } => {
                    object_names.push(format!("KC-{}", prefix));
                }
                EvalSpec::SpaceEngine { .. } => {}
                EvalSpec::MillenniumFinite { .. } => {
                    object_names.push(format!("KS-{}", prefix));
                }
            }

            // Compute witness hash from all file contents for this QID.
            let mut hash_buf = Vec::new();
            for path in &file_paths {
                if let Some(bytes) = files.get(path) {
                    hash_buf.extend_from_slice(bytes);
                }
            }
            let witness_hash = hash::hex(&hash::H(&hash_buf));

            file_paths.sort();

            index.push(WitnessIndexEntry {
                qid_hex: qid_hex.clone(),
                object_names,
                file_paths,
                witness_hash,
                domain: format!("{}", domain),
            });
        }

        index
    }

    fn build_enhanced_manifest(
        catalog: &KernelCatalog,
        atlas: &AtlasCluster,
        sat_witnesses: &[SatWitnessMoons],
        unsat_witnesses: &[UnsatWitnessCluster],
        arith_witnesses: &[ArithWitnessPlanet],
        lensing_proxies: &[LensingProxy],
        build_hash: &Hash32,
        merkle_root: &Hash32,
    ) -> serde_json::Value {
        serde_json::json!({
            "version": "0.2.0",
            "kernel_build_hash": hash::hex(build_hash),
            "catalog_merkle_root": hash::hex(merkle_root),
            "star_count": catalog.stars.len(),
            "galaxy_count": catalog.galaxies.len(),
            "nebula_count": catalog.nebulae.len(),
            "dark_object_count": catalog.dark_objects.len(),
            "cluster_count": catalog.clusters.len(),
            "witness_moon_count": sat_witnesses.iter().map(|w| w.moons.len()).sum::<usize>(),
            "witness_cluster_count": unsat_witnesses.len(),
            "witness_planet_count": arith_witnesses.len(),
            "lensing_proxy_count": lensing_proxies.len(),
            "filament_count": atlas.filaments.len(),
            "frontier_count": atlas.frontiers.len(),
            "atlas_hash": hash::hex(&atlas.atlas_hash),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_solver::Solver;
    use kernel_contracts::compiler::compile_contract;
    use kernel_types::HASH_ZERO;
    use crate::catalog::CatalogGenerator;
    use crate::atlas_builder::AtlasBuilder;
    use crate::witness_encoder::WitnessEncoder;

    fn make_full_test_data() -> (
        Vec<Contract>, Vec<SolveOutput>, KernelCatalog,
        Vec<SatWitnessMoons>, Vec<UnsatWitnessCluster>, Vec<ArithWitnessPlanet>,
        Vec<LensingProxy>, AtlasCluster,
    ) {
        let specs = vec![
            r#"{"type":"arith_find","description":"star","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#,
            r#"{"type":"bool_cnf","description":"sat galaxy","num_vars":2,"clauses":[[1,2]]}"#,
            r#"{"type":"bool_cnf","description":"unsat galaxy","num_vars":1,"clauses":[[1],[-1]]}"#,
            r#"{"type":"table","description":"nebula","entries":[{"key":"a","value":"SAT"}]}"#,
        ];
        let mut contracts = Vec::new();
        let mut outputs = Vec::new();
        for spec in specs {
            let c = compile_contract(spec).unwrap();
            let mut solver = Solver::new();
            let o = solver.solve(&c);
            contracts.push(c);
            outputs.push(o);
        }

        let mut ledger = Ledger::new();
        let catalog = CatalogGenerator::generate(&contracts, &outputs, HASH_ZERO, &mut ledger);

        let mut sat_witnesses = Vec::new();
        let mut unsat_witnesses = Vec::new();
        let mut arith_witnesses = Vec::new();

        for (c, o) in contracts.iter().zip(outputs.iter()) {
            match &c.eval {
                EvalSpec::BoolCnf { .. } => {
                    let prefix = &hash::hex(&c.qid)[..8];
                    let galaxy_name = format!("KG-{}", prefix);
                    if o.status == kernel_types::Status::Unique {
                        if let Some(sw) = WitnessEncoder::encode_sat_witness(c, o, &galaxy_name, &mut ledger) {
                            sat_witnesses.push(sw);
                        }
                    } else {
                        if let Some(uw) = WitnessEncoder::encode_unsat_witness(c, o, &galaxy_name, &mut ledger) {
                            unsat_witnesses.push(uw);
                        }
                    }
                }
                EvalSpec::ArithFind { .. } => {
                    let prefix = &hash::hex(&c.qid)[..8];
                    let star_name = format!("KS-{}", prefix);
                    if let Some(aw) = WitnessEncoder::encode_arith_witness(c, o, &star_name, &mut ledger) {
                        arith_witnesses.push(aw);
                    }
                }
                _ => {}
            }
        }

        let lensing = WitnessEncoder::encode_dark_lensing(&catalog.dark_objects, &mut ledger);
        let atlas = AtlasBuilder::build(&contracts, &outputs, &catalog.merkle_root, &mut ledger);

        (contracts, outputs, catalog, sat_witnesses, unsat_witnesses, arith_witnesses, lensing, atlas)
    }

    #[test]
    fn per_qid_galaxy_files_exist() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let build_hash = HASH_ZERO;
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &build_hash, &catalog.merkle_root, &mut ledger,
        );
        for gal in &catalog.galaxies {
            let prefix = &gal.qid_hex[..8];
            let key = format!("catalogs/galaxies/KG-{}.sc", prefix);
            assert!(files.contains_key(&key), "Missing galaxy file: {}", key);
        }
    }

    #[test]
    fn atlas_domains_sc_exists() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        assert!(files.contains_key("catalogs/galaxies/AtlasDomains.sc"));
    }

    #[test]
    fn sat_witness_moon_files() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        for witness in &sw {
            let prefix = &witness.qid_hex[..8];
            let key = format!("catalogs/planets/KP-{}.sc", prefix);
            assert!(files.contains_key(&key), "Missing SAT moon file: {}", key);
        }
    }

    #[test]
    fn unsat_cluster_files() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        for witness in &uw {
            let prefix = &witness.qid_hex[..8];
            let key = format!("catalogs/clusters/KC-{}.sc", prefix);
            assert!(files.contains_key(&key), "Missing UNSAT cluster file: {}", key);
        }
    }

    #[test]
    fn arith_planet_files() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        for witness in &aw {
            let prefix = &witness.qid_hex[..8];
            let key = format!("catalogs/planets/KP-{}.sc", prefix);
            assert!(files.contains_key(&key), "Missing ArithFind planet file: {}", key);
        }
    }

    #[test]
    fn filament_files() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        for fil in &atlas.filaments {
            let key = format!("catalogs/nebulae/{}.sc", fil.nebula_name);
            assert!(files.contains_key(&key), "Missing filament file: {}", key);
        }
    }

    #[test]
    fn lensing_proxy_files() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        for proxy in &lp {
            let prefix = &proxy.qid_hex[..8];
            let key = format!("catalogs/stars/KLens-{}.sc", prefix);
            assert!(files.contains_key(&key), "Missing lensing proxy file: {}", key);
        }
    }

    #[test]
    fn atlas_tour_script() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        assert!(files.contains_key("scripts/toe_proof_atlas.se"));
        let script = String::from_utf8_lossy(files.get("scripts/toe_proof_atlas.se").unwrap());
        assert!(script.contains("KernelTOE-Full"));
        assert!(script.contains("SaveVars"));
        assert!(script.contains("RestoreVars"));
    }

    #[test]
    fn domain_tour_scripts() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        assert!(files.contains_key("scripts/toe_proof_SAT.se"));
        assert!(files.contains_key("scripts/toe_proof_Arith.se"));
        assert!(files.contains_key("scripts/toe_proof_Table.se"));
        assert!(files.contains_key("scripts/toe_proof_Formal.se"));
        assert!(files.contains_key("scripts/toe_proof_Dominate.se"));
        assert!(files.contains_key("scripts/toe_proof_SpaceEngine.se"));
    }

    #[test]
    fn witness_index_json_valid() {
        let (contracts, outputs, catalog, sw, uw, aw, lp, atlas) = make_full_test_data();
        let mut ledger = Ledger::new();
        let files = EnhancedEmitter::emit_all(
            &catalog, &contracts, &outputs, &atlas,
            &sw, &uw, &aw, &lp, &HASH_ZERO, &catalog.merkle_root, &mut ledger,
        );
        assert!(files.contains_key("proof/witness_index.json"));
        let json_bytes = files.get("proof/witness_index.json").unwrap();
        let index: Vec<WitnessIndexEntry> = serde_json::from_slice(json_bytes).unwrap();
        assert_eq!(index.len(), contracts.len());
        for entry in &index {
            assert!(!entry.qid_hex.is_empty());
            assert!(!entry.witness_hash.is_empty());
        }
    }
}
