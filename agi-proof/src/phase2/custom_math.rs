// Phase 2C: Custom Axiomatic Math
// Finite algebraic axiom systems with deterministic proof checker.

use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

/// Custom finite algebraic axiom system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MathWorld {
    pub seed: [u8; 32],
    pub num_symbols: u32,
    pub axioms: Vec<Axiom>,
    pub target_theorem: ProofTerm,
    pub max_proof_length: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Axiom {
    pub id: u32,
    pub premises: Vec<ProofTerm>,
    pub conclusion: ProofTerm,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ProofTerm {
    pub symbol: u32,
    pub args: Vec<ProofTerm>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    pub axiom_id: u32,
    pub substitution: BTreeMap<u32, ProofTerm>,
    pub result: ProofTerm,
}

/// Proof checker (the judge).
pub fn check_proof(
    axioms: &[Axiom],
    target: &ProofTerm,
    proof_steps: &[ProofStep],
) -> JudgeVerdict {
    let mut proven: Vec<ProofTerm> = Vec::new();

    for step in proof_steps {
        // Find the axiom
        let axiom = match axioms.iter().find(|a| a.id == step.axiom_id) {
            Some(a) => a,
            None => return JudgeVerdict::Fail,
        };

        // Check that all premises are either axioms or previously proven
        for premise in &axiom.premises {
            let subst_premise = apply_substitution(premise, &step.substitution);
            if !proven.contains(&subst_premise) {
                return JudgeVerdict::Fail;
            }
        }

        // The conclusion under substitution must equal step.result
        let subst_conclusion = apply_substitution(&axiom.conclusion, &step.substitution);
        if subst_conclusion != step.result {
            return JudgeVerdict::Fail;
        }

        proven.push(step.result.clone());
    }

    // Final step must equal target
    if proven.last() == Some(target) {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

/// Apply substitution to a proof term.
fn apply_substitution(term: &ProofTerm, subst: &BTreeMap<u32, ProofTerm>) -> ProofTerm {
    if let Some(replacement) = subst.get(&term.symbol) {
        if term.args.is_empty() {
            return replacement.clone();
        }
    }
    ProofTerm {
        symbol: term.symbol,
        args: term.args.iter()
            .map(|a| apply_substitution(a, subst))
            .collect(),
    }
}

/// Generate from seed.
pub fn generate_math_world(seed: &[u8; 32], episode: u32) -> MathWorld {
    use kernel_types::hash;

    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    let num_symbols = 3 + (ep_seed[0] as u32 % 6);
    let max_proof_length = 5 + (ep_seed[1] as u32 % 15);

    // Generate axioms with a guaranteed reachable chain.
    // First axiom: base axiom with empty premises (allows proof to start).
    let mut axioms = Vec::new();
    let num_axioms = 2 + (ep_seed[2] as usize % 4);

    let base_sym = ep_seed[3] as u32 % num_symbols;
    axioms.push(Axiom {
        id: 0,
        premises: vec![],
        conclusion: ProofTerm { symbol: base_sym, args: vec![] },
    });

    // Chain axioms: each derives a new symbol from the previous conclusion.
    let mut current_sym = base_sym;
    for i in 1..num_axioms {
        let next_sym = ep_seed[(i * 2 + 4) % 32] as u32 % num_symbols;
        axioms.push(Axiom {
            id: i as u32,
            premises: vec![ProofTerm { symbol: current_sym, args: vec![] }],
            conclusion: ProofTerm { symbol: next_sym, args: vec![] },
        });
        current_sym = next_sym;
    }

    // Target: the last symbol in the chain (guaranteed reachable).
    let target_theorem = ProofTerm { symbol: current_sym, args: vec![] };

    MathWorld {
        seed: *seed,
        num_symbols,
        axioms,
        target_theorem,
        max_proof_length,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_world_deterministic() {
        let seed = [3u8; 32];
        let w1 = generate_math_world(&seed, 0);
        let w2 = generate_math_world(&seed, 0);
        assert_eq!(w1.num_symbols, w2.num_symbols);
        assert_eq!(w1.axioms.len(), w2.axioms.len());
    }

    #[test]
    fn math_proof_checker_valid_passes() {
        let axioms = vec![
            Axiom {
                id: 0,
                premises: vec![],
                conclusion: ProofTerm { symbol: 0, args: vec![] },
            },
            Axiom {
                id: 1,
                premises: vec![ProofTerm { symbol: 0, args: vec![] }],
                conclusion: ProofTerm { symbol: 1, args: vec![] },
            },
        ];
        let target = ProofTerm { symbol: 1, args: vec![] };
        let proof = vec![
            ProofStep {
                axiom_id: 0,
                substitution: BTreeMap::new(),
                result: ProofTerm { symbol: 0, args: vec![] },
            },
            ProofStep {
                axiom_id: 1,
                substitution: BTreeMap::new(),
                result: ProofTerm { symbol: 1, args: vec![] },
            },
        ];
        assert_eq!(check_proof(&axioms, &target, &proof), JudgeVerdict::Pass);
    }

    #[test]
    fn math_generated_world_has_base_axiom() {
        for ep in 0..20u32 {
            let seed = [ep as u8; 32];
            let world = generate_math_world(&seed, ep);
            let has_base = world.axioms.iter().any(|a| a.premises.is_empty());
            assert!(has_base,
                "Episode {} should have at least one base axiom (empty premises)", ep);
        }
    }

    #[test]
    fn math_generated_world_solvable() {
        for ep in 0..20u32 {
            let seed = [ep as u8; 32];
            let world = generate_math_world(&seed, ep);

            // BFS solver: start from base axioms, chain forward
            let mut proven: Vec<ProofTerm> = Vec::new();
            let mut proof_steps: Vec<ProofStep> = Vec::new();

            // Base axioms
            for axiom in &world.axioms {
                if axiom.premises.is_empty() {
                    let result = axiom.conclusion.clone();
                    if !proven.contains(&result) {
                        proof_steps.push(ProofStep {
                            axiom_id: axiom.id,
                            substitution: BTreeMap::new(),
                            result: result.clone(),
                        });
                        proven.push(result);
                    }
                }
            }

            // Chain forward
            for _ in 0..world.max_proof_length {
                for axiom in &world.axioms {
                    if axiom.premises.len() == 1 {
                        for p in proven.clone().iter() {
                            if *p == axiom.premises[0] {
                                let result = axiom.conclusion.clone();
                                if !proven.contains(&result) {
                                    proof_steps.push(ProofStep {
                                        axiom_id: axiom.id,
                                        substitution: BTreeMap::new(),
                                        result: result.clone(),
                                    });
                                    proven.push(result);
                                }
                            }
                        }
                    }
                }
            }

            assert!(proven.contains(&world.target_theorem),
                "Episode {} target should be provable", ep);
        }
    }

    #[test]
    fn math_proof_checker_invalid_fails() {
        let axioms = vec![
            Axiom {
                id: 0,
                premises: vec![],
                conclusion: ProofTerm { symbol: 0, args: vec![] },
            },
        ];
        let target = ProofTerm { symbol: 1, args: vec![] };
        let proof = vec![
            ProofStep {
                axiom_id: 0,
                substitution: BTreeMap::new(),
                result: ProofTerm { symbol: 0, args: vec![] },
            },
        ];
        // Target is symbol 1 but proof only proves symbol 0
        assert_eq!(check_proof(&axioms, &target, &proof), JudgeVerdict::Fail);
    }
}
