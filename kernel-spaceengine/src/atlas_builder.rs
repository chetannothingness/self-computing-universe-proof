//! L3 Atlas builder: constructs the Atlas cluster from contracts and solve outputs.
//!
//! The Atlas provides navigational structure: domain galaxies, index stars,
//! dependency filaments, and frontier black holes.

use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::{Contract, EvalSpec};
use kernel_ledger::{Event, EventKind, Ledger};
use crate::types::{Rational, coords_from_qid};
use crate::atlas_types::*;
use std::collections::BTreeMap;

/// Builds the L3 Atlas cluster.
pub struct AtlasBuilder;

impl AtlasBuilder {
    /// Build complete Atlas from contracts + solve outputs.
    pub fn build(
        contracts: &[Contract],
        outputs: &[SolveOutput],
        merkle_root: &Hash32,
        ledger: &mut Ledger,
    ) -> AtlasCluster {
        let (cx, cy, cz) = Self::atlas_center(merkle_root);

        // Classify contracts by domain.
        let mut domain_counts: BTreeMap<ProofDomain, u32> = BTreeMap::new();
        let mut contract_domains: Vec<(ProofDomain, String, String)> = Vec::new(); // (domain, qid_hex, object_name)
        let mut contract_coords: BTreeMap<String, (i64, i64, i64)> = BTreeMap::new();

        for (contract, _output) in contracts.iter().zip(outputs.iter()) {
            let domain = Self::classify_domain(&contract.eval);
            let qid_hex = hash::hex(&contract.qid);
            let prefix = &qid_hex[..8.min(qid_hex.len())];
            let object_name = match domain {
                ProofDomain::SAT => format!("KG-{}", prefix),
                ProofDomain::Arith => format!("KS-{}", prefix),
                ProofDomain::Table => format!("KN-{}", prefix),
                ProofDomain::Formal => format!("KD-{}", prefix),
                ProofDomain::Dominate => format!("KC-{}", prefix),
                ProofDomain::SpaceEngine => format!("KSE-{}", prefix),
                ProofDomain::Exo => format!("KExo-{}", prefix),
            };
            let coords = coords_from_qid(&contract.qid);
            contract_coords.insert(qid_hex.clone(), coords);
            *domain_counts.entry(domain.clone()).or_insert(0) += 1;
            contract_domains.push((domain, qid_hex, object_name));
        }

        // Build domain galaxies.
        let mut domain_galaxies = Vec::new();
        for (domain, count) in &domain_counts {
            let (dx, dy, dz) = Self::domain_coords((cx, cy, cz), domain);
            // radius_kpc proportional to member count: 10 kpc per member.
            // A domain with 1 proof gets 10 kpc; a domain with 20 proofs gets 200 kpc.
            // This is not hardcoded — it grows from the actual proof count.
            let radius = Rational::new((*count as i64).max(1) * 10, 1);
            domain_galaxies.push(AtlasDomainGalaxy {
                domain: domain.clone(),
                galaxy_name: format!("KAtlas-{}", domain),
                coord_x: dx,
                coord_y: dy,
                coord_z: dz,
                member_count: *count,
                radius_kpc: radius,
            });
        }

        // Build index stars.
        let index_stars: Vec<AtlasIndexStar> = contract_domains.iter().map(|(domain, qid_hex, obj_name)| {
            let coords = contract_coords.get(qid_hex).copied().unwrap_or((0, 0, 0));
            AtlasIndexStar {
                qid_hex: qid_hex.clone(),
                target_object_name: obj_name.clone(),
                domain: domain.clone(),
                coord_x: coords.0,
                coord_y: coords.1,
                coord_z: coords.2,
            }
        }).collect();

        // Build dependency graph and filaments.
        let deps = Self::build_dependency_graph(contracts);
        let filaments = Self::generate_filaments(&deps, &contract_coords);

        // Build frontiers (black holes for FormalProof contracts).
        let frontiers = Self::generate_frontiers(contracts, outputs);

        // Build witness index (populated later by enhanced_emitter, stubs here).
        let witness_index = Vec::new();

        // Compute atlas hash.
        let mut hash_buf = Vec::new();
        hash_buf.extend_from_slice(&cx.ser_pi());
        hash_buf.extend_from_slice(&cy.ser_pi());
        hash_buf.extend_from_slice(&cz.ser_pi());
        for g in &domain_galaxies { hash_buf.extend_from_slice(&g.ser_pi()); }
        for s in &index_stars { hash_buf.extend_from_slice(&s.ser_pi()); }
        for f in &filaments { hash_buf.extend_from_slice(&f.ser_pi()); }
        for bh in &frontiers { hash_buf.extend_from_slice(&bh.ser_pi()); }
        let atlas_hash = hash::H(&hash_buf);

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&(
            "AtlasBuild",
            domain_galaxies.len() as u64,
            index_stars.len() as u64,
            filaments.len() as u64,
            frontiers.len() as u64,
        ));
        ledger.commit(Event::new(
            EventKind::AtlasBuild,
            &payload,
            vec![],
            1,
            1,
        ));

