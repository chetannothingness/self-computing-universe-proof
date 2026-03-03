//! L2 witness encoder: reads SolveOutput witness data and produces witness-content objects.
//!
//! SAT BoolCnf → moons + rings, UNSAT BoolCnf → proof-step cluster,
//! ArithFind → planet + decoy, DarkObject → lensing proxies.

use kernel_types::{SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::receipt::SolveOutput;
use kernel_contracts::contract::{Contract, EvalSpec};
use kernel_ledger::{Event, EventKind, Ledger};
use crate::types::{Rational, DarkObject};
use crate::witness_types::*;

/// Core L2 witness encoder.
pub struct WitnessEncoder;

impl WitnessEncoder {
    /// SAT BoolCnf: Parse CBOR witness Vec<u8> (assignment), create moons + rings.
    pub fn encode_sat_witness(
        contract: &Contract,
        output: &SolveOutput,
        galaxy_name: &str,
        ledger: &mut Ledger,
    ) -> Option<SatWitnessMoons> {
        let (_num_vars, clauses) = match &contract.eval {
            EvalSpec::BoolCnf { num_vars, clauses } => (*num_vars, clauses.clone()),
            _ => return None,
        };

        if output.status != kernel_types::Status::Unique {
            return None;
        }

        // Parse CBOR witness: Vec<u8> assignment where each byte is 0 or 1.
        let assignment: Vec<u8> = match ciborium::from_reader(&output.payload.witness[..]) {
            Ok(a) => a,
            Err(_) => return None,
        };

        let qid_hex = hash::hex(&contract.qid);
        let prefix = &qid_hex[..8.min(qid_hex.len())];

        // Build moons: one per variable.
        let mut moons = Vec::with_capacity(assignment.len());
        for (i, &val) in assignment.iter().enumerate() {
            let bit = val != 0;
            // Assign to first clause that contains this variable.
            let parent_ring = clauses.iter().position(|clause| {
                clause.iter().any(|&lit| lit.unsigned_abs() as usize == i + 1)
            }).unwrap_or(0) as u32;

            moons.push(WitnessMoon {
                moon_index: i as u32,
                variable_index: i as u32,
                bit_value: bit,
                inclination_milli_deg: if bit { 45_000 } else { -45_000 },
                phase_quadrant: if bit { 0 } else { 2 },
                parent_ring_index: parent_ring,
            });
        }

        // Build clause rings.
        let clause_rings: Vec<ClauseRing> = clauses.iter().enumerate().map(|(ci, clause)| {
            let satisfied = clause.iter().any(|&lit| {
                let var_idx = (lit.unsigned_abs() as usize).saturating_sub(1);
                let val = assignment.get(var_idx).copied().unwrap_or(0) != 0;
                if lit > 0 { val } else { !val }
            });
            ClauseRing {
                clause_index: ci as u32,
                literal_count: clause.len() as u32,
                is_satisfied: satisfied,
                orbital_radius_milli_au: (ci as i64 + 1) * 1000,
            }
        }).collect();

        let result = SatWitnessMoons {
            parent_galaxy_name: galaxy_name.to_string(),
            qid_hex: qid_hex.clone(),
            moons,
            clause_rings,
            contract_hash: contract.qid,
        };

        // Emit ledger event.
        let payload = canonical_cbor_bytes(&("WitnessEncode", "SAT", prefix));
        ledger.commit(Event::new(
            EventKind::WitnessEncode,
            &payload,
            vec![],
            1,
            1,
        ));

        Some(result)
    }

    /// UNSAT BoolCnf: Generate proof-step stars from clause structure.
    pub fn encode_unsat_witness(
        contract: &Contract,
        output: &SolveOutput,
        galaxy_name: &str,
        ledger: &mut Ledger,
    ) -> Option<UnsatWitnessCluster> {
        let clauses = match &contract.eval {
            EvalSpec::BoolCnf { clauses, .. } => clauses.clone(),
            _ => return None,
        };

        if output.status != kernel_types::Status::Unsat {
            return None;
        }

        // Must be a BoolCnf UNSAT (not an admissibility refutation).
        let witness_str = String::from_utf8_lossy(&output.payload.witness);
        if witness_str.starts_with("UNSAT(admissibility)") {
            return None;
        }

        let qid_hex = hash::hex(&contract.qid);
        let prefix = &qid_hex[..8.min(qid_hex.len())];
        let total = clauses.len();

        // Build proof steps: one star per clause.
        let proof_steps: Vec<ProofStepStar> = clauses.iter().enumerate().map(|(i, clause)| {
            // Find parent: clause sharing the most variables with this one.
            let parent = if i == 0 {
                None
            } else {
                let vars_i: std::collections::BTreeSet<u32> = clause.iter()
                    .map(|&lit| lit.unsigned_abs())
                    .collect();
                let mut best_parent = None;
                let mut best_overlap = 0usize;
                for j in 0..i {
                    let vars_j: std::collections::BTreeSet<u32> = clauses[j].iter()
                        .map(|&lit| lit.unsigned_abs())
                        .collect();
                    let overlap = vars_i.intersection(&vars_j).count();
                    if overlap > best_overlap {
                        best_overlap = overlap;
                        best_parent = Some(j as u32);
                    }
                }
                best_parent
            };

            ProofStepStar {
                step_index: i as u32,
                radial_distance_milli_pc: (total.saturating_sub(i)) as i64 * 100,
                parent_step_index: parent,
                spectral_class: (i as u8) % 7,
            }
        }).collect();

        let contradiction_step_index = if total > 0 { (total - 1) as u32 } else { 0 };

        let result = UnsatWitnessCluster {
            parent_galaxy_name: galaxy_name.to_string(),
            qid_hex: qid_hex.clone(),
            cluster_name: format!("KC-{}", prefix),
            proof_steps,
            contradiction_step_index,
            contract_hash: contract.qid,
        };

        let payload = canonical_cbor_bytes(&("WitnessEncode", "UNSAT", prefix));
        ledger.commit(Event::new(
            EventKind::WitnessEncode,
            &payload,
            vec![],
            1,
            1,
        ));

        Some(result)
    }

    /// ArithFind: Parse CBOR i64 witness, create planet + decoy.
    pub fn encode_arith_witness(
        contract: &Contract,
        output: &SolveOutput,
        star_name: &str,
        ledger: &mut Ledger,
    ) -> Option<ArithWitnessPlanet> {
        match &contract.eval {
            EvalSpec::ArithFind { .. } => {}
            _ => return None,
        }

        if output.status != kernel_types::Status::Unique {
            return None;
        }

        // Parse CBOR witness: i64 value.
        let witness_value: i64 = match ciborium::from_reader(&output.payload.witness[..]) {
            Ok(v) => v,
            Err(_) => return None,
        };

        let qid_hex = hash::hex(&contract.qid);
        let prefix = &qid_hex[..8.min(qid_hex.len())];

        let period_exact = Rational::integer(witness_value);
        // Decoy: (2x+1)/2 — half-integer, impossible for correct integer x.
        let decoy_orbit = Rational::new(witness_value * 2 + 1, 2);
        // decoy_valid: always true when proof is correct (period != decoy).
        let decoy_valid = period_exact.num != decoy_orbit.num || period_exact.den != decoy_orbit.den;

        let result = ArithWitnessPlanet {
            parent_star_name: star_name.to_string(),
            qid_hex: qid_hex.clone(),
            witness_value,
            period_exact,
            decoy_orbit,
            decoy_valid,
            contract_hash: contract.qid,
        };

        let payload = canonical_cbor_bytes(&("WitnessEncode", "Arith", prefix));
        ledger.commit(Event::new(
            EventKind::WitnessEncode,
            &payload,
            vec![],
            1,
            1,
        ));

        Some(result)
    }

    /// Dark objects -> lensing proxies.
    pub fn encode_dark_lensing(
        dark_objects: &[DarkObject],
        ledger: &mut Ledger,
    ) -> Vec<LensingProxy> {
        let mut proxies = Vec::new();

        for dark in dark_objects {
            let h = dark.ser_pi_hash();
            let mass_bytes: [u8; 8] = h[0..8].try_into().unwrap();
            let raw_mass = i64::from_le_bytes(mass_bytes).unsigned_abs() % 10000;
            let lensing_mass = Rational::new(raw_mass as i64, 1000);
            let einstein_radius = (lensing_mass.num * 100) / lensing_mass.den as i64;

            let prefix = &dark.qid_hex[..8.min(dark.qid_hex.len())];

            proxies.push(LensingProxy {
                qid_hex: dark.qid_hex.clone(),
                proxy_name: format!("KLens-{}", prefix),
                coord_x: dark.coord_x,
                coord_y: dark.coord_y,
                coord_z: dark.coord_z,
                lensing_mass,
                einstein_radius_milli_arcsec: einstein_radius,
                dark_object_hash: dark.contract_hash,
            });
        }

        if !proxies.is_empty() {
            let payload = canonical_cbor_bytes(&("WitnessEncode", "DarkLensing", proxies.len() as u64));
            ledger.commit(Event::new(
                EventKind::WitnessEncode,
                &payload,
                vec![],
                1,
                1,
            ));
        }

        proxies
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_solver::Solver;
    use kernel_contracts::compiler::compile_contract;

    #[test]
    fn sat_witness_correct_moon_count() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"sat2","num_vars":2,"clauses":[[1,2]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_sat_witness(&c, &o, "KG-test", &mut ledger);
        assert!(w.is_some());
        let w = w.unwrap();
        assert_eq!(w.moons.len(), 2);
    }

    #[test]
    fn sat_witness_clause_rings() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"sat_rings","num_vars":2,"clauses":[[1],[2]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_sat_witness(&c, &o, "KG-test", &mut ledger).unwrap();
        assert_eq!(w.clause_rings.len(), 2);
        // All rings should be satisfied for a SAT witness.
        assert!(w.clause_rings.iter().all(|r| r.is_satisfied));
    }

    #[test]
    fn sat_witness_deterministic() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"sat_det","num_vars":2,"clauses":[[1,2]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let w1 = WitnessEncoder::encode_sat_witness(&c, &o, "KG-test", &mut l1).unwrap();
        let w2 = WitnessEncoder::encode_sat_witness(&c, &o, "KG-test", &mut l2).unwrap();
        assert_eq!(w1.ser_pi(), w2.ser_pi());
    }

    #[test]
    fn unsat_witness_step_count() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"unsat","num_vars":1,"clauses":[[1],[-1]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_unsat_witness(&c, &o, "KG-test", &mut ledger);
        assert!(w.is_some());
        let w = w.unwrap();
        assert_eq!(w.proof_steps.len(), 2);
    }

    #[test]
    fn unsat_witness_contradiction_center() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"unsat_ctr","num_vars":1,"clauses":[[1],[-1]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_unsat_witness(&c, &o, "KG-test", &mut ledger).unwrap();
        // Contradiction is the last clause.
        assert_eq!(w.contradiction_step_index, 1);
        // Last step has smallest radial distance (innermost).
        let last = &w.proof_steps[w.proof_steps.len() - 1];
        let first = &w.proof_steps[0];
        assert!(last.radial_distance_milli_pc < first.radial_distance_milli_pc);
    }

    #[test]
    fn arith_witness_period_match() {
        let c = compile_contract(
            r#"{"type":"arith_find","description":"arith","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_arith_witness(&c, &o, "KS-test", &mut ledger);
        assert!(w.is_some());
        let w = w.unwrap();
        assert_eq!(w.witness_value, 5);
        assert_eq!(w.period_exact, Rational::integer(5));
    }

    #[test]
    fn arith_witness_decoy_validity() {
        let c = compile_contract(
            r#"{"type":"arith_find","description":"arith_decoy","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_arith_witness(&c, &o, "KS-test", &mut ledger).unwrap();
        assert!(w.decoy_valid);
        // Decoy = (2*5+1)/2 = 11/2 = 5.5, differs from witness = 5.
        assert_eq!(w.decoy_orbit, Rational::new(11, 2));
        assert_ne!(w.period_exact.num * w.decoy_orbit.den as i64,
                   w.decoy_orbit.num * w.period_exact.den as i64);
    }

    #[test]
    fn dark_lensing_mass_derivation() {
        use kernel_types::HASH_ZERO;
        let dark = DarkObject {
            qid_hex: "abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234abcd1234".into(),
            name: "KD-abcd1234".into(),
            coord_x: 100, coord_y: 200, coord_z: 300,
            mass_estimate: Rational::new(i64::MAX / 2, 1),
            reason_inadmissible: "test".into(),
            contract_hash: HASH_ZERO,
        };
        let mut ledger = Ledger::new();
        let proxies = WitnessEncoder::encode_dark_lensing(&[dark.clone()], &mut ledger);
        assert_eq!(proxies.len(), 1);
        let proxy = &proxies[0];
        assert_eq!(proxy.coord_x, dark.coord_x);
        assert_eq!(proxy.coord_y, dark.coord_y);
        assert_eq!(proxy.coord_z, dark.coord_z);
        assert!(proxy.lensing_mass.num >= 0);
        assert!(proxy.lensing_mass.den > 0);
        assert!(proxy.einstein_radius_milli_arcsec >= 0);
    }

    #[test]
    fn full_round_trip_determinism() {
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"roundtrip","num_vars":2,"clauses":[[1,2]]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut l1 = Ledger::new();
        let mut l2 = Ledger::new();
        let w1 = WitnessEncoder::encode_sat_witness(&c, &o, "KG-rt", &mut l1).unwrap();
        let w2 = WitnessEncoder::encode_sat_witness(&c, &o, "KG-rt", &mut l2).unwrap();
        assert_eq!(w1.ser_pi_hash(), w2.ser_pi_hash());
    }

    #[test]
    fn empty_clauses_edge_case() {
        // BoolCnf with 0 clauses is trivially SAT.
        let c = compile_contract(
            r#"{"type":"bool_cnf","description":"empty","num_vars":1,"clauses":[]}"#
        ).unwrap();
        let mut solver = Solver::new();
        let o = solver.solve(&c);
        let mut ledger = Ledger::new();
        let w = WitnessEncoder::encode_sat_witness(&c, &o, "KG-empty", &mut ledger);
        // Depending on solver behavior: if UNIQUE with empty clauses, we get a witness.
        if let Some(w) = w {
            assert_eq!(w.clause_rings.len(), 0);
        }
    }
}
