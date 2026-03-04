use kernel_types::{Hash32, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_contracts::contract::{Contract, ContractBudget, EvalSpec, Tiebreak};
use kernel_contracts::alphabet::AnswerAlphabet;
use crate::eval_specs::{AgiDomainKind, AgiDomainSpec};
use serde_json::Value;

/// Compile an AGI domain task JSON into a Contract.
///
/// The AGI domain is encoded using EvalSpec::Table with a single entry
/// whose key is the canonical serialization of the AgiDomainSpec and
/// whose value is the domain tag. This allows the kernel solver to
/// process it through the existing pipeline. The actual evaluation
/// is performed by the domain-specific simulator judge in the
/// agi-proof crate.
///
/// Expected JSON format:
/// {
///   "type": "agi_domain",
///   "domain": "SynthPhysics" | "AlienChemistry" | ... ,
///   "description": "...",
///   "world_seed": "<hex64>",
///   "goal_spec": "<hex>",
///   "judge_hash": "<hex64>",
///   "max_experiments": 100,
///   "max_cost": 10000,
///   "max_steps": 1000
/// }
pub fn compile_agi_contract(json: &str) -> Result<(Contract, AgiDomainSpec), String> {
    let val: Value = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let contract_type = val.get("type")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'type' field")?;

    if contract_type != "agi_domain" {
        return Err(format!("Expected type 'agi_domain', got '{}'", contract_type));
    }

    let description = val.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed AGI task")
        .to_string();

    let domain_str = val.get("domain")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'domain' field")?;

    let domain = parse_domain_kind(domain_str)?;

    let world_seed = parse_hex_seed(
        val.get("world_seed").and_then(|v| v.as_str()).unwrap_or("")
    )?;

    let goal_spec = parse_hex_bytes(
        val.get("goal_spec").and_then(|v| v.as_str()).unwrap_or("")
    );

    let judge_hash = parse_hex_hash(
        val.get("judge_hash").and_then(|v| v.as_str()).unwrap_or("")
    );

    let max_experiments = val.get("max_experiments")
        .and_then(|v| v.as_u64())
        .unwrap_or(100);

    let max_cost = val.get("max_cost")
        .and_then(|v| v.as_u64())
        .unwrap_or(max_experiments * 10);

    let max_steps = val.get("max_steps")
        .and_then(|v| v.as_u64())
        .unwrap_or(1000);

    let spec = AgiDomainSpec {
        domain: domain.clone(),
        world_seed,
        goal_spec: goal_spec.clone(),
        judge_hash,
        max_experiments,
    };

    // Encode as a Table contract: single entry with domain spec as key,
    // domain name as value. The alphabet has two entries: the solution
    // or UNSAT.
    let spec_bytes = canonical_cbor_bytes(&spec.ser_pi());
    let domain_tag = domain.name().as_bytes().to_vec();

    let entries = vec![
        (spec_bytes.clone(), domain_tag.clone()),
    ];

    // The answer alphabet: AGI solution payload (variable size)
    // We use Bytes { max_len: 3 } for the verdict envelope.
    // Actual evaluation is done by the agi-proof simulator judges.
    let alphabet = AnswerAlphabet::Finite(vec![
        b"PASS".to_vec(),
        b"FAIL".to_vec(),
    ]);

    let eval = EvalSpec::Table(entries);
    let budget = ContractBudget { max_cost, max_steps };
    let tiebreak = Tiebreak::FirstFound;

    let contract = Contract::new(alphabet, eval, budget, tiebreak, description);

    Ok((contract, spec))
}

fn parse_domain_kind(s: &str) -> Result<AgiDomainKind, String> {
    match s {
        "SynthPhysics" => Ok(AgiDomainKind::SynthPhysics),
        "AlienChemistry" => Ok(AgiDomainKind::AlienChemistry),
        "CustomMath" => Ok(AgiDomainKind::CustomMath),
        "CompanySandbox" => Ok(AgiDomainKind::CompanySandbox),
        "BioMedSandbox" => Ok(AgiDomainKind::BioMedSandbox),
        "CausalReasoning" => Ok(AgiDomainKind::CausalReasoning),
        "ModelDiscovery" => Ok(AgiDomainKind::ModelDiscovery),
        "MaterialsDesign" => Ok(AgiDomainKind::MaterialsDesign),
        "AlgoDiscovery" => Ok(AgiDomainKind::AlgoDiscovery),
        "PhysicalReasoning" => Ok(AgiDomainKind::PhysicalReasoning),
        "SocialReasoning" => Ok(AgiDomainKind::SocialReasoning),
        "MultiStepPlanning" => Ok(AgiDomainKind::MultiStepPlanning),
        other => Err(format!("Unknown AGI domain: {}", other)),
    }
}

fn parse_hex_seed(s: &str) -> Result<[u8; 32], String> {
    if s.is_empty() {
        return Ok([0u8; 32]);
    }
    hash::from_hex(s).ok_or_else(|| format!("Invalid hex seed: {}", s))
}

fn parse_hex_hash(s: &str) -> Hash32 {
    if s.is_empty() {
        return [0u8; 32];
    }
    hash::from_hex(s).unwrap_or([0u8; 32])
}

fn parse_hex_bytes(s: &str) -> Vec<u8> {
    if s.is_empty() {
        return vec![];
    }
    // Decode hex pairs
    let mut result = Vec::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        let hi = hex_digit(bytes[i]);
        let lo = hex_digit(bytes[i + 1]);
        if let (Some(h), Some(l)) = (hi, lo) {
            result.push((h << 4) | l);
        }
        i += 2;
    }
    result
}

fn hex_digit(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

use kernel_types::SerPi;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_synth_physics_task() {
        let json = r#"{
            "type": "agi_domain",
            "domain": "SynthPhysics",
            "description": "stable orbit test",
            "world_seed": "0000000000000000000000000000000000000000000000000000000000000001",
            "goal_spec": "",
            "judge_hash": "",
            "max_experiments": 50
        }"#;
        let (contract, spec) = compile_agi_contract(json).unwrap();
        assert_eq!(contract.description, "stable orbit test");
        assert_eq!(spec.domain, AgiDomainKind::SynthPhysics);
        assert_eq!(spec.max_experiments, 50);
        assert_eq!(spec.b_star(), 500);
    }

    #[test]
    fn compile_agi_contract_deterministic() {
        let json = r#"{
            "type": "agi_domain",
            "domain": "CustomMath",
            "description": "proof test",
            "world_seed": "",
            "max_experiments": 100
        }"#;
        let (c1, _) = compile_agi_contract(json).unwrap();
        let (c2, _) = compile_agi_contract(json).unwrap();
        assert_eq!(c1.qid, c2.qid);
    }

    #[test]
    fn compile_rejects_unknown_domain() {
        let json = r#"{
            "type": "agi_domain",
            "domain": "FakeScience",
            "description": "nope"
        }"#;
        assert!(compile_agi_contract(json).is_err());
    }

    #[test]
    fn compile_rejects_wrong_type() {
        let json = r#"{
            "type": "bool_cnf",
            "domain": "SynthPhysics"
        }"#;
        assert!(compile_agi_contract(json).is_err());
    }
}
