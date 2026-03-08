//! Canonical invariant enumeration and search.
//!
//! The kernel's core invariant synthesis engine. Enumerates InvSyn ASTs
//! deterministically by (size, layer cost, hash). For each candidate:
//! 1. Bounded evaluation as FAST FILTER (reject bad candidates)
//! 2. Structural verification as SOUND PROOF (accept good candidates)
//!
//! The search returns Found ONLY when structural step AND structural link
//! are verified. Bounded checking alone never constitutes proof.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use super::ast::{Expr, Layer};
// eval functions used by layers internally
use super::layers::Layer as LayerTrait;
use super::layers::lia::LiaLayer;
use super::layers::polynomial::PolynomialLayer;
use super::layers::algebraic::AlgebraicLayer;
use super::normalize::ReachabilityProblem;
use super::structural::{structural_step_check, structural_link_check, structural_step_check_with_rules, structural_link_check_with_rules, StructuralVerdict};
use crate::sec::rule_db::RuleDb;
use crate::sec::prefix_ban::is_prefix_invariant;

/// Result of InvSyn search.
#[derive(Debug, Clone)]
pub enum InvSynResult {
    /// Invariant found with structurally verified Base/Step/Link.
    Found {
        inv: Expr,
        base_result: String,
        step_result: String,
        link_result: String,
        /// Step is structurally verified (not just bounded).
        step_structural: bool,
        /// Link is structurally verified (not just bounded).
        link_structural: bool,
    },
    /// No invariant found in the search space.
    Frontier {
        candidates_tried: usize,
        max_ast_size: usize,
    },
}

/// InvSyn search engine.
pub struct InvSynSearch {
    pub max_ast_size: usize,
    pub step_check_bound: u64,
}

impl InvSynSearch {
    pub fn new() -> Self {
        Self {
            max_ast_size: 10,
            step_check_bound: 500,
        }
    }

    /// Search for an invariant for the given problem.
    ///
    /// The search REQUIRES structural verification for step and link.
    /// Bounded checking is used only as a fast filter to reject candidates
    /// before the more expensive structural analysis.
    pub fn search(&self, problem: &ReachabilityProblem) -> InvSynResult {
        let candidates = self.generate_candidates(problem);
        let lia = LiaLayer::new();
        let poly = PolynomialLayer::new();
        let alg = AlgebraicLayer::new();

        let mut tried = 0;
        for inv in &candidates {
            tried += 1;

            // --- Phase 1: Bounded filter (fast reject) ---
            let layer: &dyn LayerTrait = match inv.layer() {
                Layer::LIA => &lia,
                Layer::Polynomial => &poly,
                Layer::Algebraic | Layer::Analytic => &alg,
            };

            let base = layer.check_base(inv, problem);
            if !base.passed {
                continue;
            }

            let step_bounded = layer.check_step(inv, problem);
            if !step_bounded.passed {
                continue;
            }

            let link_bounded = layer.check_link(inv, problem);
            if !link_bounded.passed {
                continue;
            }

            // --- Phase 2: Structural verification (sound proof) ---
            let step_verdict = structural_step_check(inv, problem.step_delta);
            let link_verdict = match &problem.property_expr {
                Some(prop) => structural_link_check(inv, prop),
                None => StructuralVerdict::NotVerifiable("no property_expr".into()),
            };

            let step_structural = step_verdict.is_verified();
            let link_structural = link_verdict.is_verified();

            // Accept ONLY if structurally verified
            if step_structural && link_structural {
                return InvSynResult::Found {
                    inv: inv.clone(),
                    base_result: base.description,
                    step_result: format!(
                        "STRUCTURAL: {}",
                        step_verdict.description()
                    ),
                    link_result: format!(
                        "STRUCTURAL: {}",
                        link_verdict.description()
                    ),
                    step_structural: true,
                    link_structural: true,
                };
            }
        }

        InvSynResult::Frontier {
            candidates_tried: tried,
            max_ast_size: self.max_ast_size,
        }
    }

