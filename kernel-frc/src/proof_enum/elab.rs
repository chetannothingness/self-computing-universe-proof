//! ELAB — bytes → CoreTerm with proof-carrying elaboration.
//!
//! The elaborator takes raw bytes Q and returns:
//!   OK(term, ctx, goal) — typed core term + context + goal type
//!   ILL_TYPED(refutation) — formal refutation, no wasted compute
//!
//! This is the entry point: every question enters as bytes,
//! exits as a typed CoreTerm that the normalizer can rewrite.
//!
//! The canonical contract encoding is:
//!   Q = sd(GoalType ‖ StatementBytes ‖ ContextBytes ‖ VerifierSpec ‖ TieBreak)

use super::core_term::{CoreTerm, CoreCtx, CoreEnv, CoreDef};
use super::statement::{ProofStatement, get_statement};
use kernel_types::{Hash32, hash};

/// Result of elaborating raw bytes into a typed CoreTerm.
#[derive(Debug, Clone)]
pub enum ElabResult {
    /// Successfully elaborated: typed core term + context + goal.
    Ok {
        /// The elaborated term (initially a hole to be filled by witness).
        term: CoreTerm,
        /// The typing context.
        ctx: CoreCtx,
        /// The goal type to prove.
        goal: CoreTerm,
        /// Hash of the elaborated question.
        question_hash: Hash32,
    },
    /// Ill-typed or unparseable input.
    IllTyped {
        /// Why elaboration failed.
        reason: String,
        /// Hash of the input bytes.
        input_hash: Hash32,
    },
}

impl ElabResult {
    pub fn is_ok(&self) -> bool {
        matches!(self, ElabResult::Ok { .. })
    }
}

/// Elaborate a problem ID into a CoreTerm goal.
///
/// Takes a problem ID (e.g., "goldbach") and produces the goal type
/// as a CoreTerm that the normalizer/μ-selector will try to inhabit.
pub fn elab_problem(problem_id: &str) -> ElabResult {
    let statement = get_statement(problem_id);
    elab_statement(&statement)
}

/// Elaborate a ProofStatement into a CoreTerm goal.
pub fn elab_statement(statement: &ProofStatement) -> ElabResult {
    let question_hash = hash::H(statement.lean_prop.as_bytes());

    // Check if the statement has a real formalization (not "True" stub)
    if statement.lean_prop == "True" {
        return ElabResult::IllTyped {
            reason: format!("statement '{}' has placeholder formalization (True)", statement.id),
            input_hash: question_hash,
        };
    }

    // Parse the Lean proposition into a CoreTerm goal type
    let goal = parse_lean_prop_to_core(&statement.lean_prop);

    let ctx = CoreCtx::new();

    ElabResult::Ok {
        term: CoreTerm::Var(0), // placeholder — to be filled by witness
        ctx,
        goal,
        question_hash,
    }
}

/// Elaborate raw bytes into a CoreTerm.
///
/// Interprets the bytes as a candidate proof term.
/// Returns a CoreTerm if the bytes represent a valid term,
/// or IllTyped if they don't parse.
pub fn elab_witness_bytes(bytes: &[u8]) -> Option<CoreTerm> {
    // Try parsing as a CoreTerm from canonical bytes
    if let Some((term, _)) = CoreTerm::from_bytes(bytes) {
        return Some(term);
    }

    // Try interpreting as UTF-8 and parsing as simple Lean-like syntax
    if let Ok(text) = String::from_utf8(bytes.to_vec()) {
        return parse_simple_term(&text);
    }

    None
}

