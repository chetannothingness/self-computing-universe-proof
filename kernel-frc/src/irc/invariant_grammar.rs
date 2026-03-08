// Invariant grammar — enumerate candidate invariants for IRC synthesis.
//
// Five invariant schemas, each producing candidates for a given transition system:
// 1. PrefixSchema:     I(n) = "P holds for all m ≤ n"
// 2. BoundingSchema:   I(n) = "f(n) ≤ g(n)" for monotone/bounded functions
// 3. ModularSchema:    I(n) = property depending on n mod k
// 4. StructuralSchema: I(n) = "state in finite set S"
// 5. SpecializedSchema: problem-specific known invariants

use crate::frc_types::{Invariant, InvariantKind, TransitionSystem};
use super::problem_invariants;

/// Invariant schema — generates candidate invariants for a transition system.
pub trait InvariantSchema: Send + Sync {
    fn id(&self) -> InvariantKind;
    fn name(&self) -> &str;
    fn cost(&self) -> u64;

    /// Generate candidate invariants for the given transition system.
    fn candidates(
        &self,
        ts: &TransitionSystem,
        problem_id: &str,
        max_candidates: usize,
    ) -> Vec<Invariant>;
}

// === Schema 1: Prefix ===

pub struct PrefixSchema;

impl InvariantSchema for PrefixSchema {
    fn id(&self) -> InvariantKind { InvariantKind::Prefix }
    fn name(&self) -> &str { "Prefix" }
    fn cost(&self) -> u64 { 1 }

    fn candidates(
        &self,
        ts: &TransitionSystem,
        _problem_id: &str,
        _max_candidates: usize,
    ) -> Vec<Invariant> {
        // Universal prefix invariant: I(n) = ∀m ≤ n, P(m)
        vec![
            Invariant::new(
                InvariantKind::Prefix,
                format!("∀m ≤ n, {}", ts.property_desc),
                format!(
                    "def prefixInvariant (n : Nat) : Prop := ∀ m, m ≤ n → {}",
                    ts.property_desc
                ),
            ),
        ]
    }
}

// === Schema 2: Bounding ===

pub struct BoundingSchema;

impl InvariantSchema for BoundingSchema {
    fn id(&self) -> InvariantKind { InvariantKind::Bounding }
    fn name(&self) -> &str { "Bounding" }
    fn cost(&self) -> u64 { 2 }

    fn candidates(
        &self,
        ts: &TransitionSystem,
        problem_id: &str,
        _max_candidates: usize,
    ) -> Vec<Invariant> {
        // Bounding invariants are problem-specific
        match problem_id {
            "mertens" => vec![
                Invariant::new(
                    InvariantKind::Bounding,
                    "|M(n)| ≤ √n — Mertens function bounded by square root".to_string(),
                    "def mertensInvariant (n : Nat) : Prop := M(n) * M(n) ≤ n".to_string(),
                ),
            ],
            "bsd_ec" => vec![
                Invariant::new(
                    InvariantKind::Bounding,
                    "|#E(F_p) - (p+1)| ≤ 2√p — Hasse bound".to_string(),
                    "def hasseInvariant (p : Nat) : Prop := |cardE p - (p + 1)| ≤ 2 * Nat.sqrt p".to_string(),
                ),
            ],
            _ => {
                // Generic bounding: P(n) ∧ bound(n) — only if property mentions bounds
                if ts.property_desc.contains('≤') || ts.property_desc.contains('√') {
                    vec![
                        Invariant::new(
                            InvariantKind::Bounding,
                            format!("bound-based invariant for {}", ts.property_desc),
                            format!(
                                "def boundInvariant (n : Nat) : Prop := {}",
                                ts.property_desc
                            ),
                        ),
                    ]
                } else {
                    vec![]
                }
            }
        }
    }
}

// === Schema 3: Modular ===

pub struct ModularSchema;

impl InvariantSchema for ModularSchema {
    fn id(&self) -> InvariantKind { InvariantKind::Modular }
    fn name(&self) -> &str { "Modular" }
    fn cost(&self) -> u64 { 3 }

    fn candidates(
        &self,
        _ts: &TransitionSystem,
        problem_id: &str,
        _max_candidates: usize,
    ) -> Vec<Invariant> {
        match problem_id {
            "goldbach" => vec![
                Invariant::new(
                    InvariantKind::Modular,
                    "Even/odd split: only even n ≥ 4 need Goldbach property".to_string(),
                    "def goldbachModular (n : Nat) : Prop := n % 2 = 1 ∨ n < 4 ∨ isSumOfTwoPrimes n".to_string(),
                ),
            ],
            "erdos_straus" => vec![
                Invariant::new(
                    InvariantKind::Modular,
                    "Modular structure: 4/n decomposition depends on n mod 4".to_string(),
                    "def erdosStrausModular (n : Nat) : Prop := n < 2 ∨ ∃ x y z, 4 * x * y * z = n * (y * z + x * z + x * y)".to_string(),
                ),
            ],
            _ => vec![],
        }
    }
}