    /// Search for an invariant using an extended rule database from SEC.
    ///
    /// Same as `search()` but uses SEC-proven rules for structural verification.
    /// Also enforces the prefix invariant ban: candidates of the form
    /// `∀m≤n, P(m)` are rejected unless their step is independently proved.
    pub fn search_with_rules(&self, problem: &ReachabilityProblem, rule_db: &RuleDb) -> InvSynResult {
        let candidates = self.generate_candidates(problem);
        let lia = LiaLayer::new();
        let poly = PolynomialLayer::new();
        let alg = AlgebraicLayer::new();

        let mut tried = 0;
        for inv in &candidates {
            tried += 1;

            // Prefix ban: reject prefix invariants whose step IS the conjecture
            if let Some(ref prop) = problem.property_expr {
                if is_prefix_invariant(inv, prop) {
                    continue;
                }
            }

            // --- Phase 1: Bounded filter (fast reject) ---
            let layer: &dyn LayerTrait = match inv.layer() {
                Layer::LIA => &lia,
                Layer::Polynomial => &poly,
                Layer::Algebraic | Layer::Analytic => &alg,
            };

            let base = layer.check_base(inv, problem);
            if !base.passed {
                continue;
            }

            let step_bounded = layer.check_step(inv, problem);
            if !step_bounded.passed {
                continue;
            }

            let link_bounded = layer.check_link(inv, problem);
            if !link_bounded.passed {
                continue;
            }

            // --- Phase 2: Structural verification with SEC rules ---
            let step_verdict = structural_step_check_with_rules(inv, problem.step_delta, Some(rule_db));
            let link_verdict = match &problem.property_expr {
                Some(prop) => structural_link_check_with_rules(inv, prop, Some(rule_db)),
                None => StructuralVerdict::NotVerifiable("no property_expr".into()),
            };

            let step_structural = step_verdict.is_verified();
            let link_structural = link_verdict.is_verified();

            if step_structural && link_structural {
                return InvSynResult::Found {
                    inv: inv.clone(),
                    base_result: base.description,
                    step_result: format!("STRUCTURAL+SEC: {}", step_verdict.description()),
                    link_result: format!("STRUCTURAL+SEC: {}", link_verdict.description()),
                    step_structural: true,
                    link_structural: true,
                };
            }
        }

        InvSynResult::Frontier {
            candidates_tried: tried,
            max_ast_size: self.max_ast_size,
        }
    }

    /// Generate invariant candidates for a problem (public API).
    pub fn generate_candidates_public(&self, problem: &ReachabilityProblem) -> Vec<Expr> {
        self.generate_candidates(problem)
    }

    /// Generate invariant candidates for a problem.
    ///
    /// Deep enumeration: generates many invariant forms including
    /// modular conditions, range conditions, conjunctions, and
    /// property-based invariants.
    fn generate_candidates(&self, problem: &ReachabilityProblem) -> Vec<Expr> {
        let mut candidates = Vec::new();

        // 1. Structural invariants that have provable step
        candidates.extend(structural_invariants(problem));

        // 2. Property-based invariants
        candidates.extend(property_based_invariants(problem));

        // 3. Problem-specific known invariants
        candidates.extend(known_invariants(&problem.problem_id));

        // 4. Generic invariant templates
        candidates.extend(generic_invariants(problem));

        // 4b. Structural bound invariants (monotone functions)
        candidates.extend(structural_bound_invariants(problem));

        // 5. Enumerated small ASTs (up to max_ast_size)
        candidates.extend(enumerate_small_asts(self.max_ast_size, problem));

        // Sort by (size, layer cost, hash) for deterministic ordering
        candidates.sort_by(|a, b| {
            let size_cmp = a.size().cmp(&b.size());
            if size_cmp != std::cmp::Ordering::Equal {
                return size_cmp;
            }
            let layer_cmp = (a.layer() as u8).cmp(&(b.layer() as u8));
            if layer_cmp != std::cmp::Ordering::Equal {
                return layer_cmp;
            }
            let ha = hash_expr(a);
            let hb = hash_expr(b);
            ha.cmp(&hb)
        });

        // Dedup by hash
        candidates.dedup_by(|a, b| hash_expr(a) == hash_expr(b));

        candidates
    }
}

/// Deterministic hash for an expression.
fn hash_expr(e: &Expr) -> u64 {
    let mut hasher = DefaultHasher::new();
    e.hash(&mut hasher);
    hasher.finish()
}

