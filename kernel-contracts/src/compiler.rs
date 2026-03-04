use crate::contract::{Contract, ContractBudget, EvalSpec, Tiebreak};
use crate::alphabet::AnswerAlphabet;
use kernel_types::serpi::canonical_cbor_bytes;
use serde_json::Value;

/// Compile a JSON contract specification into a Contract.
///
/// Expected JSON format:
/// {
///   "type": "bool_cnf" | "table" | "arith_find",
///   "description": "...",
///   "max_cost": 1000000,
///   "max_steps": 10000,
///   "tiebreak": "lex_min" | "first_found",
///   ... type-specific fields ...
/// }
pub fn compile_contract(json: &str) -> Result<Contract, String> {
    let val: Value = serde_json::from_str(json)
        .map_err(|e| format!("JSON parse error: {}", e))?;

    let contract_type = val.get("type")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'type' field")?;

    let description = val.get("description")
        .and_then(|v| v.as_str())
        .unwrap_or("unnamed contract")
        .to_string();

    let max_cost = val.get("max_cost")
        .and_then(|v| v.as_u64())
        .unwrap_or(1_000_000);

    let max_steps = val.get("max_steps")
        .and_then(|v| v.as_u64())
        .unwrap_or(10_000);

    let tiebreak = match val.get("tiebreak").and_then(|v| v.as_str()) {
        Some("first_found") => Tiebreak::FirstFound,
        _ => Tiebreak::LexMin,
    };

    let budget = ContractBudget { max_cost, max_steps };

    match contract_type {
        "bool_cnf" => compile_bool_cnf(&val, budget, tiebreak, description),
        "table" => compile_table(&val, budget, tiebreak, description),
        "arith_find" => compile_arith_find(&val, budget, tiebreak, description),
        "formal_proof" => compile_formal_proof(&val, budget, tiebreak, description),
        "dominate" => compile_dominate(&val, budget, tiebreak, description),
        "space_engine" => compile_space_engine(&val, budget, tiebreak, description),
        "millennium_finite" => compile_millennium_finite(&val, budget, tiebreak, description),
        other => Err(format!("Unknown contract type: {}", other)),
    }
}

fn compile_bool_cnf(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let num_vars = val.get("num_vars")
        .and_then(|v| v.as_u64())
        .ok_or("Missing 'num_vars' for bool_cnf")? as usize;

    let clauses: Vec<Vec<i32>> = val.get("clauses")
        .and_then(|v| v.as_array())
        .ok_or("Missing 'clauses' for bool_cnf")?
        .iter()
        .map(|clause| {
            clause.as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|lit| lit.as_i64().map(|l| l as i32))
                .collect()
        })
        .collect();

    // Answer alphabet: all 2^n assignments (for small n).
    // Represent each assignment as a byte vector of 0s and 1s.
    if num_vars > 20 {
        return Err(format!("Too many variables for exhaustive search: {}", num_vars));
    }

    let domain_size = 1u64 << num_vars;
    let mut domain = Vec::new();
    for i in 0..domain_size {
        let assignment: Vec<u8> = (0..num_vars)
            .map(|bit| ((i >> bit) & 1) as u8)
            .collect();
        domain.push(canonical_cbor_bytes(&assignment));
    }

    let alphabet = AnswerAlphabet::Finite(domain);
    let eval = EvalSpec::BoolCnf { num_vars, clauses };

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

