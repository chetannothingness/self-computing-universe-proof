use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::{Contract, EvalSpec};
use kernel_ledger::{Event, EventKind, Ledger};
use crate::types::*;
use std::collections::BTreeMap;

/// Generates a KernelCatalog from solved contracts.
pub struct CatalogGenerator;

impl CatalogGenerator {
    /// Generate the full kernel-derived catalog from contracts and their solve outputs.
    pub fn generate(
        contracts: &[Contract],
        outputs: &[SolveOutput],
        kernel_build_hash: Hash32,
        ledger: &mut Ledger,
    ) -> KernelCatalog {
        let mut stars = Vec::new();
        let mut galaxies = Vec::new();
        let mut nebulae = Vec::new();
        let mut dark_objects = Vec::new();
        let mut clusters = Vec::new();

        for (contract, output) in contracts.iter().zip(outputs.iter()) {
            let qid = contract.qid;
            let qid_hex = hash::hex(&qid);
            let name_prefix = &qid_hex[..8.min(qid_hex.len())];
            let (cx, cy, cz) = coords_from_qid(&qid);

            match &contract.eval {
                EvalSpec::ArithFind { coefficients, target } => {
                    let spectral = (*target as u8) % 7;
                    let luminosity = Rational::new(
                        coefficients.first().copied().unwrap_or(1).wrapping_abs(),
                        (coefficients.len() as u64).max(1),
                    );
                    let planet_orbits: Vec<Rational> = coefficients.iter()
                        .map(|c| Rational::new(c.wrapping_abs(), 10))
                        .collect();
                    stars.push(StarSystem {
                        qid_hex: qid_hex.clone(),
                        name: format!("KS-{}", name_prefix),
                        coord_x: cx, coord_y: cy, coord_z: cz,
                        spectral_class: spectral,
                        luminosity,
                        planet_orbits,
                        contract_hash: qid,
                    });
                }
                EvalSpec::BoolCnf { num_vars, clauses } => {
                    let is_sat = output.status == kernel_types::Status::Unique;
                    let morphology = if is_sat {
                        GalaxyMorphology::Spiral
                    } else {
                        GalaxyMorphology::Elliptical
                    };
                    galaxies.push(Galaxy {
                        qid_hex: qid_hex.clone(),
                        name: format!("KG-{}", name_prefix),
                        coord_x: cx, coord_y: cy, coord_z: cz,
                        arm_count: (*num_vars as u32).max(2),
                        radius_kpc: Rational::new(clauses.len() as i64, 10),
                        morphology,
                        contract_hash: qid,
                    });
                }
                EvalSpec::Table(entries) => {
                    nebulae.push(Nebula {
                        qid_hex: qid_hex.clone(),
                        name: format!("KN-{}", name_prefix),
                        coord_x: cx, coord_y: cy, coord_z: cz,
                        radius_ly: Rational::new(entries.len() as i64, 1),
                        density: Rational::new(1, (entries.len() as u64).max(1)),
                        contract_hash: qid,
                    });
                }
                EvalSpec::FormalProof { statement, .. } => {
                    dark_objects.push(DarkObject {
                        qid_hex: qid_hex.clone(),
                        name: format!("KD-{}", name_prefix),
                        coord_x: cx, coord_y: cy, coord_z: cz,
                        mass_estimate: Rational::new(i64::MAX / 2, 1),
                        reason_inadmissible: statement.clone(),
                        contract_hash: qid,
                    });
                }
                EvalSpec::Dominate { competitor_id, .. } => {
                    clusters.push(StarCluster {
                        qid_hex: qid_hex.clone(),
                        name: format!("KC-{}", name_prefix),
                        coord_x: cx, coord_y: cy, coord_z: cz,
                        member_count: 100,
                        radius_pc: Rational::new(
                            competitor_id.len() as i64,
                            1,
                        ),
                        contract_hash: qid,
                    });
                }
                EvalSpec::SpaceEngine { .. } => {
                    // Meta-contract — skip catalog generation.
                }
            }
        }

        // Compute Merkle root of all objects' SerPi.
        let mut file_hashes = Vec::new();
        for s in &stars { file_hashes.push(s.ser_pi_hash()); }
        for g in &galaxies { file_hashes.push(g.ser_pi_hash()); }
        for n in &nebulae { file_hashes.push(n.ser_pi_hash()); }
        for d in &dark_objects { file_hashes.push(d.ser_pi_hash()); }
        for c in &clusters { file_hashes.push(c.ser_pi_hash()); }
        let merkle_root = hash::merkle_root(&file_hashes);

        // Emit ledger event per category.
        let categories = ["stars", "galaxies", "nebulae", "dark_objects", "clusters"];
        let counts = [stars.len(), galaxies.len(), nebulae.len(), dark_objects.len(), clusters.len()];
        for (cat, count) in categories.iter().zip(counts.iter()) {
            if *count > 0 {
                let payload = canonical_cbor_bytes(&(cat, *count as u64, &merkle_root.to_vec()));
                ledger.commit(Event::new(
                    EventKind::SpaceEngineCatalogEmit,
                    &payload,
                    vec![],
                    1,
                    1,
                ));
            }
        }

        KernelCatalog {
            stars,
            galaxies,
            nebulae,
            dark_objects,
            clusters,
            merkle_root,
            kernel_build_hash,
        }
    }