/// Generate structural invariants that have provable step.
///
/// These are invariant forms where the structural step checker
/// can verify ∀n, I(n) → I(n+δ) algebraically.
fn structural_invariants(problem: &ReachabilityProblem) -> Vec<Expr> {
    let mut invs = Vec::new();
    let init = problem.initial_value;
    let delta = problem.step_delta;

    // --- Range invariants ---
    // Le(Const(c), Var(0)) for various c values around initial_value
    for c in [0, 1, init - 1, init, init + 1].iter() {
        if *c >= 0 {
            invs.push(Expr::Le(
                Box::new(Expr::Const(*c)),
                Box::new(Expr::Var(0)),
            ));
        }
    }

    // Lt(Const(c), Var(0)) — strict lower bounds
    for c in [0, init - 1, init - 2].iter() {
        if *c >= 0 {
            invs.push(Expr::Lt(
                Box::new(Expr::Const(*c)),
                Box::new(Expr::Var(0)),
            ));
        }
    }

    // --- Modular invariants (only when delta divides modulus) ---
    // For delta=2: n mod 2 = r
    // For delta=3: n mod 3 = r
    // For delta=k: n mod k = r, n mod (2k) = r, etc.
    let moduli: Vec<i64> = if delta > 1 {
        let mut m = vec![delta, 2 * delta];
        // Also try divisors of delta
        for d in 2..=delta {
            if delta % d == 0 && !m.contains(&d) {
                m.push(d);
            }
        }
        m
    } else {
        // delta=1: only mod 1 works (trivial), so skip
        vec![]
    };

    for m in &moduli {
        let r = ((init % m) + m) % m; // init mod m (positive)
        invs.push(Expr::Eq(
            Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(*m)))),
            Box::new(Expr::Const(r)),
        ));
    }

    // --- Conjunction: range + modular ---
    let range_invs: Vec<Expr> = vec![
        Expr::Le(Box::new(Expr::Const(init)), Box::new(Expr::Var(0))),
    ];

    for m in &moduli {
        let r = ((init % m) + m) % m;
        let mod_inv = Expr::Eq(
            Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(*m)))),
            Box::new(Expr::Const(r)),
        );
        for range in &range_invs {
            invs.push(Expr::And(
                Box::new(range.clone()),
                Box::new(mod_inv.clone()),
            ));
        }
    }

    // --- Negation-based invariants ---
    // Not(Le(Var(0), Const(c))) = n > c
    if init > 0 {
        invs.push(Expr::Not(Box::new(Expr::Le(
            Box::new(Expr::Var(0)),
            Box::new(Expr::Const(init - 1)),
        ))));
    }

    invs
}

/// Generate invariants based on the problem's property expression.
///
/// If the property is available, try using it directly or in conjunction
/// with structural conditions.
fn property_based_invariants(problem: &ReachabilityProblem) -> Vec<Expr> {
    let mut invs = Vec::new();
    let init = problem.initial_value;
    let delta = problem.step_delta;

    if let Some(ref prop) = problem.property_expr {
        // Property itself as invariant — step requires P(n) → P(n+δ)
        invs.push(prop.clone());

        // Range + property conjunction — step needs both parts
        invs.push(Expr::And(
            Box::new(Expr::Le(Box::new(Expr::Const(init)), Box::new(Expr::Var(0)))),
            Box::new(prop.clone()),
        ));

        // Modular + property conjunction
        if delta > 1 {
            let r = ((init % delta) + delta) % delta;
            invs.push(Expr::And(
                Box::new(Expr::Eq(
                    Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(delta)))),
                    Box::new(Expr::Const(r)),
                )),
                Box::new(prop.clone()),
            ));

            // Range + modular + property
            invs.push(Expr::And(
                Box::new(Expr::And(
                    Box::new(Expr::Le(Box::new(Expr::Const(init)), Box::new(Expr::Var(0)))),
                    Box::new(Expr::Eq(
                        Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(delta)))),
                        Box::new(Expr::Const(r)),
                    )),
                )),
                Box::new(prop.clone()),
            ));
        }
    }

    invs
}

/// Known invariants for specific problems.
fn known_invariants(problem_id: &str) -> Vec<Expr> {
    match problem_id {
        "zfc_zero_ne_one" => vec![Expr::Const(1)],
        "mersenne" => vec![Expr::Const(1)],
        "bsd_ec" => vec![Expr::Const(1)],
        "odd_perfect" => vec![
            // ¬(odd(n) ∧ σ(n) = 2n) — the property itself
            Expr::Not(Box::new(Expr::And(
                Box::new(Expr::Ne(
                    Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(2)))),
                    Box::new(Expr::Const(0)),
                )),
                Box::new(Expr::Eq(
                    Box::new(Expr::DivisorSum(Box::new(Expr::Var(0)))),
                    Box::new(Expr::Mul(Box::new(Expr::Const(2)), Box::new(Expr::Var(0)))),
                )),
            ))),
        ],
        _ => vec![],
    }
}