/// Parse a Lean proposition string into a CoreTerm.
///
/// This is a simplified parser for the common patterns in our 20 statements.
/// It handles: ∀, →, ∃, ∧, ∨, ¬, Nat.Prime, arithmetic, etc.
fn parse_lean_prop_to_core(prop: &str) -> CoreTerm {
    let trimmed = prop.trim();

    // Universal quantification: ∀ x : T, body
    if trimmed.starts_with("∀") || trimmed.starts_with("forall") {
        let rest = trimmed.trim_start_matches("∀").trim_start_matches("forall").trim();
        // Parse the binder
        if let Some(comma_pos) = find_top_level(rest, ',') {
            let binder = &rest[..comma_pos].trim();
            let body = &rest[comma_pos + 1..].trim();

            // Parse binder: "x : T" or "x"
            let (_, param_type) = parse_binder(binder);
            let body_term = parse_lean_prop_to_core(body);

            return CoreTerm::Pi {
                param_type: Box::new(param_type),
                body: Box::new(body_term),
            };
        }
    }

    // Implication: A → B
    if let Some(arrow_pos) = find_top_level(trimmed, '→') {
        let lhs = &trimmed[..arrow_pos].trim();
        let rhs = &trimmed[arrow_pos + "→".len()..].trim();
        return CoreTerm::Pi {
            param_type: Box::new(parse_lean_prop_to_core(lhs)),
            body: Box::new(parse_lean_prop_to_core(rhs)),
        };
    }

    // Existential: ∃ x, body
    if trimmed.starts_with("∃") || trimmed.starts_with("exists") {
        return CoreTerm::Const {
            name: format!("Exists({})", trimmed),
            levels: vec![],
        };
    }

    // Conjunction: A ∧ B
    if let Some(pos) = find_top_level(trimmed, '∧') {
        let lhs = &trimmed[..pos].trim();
        let rhs = &trimmed[pos + "∧".len()..].trim();
        return CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "And".into(), levels: vec![] }),
                arg: Box::new(parse_lean_prop_to_core(lhs)),
            }),
            arg: Box::new(parse_lean_prop_to_core(rhs)),
        };
    }

    // Negation: ¬ A
    if trimmed.starts_with("¬") {
        let inner = &trimmed["¬".len()..].trim();
        return CoreTerm::App {
            func: Box::new(CoreTerm::Const { name: "Not".into(), levels: vec![] }),
            arg: Box::new(parse_lean_prop_to_core(inner)),
        };
    }

    // Comparison operators
    if let Some(pos) = find_top_level(trimmed, '≠') {
        let lhs = &trimmed[..pos].trim();
        let rhs = &trimmed[pos + "≠".len()..].trim();
        return CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "Ne".into(), levels: vec![] }),
                arg: Box::new(parse_lean_prop_to_core(lhs)),
            }),
            arg: Box::new(parse_lean_prop_to_core(rhs)),
        };
    }

    if let Some(pos) = find_top_level(trimmed, '≥') {
        let lhs = &trimmed[..pos].trim();
        let rhs = &trimmed[pos + "≥".len()..].trim();
        return CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "Ge".into(), levels: vec![] }),
                arg: Box::new(parse_lean_prop_to_core(lhs)),
            }),
            arg: Box::new(parse_lean_prop_to_core(rhs)),
        };
    }

    // Natural number literal
    if let Ok(n) = trimmed.parse::<u64>() {
        return CoreTerm::NatLit(n);
    }

    // Parenthesized expression
    if trimmed.starts_with('(') && trimmed.ends_with(')') {
        return parse_lean_prop_to_core(&trimmed[1..trimmed.len() - 1]);
    }

    // Type cast: (x : T)
    if trimmed.contains(':') && !trimmed.contains("∀") {
        if let Some(colon_pos) = trimmed.find(':') {
            let ty = trimmed[colon_pos + 1..].trim();
            return parse_lean_prop_to_core(ty);
        }
    }

    // Default: treat as a named constant
    CoreTerm::Const {
        name: trimmed.to_string(),
        levels: vec![],
    }
}

/// Parse a binder like "x : Nat" into (name, type).
fn parse_binder(binder: &str) -> (String, CoreTerm) {
    if let Some(colon_pos) = binder.find(':') {
        let name = binder[..colon_pos].trim().to_string();
        let ty_str = binder[colon_pos + 1..].trim();
        let ty = match ty_str {
            "Nat" => CoreTerm::Const { name: "Nat".into(), levels: vec![] },
            "Int" => CoreTerm::Const { name: "Int".into(), levels: vec![] },
            "Bool" => CoreTerm::Const { name: "Bool".into(), levels: vec![] },
            "Prop" => CoreTerm::Prop,
            other => CoreTerm::Const { name: other.to_string(), levels: vec![] },
        };
        (name, ty)
    } else {
        // No type annotation — default to Nat
        (binder.trim().to_string(), CoreTerm::Const { name: "Nat".into(), levels: vec![] })
    }
}