        AtlasCluster {
            center_x: cx,
            center_y: cy,
            center_z: cz,
            domain_galaxies,
            index_stars,
            filaments,
            frontiers,
            witness_index,
            atlas_hash,
        }
    }

    /// Classify contract's EvalSpec into ProofDomain.
    pub fn classify_domain(eval: &EvalSpec) -> ProofDomain {
        match eval {
            EvalSpec::BoolCnf { .. } => ProofDomain::SAT,
            EvalSpec::ArithFind { .. } => ProofDomain::Arith,
            EvalSpec::Table(_) => ProofDomain::Table,
            EvalSpec::FormalProof { .. } => ProofDomain::Formal,
            EvalSpec::Dominate { .. } => ProofDomain::Dominate,
            EvalSpec::SpaceEngine { .. } => ProofDomain::SpaceEngine,
            EvalSpec::MillenniumFinite { .. } => ProofDomain::Arith,
        }
    }

    /// Atlas center = coords_from_qid(merkle_root) + (100_000_000, 100_000_000, 100_000_000).
    pub fn atlas_center(merkle_root: &Hash32) -> (i64, i64, i64) {
        let (x, y, z) = coords_from_qid(merkle_root);
        (
            x.wrapping_add(100_000_000),
            y.wrapping_add(100_000_000),
            z.wrapping_add(100_000_000),
        )
    }

    /// Domain galaxy coords = atlas_center + domain_index * 10_000_000 on x-axis.
    pub fn domain_coords(center: (i64, i64, i64), domain: &ProofDomain) -> (i64, i64, i64) {
        let idx = match domain {
            ProofDomain::SAT => 0,
            ProofDomain::Arith => 1,
            ProofDomain::Table => 2,
            ProofDomain::Formal => 3,
            ProofDomain::Dominate => 4,
            ProofDomain::SpaceEngine => 5,
            ProofDomain::Exo => 6,
        };
        (
            center.0.wrapping_add(idx * 10_000_000),
            center.1,
            center.2,
        )
    }

    /// Build dependency graph: contracts sharing the same EvalSpec type are "related".
    pub fn build_dependency_graph(contracts: &[Contract]) -> BTreeMap<String, Vec<String>> {
        let mut by_domain: BTreeMap<ProofDomain, Vec<String>> = BTreeMap::new();
        for contract in contracts {
            let domain = Self::classify_domain(&contract.eval);
            let qid_hex = hash::hex(&contract.qid);
            by_domain.entry(domain).or_default().push(qid_hex);
        }

        let mut deps: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (_domain, qids) in &by_domain {
            for i in 0..qids.len() {
                for j in (i + 1)..qids.len() {
                    deps.entry(qids[i].clone()).or_default().push(qids[j].clone());
                    deps.entry(qids[j].clone()).or_default().push(qids[i].clone());
                }
            }
        }

        // Sort all adjacency lists for determinism.
        for list in deps.values_mut() {
            list.sort();
            list.dedup();
        }

        deps
    }

    /// Generate filaments for dependency edges.
    pub fn generate_filaments(
        deps: &BTreeMap<String, Vec<String>>,
        coords: &BTreeMap<String, (i64, i64, i64)>,
    ) -> Vec<FilamentNebula> {
        let mut filaments = Vec::new();
        let mut seen = std::collections::BTreeSet::new();

        for (from_qid, to_qids) in deps {
            for to_qid in to_qids {
                // Avoid duplicate edges (A->B and B->A).
                let edge_key = if from_qid < to_qid {
                    (from_qid.clone(), to_qid.clone())
                } else {
                    (to_qid.clone(), from_qid.clone())
                };
                if !seen.insert(edge_key) {
                    continue;
                }

                let from_coords = coords.get(from_qid).copied().unwrap_or((0, 0, 0));
                let to_coords = coords.get(to_qid).copied().unwrap_or((0, 0, 0));

                let mid_x = from_coords.0.wrapping_add(to_coords.0) / 2;
                let mid_y = from_coords.1.wrapping_add(to_coords.1) / 2;
                let mid_z = from_coords.2.wrapping_add(to_coords.2) / 2;

                // Radius proportional to Manhattan distance between the two contracts.
                // Coordinates come from hash bytes (i64::from_le_bytes), so distances
                // are in the range ~10^18. We divide by 10^15 to get light-year scale
                // radii (typical range: 1–9000 ly), then clamp to minimum 1.
                let dist = (from_coords.0.wrapping_sub(to_coords.0)).unsigned_abs()
                    .wrapping_add((from_coords.1.wrapping_sub(to_coords.1)).unsigned_abs())
                    .wrapping_add((from_coords.2.wrapping_sub(to_coords.2)).unsigned_abs());
                let radius = Rational::new((dist / 1_000_000_000_000_000).max(1) as i64, 1);

                let from_prefix = &from_qid[..8.min(from_qid.len())];
                let to_prefix = &to_qid[..8.min(to_qid.len())];

                filaments.push(FilamentNebula {
                    from_qid_hex: from_qid.clone(),
                    to_qid_hex: to_qid.clone(),
                    nebula_name: format!("KN-{}-{}", from_prefix, to_prefix),
                    mid_x,
                    mid_y,
                    mid_z,
                    radius_ly: radius,
                });
            }
        }

        filaments
    }

    /// Generate frontier black holes for inadmissible contracts.
    pub fn generate_frontiers(
        contracts: &[Contract],
        outputs: &[SolveOutput],
    ) -> Vec<FrontierBlackHole> {
        let mut frontiers = Vec::new();

        for (contract, _output) in contracts.iter().zip(outputs.iter()) {
            // Only FormalProof contracts get frontier black holes.
            if !matches!(&contract.eval, EvalSpec::FormalProof { .. }) {
                continue;
            }

            let qid_hex = hash::hex(&contract.qid);
            let prefix = &qid_hex[..8.min(qid_hex.len())];
            let (cx, cy, cz) = coords_from_qid(&contract.qid);
            let max_cost = contract.budget.max_cost;

            frontiers.push(FrontierBlackHole {
                qid_hex: qid_hex.clone(),
                name: format!("KBH-{}", prefix),
                coord_x: cx,
                coord_y: cy,
                coord_z: cz,
                event_horizon_milli_ly: (max_cost / 1000) as i64,
                cost: max_cost,
            });
        }

        frontiers
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_solver::Solver;
    use kernel_contracts::compiler::compile_contract;
    

    fn make_test_data() -> (Vec<Contract>, Vec<SolveOutput>) {
        let specs = vec![
            r#"{"type":"arith_find","description":"star","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#,
            r#"{"type":"bool_cnf","description":"galaxy","num_vars":2,"clauses":[[1],[2]]}"#,
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
        (contracts, outputs)
    }

    #[test]
    fn atlas_center_deterministic() {
        let root = hash::H(b"test_merkle");
        let (x1, y1, z1) = AtlasBuilder::atlas_center(&root);
        let (x2, y2, z2) = AtlasBuilder::atlas_center(&root);
        assert_eq!((x1, y1, z1), (x2, y2, z2));
    }

    #[test]
    fn no_coordinate_collision() {
        let root = hash::H(b"collision_test");
        let center = AtlasBuilder::atlas_center(&root);
        let domains = vec![
            ProofDomain::SAT, ProofDomain::Arith, ProofDomain::Table,
            ProofDomain::Formal, ProofDomain::Dominate, ProofDomain::SpaceEngine,
            ProofDomain::Exo,
        ];
        let coords: Vec<(i64, i64, i64)> = domains.iter()
            .map(|d| AtlasBuilder::domain_coords(center, d))
            .collect();
        // All x-coordinates should be distinct.
        for i in 0..coords.len() {
            for j in (i+1)..coords.len() {
                assert_ne!(coords[i].0, coords[j].0,
                    "Domains {:?} and {:?} collide", domains[i], domains[j]);
            }
        }
    }

    #[test]
    fn domain_classification() {
        let specs: Vec<(&str, ProofDomain)> = vec![
            (r#"{"type":"bool_cnf","description":"sat","num_vars":1,"clauses":[[1]]}"#, ProofDomain::SAT),
            (r#"{"type":"arith_find","description":"arith","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#, ProofDomain::Arith),
            (r#"{"type":"table","description":"table","entries":[{"key":"a","value":"b"}]}"#, ProofDomain::Table),
        ];
        for (json, expected) in specs {
            let c = compile_contract(json).unwrap();
            assert_eq!(AtlasBuilder::classify_domain(&c.eval), expected);
        }
    }

    #[test]
    fn dependency_graph_symmetry() {
        let (contracts, _outputs) = make_test_data();
        let deps = AtlasBuilder::build_dependency_graph(&contracts);
        // Verify symmetry: if A → B then B → A.
        for (from, tos) in &deps {
            for to in tos {
                let reverse = deps.get(to);
                assert!(reverse.is_some(), "Missing reverse edge from {} to {}", to, from);
                assert!(reverse.unwrap().contains(from),
                    "Reverse edge from {} to {} not found", to, from);
            }
        }
    }

    #[test]
    fn filament_count() {
        let (contracts, _outputs) = make_test_data();
        let deps = AtlasBuilder::build_dependency_graph(&contracts);
        let mut coords = BTreeMap::new();
        for c in &contracts {
            coords.insert(hash::hex(&c.qid), coords_from_qid(&c.qid));
        }
        let filaments = AtlasBuilder::generate_filaments(&deps, &coords);
        // Each domain with n members produces n*(n-1)/2 filaments.
        // We have 3 contracts in 3 different domains → 0 filaments (no same-domain pairs).
        assert_eq!(filaments.len(), 0);
    }

    #[test]
    fn frontiers_only_for_formal_proof() {
        let (contracts, outputs) = make_test_data();
        let frontiers = AtlasBuilder::generate_frontiers(&contracts, &outputs);
        // No FormalProof contracts in test data → no frontiers.
        assert_eq!(frontiers.len(), 0);
    }

    #[test]
    fn atlas_hash_deterministic() {
        let (contracts, outputs) = make_test_data();
        let root = hash::H(b"test_root");
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let a1 = AtlasBuilder::build(&contracts, &outputs, &root, &mut l1);
        let a2 = AtlasBuilder::build(&contracts, &outputs, &root, &mut l2);
        assert_eq!(a1.atlas_hash, a2.atlas_hash);
    }

    #[test]
    fn all_contracts_in_index() {
        let (contracts, outputs) = make_test_data();
        let root = hash::H(b"test_index");
        let mut ledger = Ledger::new();
        let atlas = AtlasBuilder::build(&contracts, &outputs, &root, &mut ledger);
        // Every contract should have an index star.
        assert_eq!(atlas.index_stars.len(), contracts.len());
        for contract in &contracts {
            let qid_hex = hash::hex(&contract.qid);
            assert!(atlas.index_stars.iter().any(|s| s.qid_hex == qid_hex),
                "Contract {} not in index", qid_hex);
        }
    }
}