/// Generate structural bound invariants using monotone functions.
/// These are invariants whose bodies step structurally (monotone).
/// For implies(le(c, var0), body) where body is a monotone bound,
/// bounded_structural_forall gives TRUE unbounded proofs.
fn structural_bound_invariants(problem: &ReachabilityProblem) -> Vec<Expr> {
    let mut invs = Vec::new();

    // Monotone function bounds — these step via lePrimeCount
    for c in 0..=3 {
        // le(c, primeCount(var0)) — monotone, steps
        invs.push(Expr::Le(
            Box::new(Expr::Const(c)),
            Box::new(Expr::PrimeCount(Box::new(Expr::Var(0)))),
        ));
    }

    // Implies with forward guard + monotone body
    // implies(le(c, var0), le(0, primeCount(var0)))
    // This is the bounded+structural pattern
    if let Some(ref prop) = problem.property_expr {
        // Try wrapping monotone bounds inside implies guards
        for guard_c in [0, 1, 2, 4, 5] {
            for body_c in 0..=2 {
                // implies(le(guard_c, var0), le(body_c, primeCount(var0)))
                invs.push(Expr::Implies(
                    Box::new(Expr::Le(
                        Box::new(Expr::Const(guard_c)),
                        Box::new(Expr::Var(0)),
                    )),
                    Box::new(Expr::Le(
                        Box::new(Expr::Const(body_c)),
                        Box::new(Expr::PrimeCount(Box::new(Expr::Var(0)))),
                    )),
                ));
            }
        }
    }

    invs
}

/// Generic invariant templates applicable to any problem.
fn generic_invariants(problem: &ReachabilityProblem) -> Vec<Expr> {
    let mut invs = Vec::new();

    // Constant true (trivial invariant — works only if property is also trivial)
    invs.push(Expr::Const(1));

    // n ≥ init (range invariant)
    invs.push(Expr::Le(
        Box::new(Expr::Const(problem.initial_value)),
        Box::new(Expr::Var(0)),
    ));

    // n ≥ 0
    if problem.initial_value > 0 {
        invs.push(Expr::Le(
            Box::new(Expr::Const(0)),
            Box::new(Expr::Var(0)),
        ));
    }

    invs
}