    /// Emit .sc files from a kernel catalog. BTreeMap for deterministic ordering.
    pub fn emit_sc_files(catalog: &KernelCatalog) -> BTreeMap<String, Vec<u8>> {
        let mut files = BTreeMap::new();

        // Stars
        if !catalog.stars.is_empty() {
            let mut sc = String::new();
            for star in &catalog.stars {
                sc.push_str(&format!("Star \"{}\"\n{{\n", star.name));
                sc.push_str(&format!("    RA  {}\n", write_integer_as_decimal(star.coord_x % 360_000, 3)));
                sc.push_str(&format!("    Dec {}\n", write_integer_as_decimal(star.coord_y % 90_000, 3)));
                sc.push_str(&format!("    Dist {}\n", write_integer_as_decimal(star.coord_z.unsigned_abs() as i64 % 100_000, 3)));
                sc.push_str(&format!("    Class \"{}\"\n", spectral_class_name(star.spectral_class)));
                sc.push_str(&format!("    Lum {}\n", star.luminosity.to_sc_decimal(3)));
                for (i, orbit) in star.planet_orbits.iter().enumerate() {
                    sc.push_str(&format!("    // Planet {} orbit: {} AU\n", i, orbit.to_sc_decimal(3)));
                }
                sc.push_str("}\n\n");
            }
            files.insert("catalogs/stars/kernel_stars.sc".into(), sc.into_bytes());
        }

        // Galaxies
        if !catalog.galaxies.is_empty() {
            let mut sc = String::new();
            for gal in &catalog.galaxies {
                sc.push_str(&format!("Galaxy \"{}\"\n{{\n", gal.name));
                sc.push_str(&format!("    RA  {}\n", write_integer_as_decimal(gal.coord_x % 360_000, 3)));
                sc.push_str(&format!("    Dec {}\n", write_integer_as_decimal(gal.coord_y % 90_000, 3)));
                sc.push_str(&format!("    Dist {}\n", write_integer_as_decimal(gal.coord_z.unsigned_abs() as i64 % 10_000_000, 3)));
                let morph = match gal.morphology {
                    GalaxyMorphology::Spiral => "S",
                    GalaxyMorphology::Elliptical => "E",
                };
                sc.push_str(&format!("    Type \"{}\"\n", morph));
                sc.push_str(&format!("    Radius {}\n", gal.radius_kpc.to_sc_decimal(3)));
                sc.push_str("}\n\n");
            }
            files.insert("catalogs/galaxies/kernel_galaxies.sc".into(), sc.into_bytes());
        }

        // Nebulae
        if !catalog.nebulae.is_empty() {
            let mut sc = String::new();
            for neb in &catalog.nebulae {
                sc.push_str(&format!("Nebula \"{}\"\n{{\n", neb.name));
                sc.push_str(&format!("    RA  {}\n", write_integer_as_decimal(neb.coord_x % 360_000, 3)));
                sc.push_str(&format!("    Dec {}\n", write_integer_as_decimal(neb.coord_y % 90_000, 3)));
                sc.push_str(&format!("    Dist {}\n", write_integer_as_decimal(neb.coord_z.unsigned_abs() as i64 % 100_000, 3)));
                sc.push_str(&format!("    Radius {}\n", neb.radius_ly.to_sc_decimal(3)));
                sc.push_str("}\n\n");
            }
            files.insert("catalogs/nebulae/kernel_nebulae.sc".into(), sc.into_bytes());
        }

        // Clusters
        if !catalog.clusters.is_empty() {
            let mut sc = String::new();
            for cl in &catalog.clusters {
                sc.push_str(&format!("Cluster \"{}\"\n{{\n", cl.name));
                sc.push_str(&format!("    RA  {}\n", write_integer_as_decimal(cl.coord_x % 360_000, 3)));
                sc.push_str(&format!("    Dec {}\n", write_integer_as_decimal(cl.coord_y % 90_000, 3)));
                sc.push_str(&format!("    Dist {}\n", write_integer_as_decimal(cl.coord_z.unsigned_abs() as i64 % 100_000, 3)));
                sc.push_str(&format!("    NStars {}\n", cl.member_count));
                sc.push_str("}\n\n");
            }
            files.insert("catalogs/clusters/kernel_clusters.sc".into(), sc.into_bytes());
        }

        // Dark objects are NOT emitted as .sc — they are invisible by definition.
        // Their presence is recorded in the Merkle root only.

        files
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_solver::Solver;
    use kernel_contracts::compiler::compile_contract;

    fn make_test_contracts() -> (Vec<Contract>, Vec<SolveOutput>) {
        let specs = vec![
            r#"{"type":"arith_find","description":"star test","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#,
            r#"{"type":"bool_cnf","description":"galaxy test","num_vars":2,"clauses":[[1],[2]]}"#,
            r#"{"type":"table","description":"nebula test","entries":[{"key":"a","value":"SAT"}]}"#,
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
        (contracts, outputs)
    }

    #[test]
    fn catalog_deterministic() {
        let (contracts, outputs) = make_test_contracts();
        let mut ledger1 = Ledger::new();
        let mut ledger2 = Ledger::new();
        let cat1 = CatalogGenerator::generate(&contracts, &outputs, HASH_ZERO, &mut ledger1);
        let cat2 = CatalogGenerator::generate(&contracts, &outputs, HASH_ZERO, &mut ledger2);
        assert_eq!(cat1.merkle_root, cat2.merkle_root);
    }

    #[test]
    fn arith_find_maps_to_star() {
        let c = compile_contract(
            r#"{"type":"arith_find","description":"star","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let cat = CatalogGenerator::generate(&[c], &[o], HASH_ZERO, &mut ledger);
        assert_eq!(cat.stars.len(), 1);
        assert!(cat.stars[0].name.starts_with("KS-"));
    }

    #[test]
    fn bool_cnf_sat_spiral() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"sat","num_vars":1,"clauses":[[1]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let cat = CatalogGenerator::generate(&[c], &[o], HASH_ZERO, &mut ledger);
        assert_eq!(cat.galaxies.len(), 1);
    }

    #[test]
    fn bool_cnf_unsat_elliptical() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"unsat","num_vars":1,"clauses":[[1],[-1]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let cat = CatalogGenerator::generate(&[c], &[o], HASH_ZERO, &mut ledger);
        assert_eq!(cat.galaxies.len(), 1);
        assert_eq!(cat.galaxies[0].morphology, GalaxyMorphology::Elliptical);
    }

    #[test]
    fn sc_files_deterministic() {
        let (contracts, outputs) = make_test_contracts();
        let mut ledger = Ledger::new();
        let cat = CatalogGenerator::generate(&contracts, &outputs, HASH_ZERO, &mut ledger);
        let files1 = CatalogGenerator::emit_sc_files(&cat);
        let files2 = CatalogGenerator::emit_sc_files(&cat);
        assert_eq!(files1.len(), files2.len());
        for (k, v1) in &files1 {
            assert_eq!(v1, files2.get(k).unwrap());
        }
    }
}