// === Schema 4: Structural ===

pub struct StructuralSchema;

impl InvariantSchema for StructuralSchema {
    fn id(&self) -> InvariantKind { InvariantKind::Structural }
    fn name(&self) -> &str { "Structural" }
    fn cost(&self) -> u64 { 4 }

    fn candidates(
        &self,
        _ts: &TransitionSystem,
        problem_id: &str,
        _max_candidates: usize,
    ) -> Vec<Invariant> {
        match problem_id {
            "collatz" => vec![
                Invariant::new(
                    InvariantKind::Structural,
                    "Trajectory structure: Collatz orbit eventually enters {4,2,1}".to_string(),
                    "def collatzStructural (n : Nat) : Prop := ∃ k, Nat.iterate collatzStep k n = 1".to_string(),
                ),
            ],
            "zfc_zero_ne_one" => vec![
                Invariant::new(
                    InvariantKind::Structural,
                    "Trivial: 0 ≠ 1 holds unconditionally".to_string(),
                    "def zfcStructural (_ : Nat) : Prop := (0 : Nat) ≠ 1".to_string(),
                ),
            ],
            _ => vec![],
        }
    }
}

// === Schema 5: Specialized ===

pub struct SpecializedSchema;

impl InvariantSchema for SpecializedSchema {
    fn id(&self) -> InvariantKind { InvariantKind::Specialized }
    fn name(&self) -> &str { "Specialized" }
    fn cost(&self) -> u64 { 5 }

    fn candidates(
        &self,
        _ts: &TransitionSystem,
        problem_id: &str,
        _max_candidates: usize,
    ) -> Vec<Invariant> {
        // Delegate to known invariants from problem_invariants module
        problem_invariants::known_invariants(problem_id)
    }
}

// === Schema 6: InvSyn (Structural Invariant Synthesis) ===

pub struct InvSynSchema;

impl InvariantSchema for InvSynSchema {
    fn id(&self) -> InvariantKind { InvariantKind::Specialized }
    fn name(&self) -> &str { "InvSyn" }
    fn cost(&self) -> u64 { 6 }

    fn candidates(
        &self,
        _ts: &TransitionSystem,
        problem_id: &str,
        max_candidates: usize,
    ) -> Vec<Invariant> {
        use crate::invsyn::{InvSynSearch, normalize};

        let problem = normalize(problem_id);
        let engine = InvSynSearch::new();
        let candidates = engine.generate_candidates_public(&problem);

        candidates.into_iter()
            .take(max_candidates)
            .map(|expr| {
                Invariant::new(
                    InvariantKind::Specialized,
                    format!("InvSyn: {:?}", expr),
                    format!("-- InvSyn AST: {}", expr.to_lean()),
                )
            })
            .collect()
    }
}

/// Build the canonical list of invariant schemas (Π-ordered by cost).
pub fn build_invariant_schemas() -> Vec<Box<dyn InvariantSchema>> {
    vec![
        Box::new(PrefixSchema),
        Box::new(BoundingSchema),
        Box::new(ModularSchema),
        Box::new(StructuralSchema),
        Box::new(SpecializedSchema),
        Box::new(InvSynSchema),
    ]
}

/// Enumerate all candidate invariants for a problem, in canonical order.
pub fn enumerate_candidates(
    ts: &TransitionSystem,
    problem_id: &str,
    max_per_schema: usize,
) -> Vec<Invariant> {
    let schemas = build_invariant_schemas();
    let mut candidates = Vec::new();
    for schema in &schemas {
        let cs = schema.candidates(ts, problem_id, max_per_schema);
        candidates.extend(cs);
    }
    candidates
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::irc::transition_system::build_transition_system;

    #[test]
    fn all_problems_produce_candidates() {
        let problems = [
            "goldbach", "collatz", "twin_primes", "flt", "odd_perfect",
            "mersenne", "zfc_zero_ne_one", "mertens", "legendre", "erdos_straus",
            "bsd_ec", "weak_goldbach", "bertrand", "lagrange",
        ];
        for id in &problems {
            let ts = build_transition_system(id);
            let candidates = enumerate_candidates(&ts, id, 10);
            // Every problem should get at least the prefix invariant
            assert!(!candidates.is_empty(), "no candidates for {}", id);
        }
    }

    #[test]
    fn proved_theorems_get_specialized() {
        for id in &["zfc_zero_ne_one", "bertrand", "lagrange", "weak_goldbach"] {
            let ts = build_transition_system(id);
            let candidates = enumerate_candidates(&ts, id, 10);
            let has_specialized = candidates.iter().any(|c| c.kind == InvariantKind::Specialized);
            assert!(has_specialized, "no specialized invariant for {}", id);
        }
    }

    #[test]
    fn schema_order_is_canonical() {
        let schemas = build_invariant_schemas();
        for i in 1..schemas.len() {
            assert!(schemas[i-1].cost() <= schemas[i].cost());
        }
    }
}