/// Enumerate small InvSyn ASTs up to a given size.
fn enumerate_small_asts(max_size: usize, problem: &ReachabilityProblem) -> Vec<Expr> {
    let mut asts = Vec::new();
    let init = problem.initial_value;
    let delta = problem.step_delta;

    // Size 1: atoms
    if max_size >= 1 {
        asts.push(Expr::Var(0));
        asts.push(Expr::Const(0));
        asts.push(Expr::Const(1));
        asts.push(Expr::Const(2));
        if init > 2 {
            asts.push(Expr::Const(init));
        }
    }

    // Size 2: unary operations on atoms
    if max_size >= 2 {
        asts.push(Expr::Neg(Box::new(Expr::Var(0))));
        asts.push(Expr::Not(Box::new(Expr::Var(0))));
        asts.push(Expr::IsPrime(Box::new(Expr::Var(0))));
    }

    // Size 3: binary operations on atoms
    if max_size >= 3 {
        let key_consts: Vec<i64> = {
            let mut v = vec![0, 1, 2];
            if init > 2 && !v.contains(&init) { v.push(init); }
            if delta > 1 && !v.contains(&delta) { v.push(delta); }
            v
        };

        for c in &key_consts {
            // Le(Const(c), Var(0)) — n ≥ c
            asts.push(Expr::Le(Box::new(Expr::Const(*c)), Box::new(Expr::Var(0))));
            // Le(Var(0), Const(c)) — n ≤ c
            asts.push(Expr::Le(Box::new(Expr::Var(0)), Box::new(Expr::Const(*c))));
            // Eq comparisons
            asts.push(Expr::Eq(Box::new(Expr::Var(0)), Box::new(Expr::Const(*c))));
            // Modular: Mod(Var(0), Const(c))
            if *c > 1 {
                asts.push(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(*c))));
            }
        }

        // Modular equality: Eq(Mod(Var(0), Const(m)), Const(r))
        for m in &key_consts {
            if *m > 1 {
                for r in 0..*m {
                    asts.push(Expr::Eq(
                        Box::new(Expr::Mod(Box::new(Expr::Var(0)), Box::new(Expr::Const(*m)))),
                        Box::new(Expr::Const(r)),
                    ));
                }
            }
        }
    }

    asts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invsyn::normalize::normalize;

    #[test]
    fn search_zfc() {
        let engine = InvSynSearch::new();
        let problem = normalize("zfc_zero_ne_one");
        let result = engine.search(&problem);
        match result {
            InvSynResult::Found { inv, step_structural, link_structural, .. } => {
                // ZFC should find Const(1) with structural verification
                assert!(step_structural, "ZFC step must be structural");
                assert!(link_structural, "ZFC link must be structural");
                // Const(1) works because property is Const(1)
                assert_eq!(inv, Expr::Const(1));
            }
            InvSynResult::Frontier { .. } => {
                panic!("Expected Found for ZFC");
            }
        }
    }

    #[test]
    fn search_zfc_structural() {
        // ZFC must produce structurally verified proofs
        let engine = InvSynSearch::new();
        let problem = normalize("zfc_zero_ne_one");
        let result = engine.search(&problem);
        match result {
            InvSynResult::Found { step_result, link_result, .. } => {
                assert!(step_result.contains("STRUCTURAL"));
                assert!(link_result.contains("STRUCTURAL"));
            }
            _ => panic!("Expected Found"),
        }
    }

    #[test]
    fn search_mersenne_structural() {
        // Mersenne property is Const(1), so Const(1) invariant should work
        let engine = InvSynSearch::new();
        let problem = normalize("mersenne");
        let result = engine.search(&problem);
        match result {
            InvSynResult::Found { step_structural, link_structural, .. } => {
                assert!(step_structural);
                assert!(link_structural);
            }
            _ => panic!("Expected Found for mersenne (trivial property)"),
        }
    }

    #[test]
    fn search_bsd_ec_structural() {
        // BSD EC property is Const(1), so Const(1) invariant should work
        let engine = InvSynSearch::new();
        let problem = normalize("bsd_ec");
        let result = engine.search(&problem);
        match result {
            InvSynResult::Found { step_structural, link_structural, .. } => {
                assert!(step_structural);
                assert!(link_structural);
            }
            _ => panic!("Expected Found for bsd_ec (trivial property)"),
        }
    }

    #[test]
    fn search_deterministic() {
        let engine = InvSynSearch::new();
        let problem = normalize("zfc_zero_ne_one");
        let r1 = engine.search(&problem);
        let r2 = engine.search(&problem);
        match (r1, r2) {
            (InvSynResult::Found { inv: i1, .. }, InvSynResult::Found { inv: i2, .. }) => {
                assert_eq!(hash_expr(&i1), hash_expr(&i2));
            }
            (InvSynResult::Frontier { candidates_tried: c1, .. },
             InvSynResult::Frontier { candidates_tried: c2, .. }) => {
                assert_eq!(c1, c2);
            }
            _ => panic!("Results should be same type"),
        }
    }

    #[test]
    fn search_generates_structural_candidates() {
        // Goldbach should generate range+modular invariants
        let engine = InvSynSearch::new();
        let problem = normalize("goldbach");
        let candidates = engine.generate_candidates_public(&problem);
        // Should contain Le(Const(4), Var(0))
        let has_range = candidates.iter().any(|c| {
            matches!(c, Expr::Le(l, r)
                if matches!(l.as_ref(), Expr::Const(4))
                    && matches!(r.as_ref(), Expr::Var(0)))
        });
        assert!(has_range, "Should generate n ≥ 4 candidate");
        // Should contain modular: n mod 2 = 0
        let has_mod = candidates.iter().any(|c| {
            matches!(c, Expr::Eq(l, r)
                if matches!(l.as_ref(), Expr::Mod(..))
                    && matches!(r.as_ref(), Expr::Const(0)))
        });
        assert!(has_mod, "Should generate n mod 2 = 0 candidate");
    }

    #[test]
    fn goldbach_is_frontier_structurally() {
        // Goldbach should be FRONTIER — no invariant has both structural step AND link
        let engine = InvSynSearch::new();
        let problem = normalize("goldbach");
        let result = engine.search(&problem);
        match result {
            InvSynResult::Frontier { candidates_tried, .. } => {
                assert!(candidates_tried > 10, "Should try many candidates before frontier");
            }
            InvSynResult::Found { .. } => {
                panic!("Goldbach should NOT be Found with structural verification — it's an open problem!");
            }
        }
    }

    #[test]
    fn collatz_is_frontier_structurally() {
        let engine = InvSynSearch::new();
        let problem = normalize("collatz");
        let result = engine.search(&problem);
        assert!(matches!(result, InvSynResult::Frontier { .. }));
    }

    #[test]
    fn frontier_problems_remain_frontier() {
        let engine = InvSynSearch::new();
        for id in &["p_vs_np", "riemann_full", "navier_stokes", "yang_mills", "hodge", "bsd_full"] {
            let problem = normalize(id);
            let result = engine.search(&problem);
            assert!(
                matches!(result, InvSynResult::Frontier { .. }),
                "{} should be frontier", id
            );
        }
    }
}