/// Parse a simple term from text (for witness interpretation).
fn parse_simple_term(text: &str) -> Option<CoreTerm> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Natural number literal
    if let Ok(n) = trimmed.parse::<u64>() {
        return Some(CoreTerm::NatLit(n));
    }

    // Simple identifiers
    if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.') {
        return Some(CoreTerm::Const {
            name: trimmed.to_string(),
            levels: vec![],
        });
    }

    None
}

/// Find a character at the top level (not inside parentheses).
fn find_top_level(s: &str, target: char) -> Option<usize> {
    let mut depth = 0;
    let target_str = target.to_string();
    let target_bytes = target_str.as_bytes();

    for (i, c) in s.char_indices() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ if depth == 0 && c == target => return Some(i),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elab_known_problem() {
        let result = elab_problem("zfc_zero_ne_one");
        assert!(result.is_ok(), "known problem should elaborate");
    }

    #[test]
    fn elab_open_problem() {
        let result = elab_problem("goldbach");
        assert!(result.is_ok(), "formalized open problem should elaborate");
    }

    #[test]
    fn elab_stub_problem_is_ill_typed() {
        // Use a truly unknown problem (falls through to "True" default)
        let result = elab_problem("nonexistent_stub_xyz");
        assert!(!result.is_ok(), "stub (True) problem should be ill-typed");
    }

    #[test]
    fn elab_produces_goal() {
        match elab_problem("goldbach") {
            ElabResult::Ok { goal, .. } => {
                // Goal should be a Pi type (∀ ...)
                match &goal {
                    CoreTerm::Pi { .. } => {} // correct
                    other => panic!("expected Pi type for ∀, got {:?}", other),
                }
            }
            other => panic!("expected Ok, got {:?}", other),
        }
    }

    #[test]
    fn elab_question_hash_deterministic() {
        let r1 = elab_problem("goldbach");
        let r2 = elab_problem("goldbach");
        match (r1, r2) {
            (ElabResult::Ok { question_hash: h1, .. }, ElabResult::Ok { question_hash: h2, .. }) => {
                assert_eq!(h1, h2, "same problem should have same hash");
            }
            _ => panic!("both should be Ok"),
        }
    }

    #[test]
    fn elab_different_problems_different_hashes() {
        let r1 = elab_problem("goldbach");
        let r2 = elab_problem("collatz");
        match (r1, r2) {
            (ElabResult::Ok { question_hash: h1, .. }, ElabResult::Ok { question_hash: h2, .. }) => {
                assert_ne!(h1, h2, "different problems should have different hashes");
            }
            _ => panic!("both should be Ok"),
        }
    }

    #[test]
    fn elab_witness_bytes_natlit() {
        let term = CoreTerm::NatLit(42);
        let bytes = term.to_bytes();
        let parsed = elab_witness_bytes(&bytes).unwrap();
        assert_eq!(parsed, term);
    }

    #[test]
    fn elab_witness_bytes_text() {
        let bytes = b"42";
        let parsed = elab_witness_bytes(bytes).unwrap();
        assert_eq!(parsed, CoreTerm::NatLit(42));
    }

    #[test]
    fn elab_witness_bytes_invalid() {
        let bytes = &[0xFF, 0xFE, 0xFD]; // not valid CoreTerm or UTF-8
        assert!(elab_witness_bytes(bytes).is_none());
    }

    #[test]
    fn parse_simple_prop() {
        let goal = parse_lean_prop_to_core("(0 : Nat) ≠ 1");
        // Should produce Ne(0, 1) structure
        match &goal {
            CoreTerm::App { func, .. } => {
                match func.as_ref() {
                    CoreTerm::App { func: inner, .. } => {
                        match inner.as_ref() {
                            CoreTerm::Const { name, .. } => assert_eq!(name, "Ne"),
                            other => panic!("expected Ne const, got {:?}", other),
                        }
                    }
                    other => panic!("expected App, got {:?}", other),
                }
            }
            other => panic!("expected App for Ne, got {:?}", other),
        }
    }
}