fn compile_table(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let entries: Vec<(Vec<u8>, Vec<u8>)> = val.get("entries")
        .and_then(|v| v.as_array())
        .ok_or("Missing 'entries' for table contract")?
        .iter()
        .filter_map(|entry| {
            let key = entry.get("key")?.as_str()?.as_bytes().to_vec();
            let value = entry.get("value")?.as_str()?.as_bytes().to_vec();
            Some((key, value))
        })
        .collect();

    let domain: Vec<Vec<u8>> = entries.iter().map(|(k, _)| k.clone()).collect();
    let alphabet = AnswerAlphabet::Finite(domain);
    let eval = EvalSpec::Table(entries);

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

fn compile_arith_find(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let coefficients: Vec<i64> = val.get("coefficients")
        .and_then(|v| v.as_array())
        .ok_or("Missing 'coefficients' for arith_find")?
        .iter()
        .filter_map(|c| c.as_i64())
        .collect();

    let target = val.get("target")
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'target' for arith_find")?;

    let lo = val.get("lo").and_then(|v| v.as_i64()).unwrap_or(-100);
    let hi = val.get("hi").and_then(|v| v.as_i64()).unwrap_or(100);

    let alphabet = AnswerAlphabet::IntRange { lo, hi };
    let eval = EvalSpec::ArithFind { coefficients, target };

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

fn compile_formal_proof(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let statement = val.get("statement")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'statement' for formal_proof")?
        .to_string();

    let formal_system = val.get("formal_system")
        .and_then(|v| v.as_str())
        .unwrap_or("Lean4")
        .to_string();

    let verifier_hash = val.get("verifier_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unpinned")
        .as_bytes()
        .to_vec();

    let library_hash = val.get("library_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unpinned")
        .as_bytes()
        .to_vec();

    let known_dependencies: Vec<String> = val.get("known_dependencies")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();

    let required_separator = val.get("required_separator")
        .and_then(|v| v.as_str())
        .unwrap_or("complete proof term in formal system")
        .to_string();

    let alphabet = AnswerAlphabet::FormalProof {
        verifier_hash: verifier_hash.clone(),
        formal_system: formal_system.clone(),
        library_hash: library_hash.clone(),
    };

    let eval = EvalSpec::FormalProof {
        statement,
        formal_system,
        known_dependencies,
        required_separator,
    };

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

fn compile_dominate(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let suite_hash = val.get("suite_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unpinned")
        .as_bytes()
        .to_vec();

    let competitor_id = val.get("competitor_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'competitor_id' for dominate")?
        .to_string();

    let scoring = val.get("scoring")
        .and_then(|v| v.as_str())
        .unwrap_or("lex:verified_success,false_claims,cost")
        .to_string();

    let alphabet = AnswerAlphabet::DominanceVerdict {
        suite_hash: suite_hash.clone(),
    };

    let eval = EvalSpec::Dominate {
        suite_hash,
        competitor_id,
        scoring,
    };

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

fn compile_space_engine(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let catalog_hash = val.get("catalog_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unpinned")
        .as_bytes()
        .to_vec();

    let scenario_hash = val.get("scenario_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unpinned")
        .as_bytes()
        .to_vec();

    let kernel_build_hash = val.get("kernel_build_hash")
        .and_then(|v| v.as_str())
        .unwrap_or("unpinned")
        .as_bytes()
        .to_vec();

    let alphabet = AnswerAlphabet::SpaceEngineVerdict;

    let eval = EvalSpec::SpaceEngine {
        catalog_hash,
        scenario_hash,
        kernel_build_hash,
    };

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

fn compile_millennium_finite(
    val: &Value,
    budget: ContractBudget,
    tiebreak: Tiebreak,
    description: String,
) -> Result<Contract, String> {
    let problem_id = val.get("problem_id")
        .and_then(|v| v.as_str())
        .ok_or("Missing 'problem_id' for millennium_finite")?
        .to_string();

    let parameter_n = val.get("parameter_n")
        .and_then(|v| v.as_i64())
        .ok_or("Missing 'parameter_n' for millennium_finite")?;

    let parameter_aux = val.get("parameter_aux")
        .and_then(|v| v.as_i64());

    let alphabet = AnswerAlphabet::Bool;
    let eval = EvalSpec::MillenniumFinite {
        problem_id,
        parameter_n,
        parameter_aux,
    };

    Ok(Contract::new(alphabet, eval, budget, tiebreak, description))
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::SerPi;

    #[test]
    fn compile_simple_bool_cnf() {
        let json = r#"{
            "type": "bool_cnf",
            "description": "simple 2-var SAT",
            "num_vars": 2,
            "clauses": [[1, 2], [-1, 2]],
            "max_cost": 100,
            "max_steps": 100
        }"#;
        let contract = compile_contract(json).unwrap();
        assert_eq!(contract.description, "simple 2-var SAT");
    }

    #[test]
    fn compile_dominate_contract() {
        let json = r#"{
            "type": "dominate",
            "description": "dominance test",
            "competitor_id": "gpt-4",
            "suite_hash": "abc123",
            "scoring": "lex:verified_success,false_claims,cost"
        }"#;
        let contract = compile_contract(json).unwrap();
        assert_eq!(contract.description, "dominance test");
        assert!(contract.answer_alphabet.is_dominance());
        assert_eq!(contract.answer_alphabet.size(), 2);
    }

    #[test]
    fn compile_space_engine_contract() {
        let json = r#"{
            "type": "space_engine",
            "description": "SE verify test",
            "catalog_hash": "deadbeef01",
            "scenario_hash": "cafebabe02",
            "kernel_build_hash": "0102030405"
        }"#;
        let contract = compile_contract(json).unwrap();
        assert_eq!(contract.description, "SE verify test");
        assert!(contract.answer_alphabet.is_space_engine());
        assert_eq!(contract.answer_alphabet.size(), 2);
    }

    #[test]
    fn space_engine_verdict_enumerates() {
        let alphabet = AnswerAlphabet::SpaceEngineVerdict;
        let vals = alphabet.enumerate();
        assert_eq!(vals.len(), 2);
        assert_eq!(vals[0], b"VERIFIED");
        assert_eq!(vals[1], b"NOT_VERIFIED");
        assert!(alphabet.is_enumerable());
    }

    #[test]
    fn space_engine_serpi_deterministic() {
        let json = r#"{
            "type": "space_engine",
            "description": "SE serpi test",
            "catalog_hash": "aabb",
            "scenario_hash": "ccdd",
            "kernel_build_hash": "eeff"
        }"#;
        let c1 = compile_contract(json).unwrap();
        let c2 = compile_contract(json).unwrap();
        assert_eq!(c1.qid, c2.qid);
        assert_eq!(c1.eval.ser_pi(), c2.eval.ser_pi());
    }

    #[test]
    fn compile_table_contract() {
        let json = r#"{
            "type": "table",
            "description": "lookup",
            "entries": [
                {"key": "a", "value": "SAT"},
                {"key": "b", "value": "UNSAT"},
                {"key": "c", "value": "SAT"}
            ]
        }"#;
        let contract = compile_contract(json).unwrap();
        assert_eq!(contract.description, "lookup");
    }
}
