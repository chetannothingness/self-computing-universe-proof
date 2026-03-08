//! ExtractRule — verified proof → rewrite rules.
//!
//! When the proof checker accepts a witness π for goal S,
//! ExtractRule(π) compiles the witness into rewrite rules in R.
//!
//! The soundness proof IS the witness itself — it was verified by the checker.
//! R grows only from verified computation. Never from heuristics. Never from humans.
//!
//! Extraction produces:
//!   1. The full proof as a single rule: goal_type → PROVED
//!   2. Sub-terms that represent useful lemmas
//!   3. Application patterns that may match future goals

use super::core_term::CoreTerm;
use super::rewrite::{RewriteRule, make_rule};
use kernel_types::{Hash32, hash};

/// Extract rewrite rules from a verified proof witness.
///
/// The witness has been verified by the type checker: it inhabits goal_type.
/// Now we compile it into rewrite rules so future normalization can use it.
///
/// The soundness proof IS the serialized witness bytes.
pub fn extract_rules(
    witness: &CoreTerm,
    goal_type: &CoreTerm,
    witness_hash: Hash32,
) -> Vec<RewriteRule> {
    let mut rules = Vec::new();
    let witness_bytes = witness.to_bytes();

    // 1. Full proof as a single rule: goal_type → PROVED(witness_hash)
    //    This means: if we see this exact goal again, it's already proved.
    let proved_marker = CoreTerm::Constructor {
        type_name: "Proved".into(),
        ctor_name: "mk".into(),
        args: vec![goal_type.clone()],
    };
    rules.push(make_rule(
        goal_type.clone(),
        proved_marker,
        &witness_bytes,
        witness_hash,
    ));

    // 2. Extract sub-lemmas from the proof structure
    extract_sublemmas(witness, &witness_bytes, witness_hash, &mut rules);

    rules
}

/// Extract sub-lemmas from a proof term.
/// Each application, lambda body, or let-binding may contain reusable structure.
fn extract_sublemmas(
    term: &CoreTerm,
    witness_bytes: &[u8],
    witness_hash: Hash32,
    rules: &mut Vec<RewriteRule>,
) {
    match term {
        // Application chains: f a b c → extract (f a), (f a b) as potentially reusable
        CoreTerm::App { func, arg } => {
            // If the function is itself an application, the intermediate result may be a lemma
            if let CoreTerm::App { func: inner_func, arg: inner_arg } = func.as_ref() {
                // Pattern: inner_func applied to inner_arg, then to arg
                // The intermediate (inner_func inner_arg) might be a reusable lemma
                let intermediate = CoreTerm::App {
                    func: inner_func.clone(),
                    arg: inner_arg.clone(),
                };
                let intermediate_hash = intermediate.term_hash();
                // Only add if it's non-trivial (size > 2)
                if intermediate.size() > 2 {
                    let result = CoreTerm::App {
                        func: Box::new(intermediate.clone()),
                        arg: arg.clone(),
                    };
                    rules.push(make_rule(
                        result,
                        term.clone(), // the full application
                        witness_bytes,
                        witness_hash,
                    ));
                }
            }

            // Recurse into subterms
            extract_sublemmas(func, witness_bytes, witness_hash, rules);
            extract_sublemmas(arg, witness_bytes, witness_hash, rules);
        }

        // Lambda bodies contain proof structure
        CoreTerm::Lam { param_type, body } => {
            extract_sublemmas(body, witness_bytes, witness_hash, rules);
        }

        // Let bindings: the value is a potentially reusable intermediate result
        CoreTerm::Let { bound_type, value, body } => {
            extract_sublemmas(value, witness_bytes, witness_hash, rules);
            extract_sublemmas(body, witness_bytes, witness_hash, rules);
        }

        // Constructors: if they have non-trivial args, extract those
        CoreTerm::Constructor { args, .. } => {
            for arg in args {
                if arg.size() > 2 {
                    extract_sublemmas(arg, witness_bytes, witness_hash, rules);
                }
            }
        }

        // Atomic terms: nothing to extract
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_simple_proof() {
        let witness = CoreTerm::NatLit(42);
        let goal = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
        let witness_hash = witness.term_hash();

        let rules = extract_rules(&witness, &goal, witness_hash);
        // Should have at least the full-proof rule
        assert!(!rules.is_empty(), "should extract at least 1 rule");

        // First rule: goal → PROVED
        assert_eq!(rules[0].lhs, goal);
    }

    #[test]
    fn extract_from_compound_proof() {
        // A proof with application structure: (f a) b
        let witness = CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "lemma1".into(), levels: vec![] }),
                arg: Box::new(CoreTerm::NatLit(5)),
            }),
            arg: Box::new(CoreTerm::NatLit(10)),
        };
        let goal = CoreTerm::Prop;
        let witness_hash = witness.term_hash();

        let rules = extract_rules(&witness, &goal, witness_hash);
        // Should have the full-proof rule + extracted sub-lemmas
        assert!(rules.len() >= 1);
    }

    #[test]
    fn extract_preserves_soundness_proof() {
        let witness = CoreTerm::NatLit(0);
        let goal = CoreTerm::Prop;
        let witness_hash = witness.term_hash();

        let rules = extract_rules(&witness, &goal, witness_hash);
        for rule in &rules {
            // Every rule should carry the witness bytes as soundness proof
            assert!(!rule.soundness_proof.is_empty());
            assert_eq!(rule.source_witness_hash, witness_hash);
        }
    }

    #[test]
    fn extract_from_lambda_proof() {
        // λ (n : Nat). Constructor(Exists, intro, [n, proof_n])
        let witness = CoreTerm::Lam {
            param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
            body: Box::new(CoreTerm::Constructor {
                type_name: "Exists".into(),
                ctor_name: "intro".into(),
                args: vec![
                    CoreTerm::Var(0),
                    CoreTerm::App {
                        func: Box::new(CoreTerm::Const { name: "proof_p".into(), levels: vec![] }),
                        arg: Box::new(CoreTerm::Var(0)),
                    },
                ],
            }),
        };
        let goal = CoreTerm::Pi {
            param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
            body: Box::new(CoreTerm::Prop),
        };
        let witness_hash = witness.term_hash();

        let rules = extract_rules(&witness, &goal, witness_hash);
        assert!(!rules.is_empty());
    }
}
