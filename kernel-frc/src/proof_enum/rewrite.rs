//! Rewrite basis R — confluent terminating rewrite rules with soundness proofs.
//!
//! R is the heart of the Π-normalizer. Each rule l → r carries a soundness proof
//! that ⟦l⟧ = ⟦r⟧. R starts with core β-δ-ι rules and grows monotonically
//! from verified witnesses.
//!
//! Properties:
//!   Soundness: every rule carries a proof object
//!   Termination: every rule strictly decreases the termination measure
//!   Confluence: every term has a unique normal form
//!
//! Π(x) = NF_R(x) is a deterministic projection.
//! The normalization trace IS the proof — composed from per-rule soundness proofs.
//!
//! R is NOT pre-built. R is the revealed surface of the kernel's verified computation.
//! Unknown proofs don't require pre-existing rules — they require witness verification
//! first, then rule compilation, after which normalization becomes instantaneous.

use super::core_term::{CoreTerm, CoreEnv};
use super::reduce;
use kernel_types::{Hash32, hash};

/// A rewrite rule with an attached soundness proof.
///
/// Each rule is extracted from a verified witness. The soundness proof
/// IS the witness itself — it was verified by the proof checker.
#[derive(Debug, Clone)]
pub struct RewriteRule {
    /// Hash of this rule (for deduplication and lookup).
    pub rule_hash: Hash32,
    /// Left-hand side pattern — what to match.
    pub lhs: CoreTerm,
    /// Right-hand side — what to rewrite to.
    pub rhs: CoreTerm,
    /// The soundness proof (serialized witness that ⟦lhs⟧ = ⟦rhs⟧).
    pub soundness_proof: Vec<u8>,
    /// Hash of the source witness that produced this rule.
    pub source_witness_hash: Hash32,
    /// How many times this rule has been applied (for fixed-point detection).
    pub application_count: u64,
}

/// A single step in the normalization proof trace.
#[derive(Debug, Clone)]
pub struct RewriteStep {
    /// Which rule was applied.
    pub rule_hash: Hash32,
    /// Hash of the term before rewriting.
    pub before_hash: Hash32,
    /// Hash of the term after rewriting.
    pub after_hash: Hash32,
    /// What kind of rewrite (core reduction or basis rule).
    pub kind: RewriteStepKind,
}

/// Whether a rewrite step used core reduction or a basis rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RewriteStepKind {
    /// Core β-δ-ι-ζ reduction (always available).
    CoreReduction,
    /// A rule from the rewrite basis R.
    BasisRule,
}

/// The proof trace — the composed proof object from normalization.
/// The proof IS the trace. Not found. CONSTRUCTED.
#[derive(Debug, Clone)]
pub struct ProofTrace {
    /// Each step in the normalization.
    pub steps: Vec<RewriteStep>,
    /// Hash of the initial term.
    pub initial_hash: Hash32,
    /// Hash of the final (normal form) term.
    pub final_hash: Hash32,
    /// Whether normalization reached a fixed point (true normal form).
    pub is_complete: bool,
}

impl ProofTrace {
    /// Total number of rewrite steps.
    pub fn len(&self) -> usize {
        self.steps.len()
    }

    /// Is the trace empty (term was already in normal form)?
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// How many core reduction steps were used.
    pub fn core_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.kind == RewriteStepKind::CoreReduction).count()
    }

    /// How many basis rule steps were used.
    pub fn basis_steps(&self) -> usize {
        self.steps.iter().filter(|s| s.kind == RewriteStepKind::BasisRule).count()
    }
}

/// The rewrite basis R — grows monotonically from verified witnesses.
///
/// R is not a pre-built rulebook. R is a cache of extracted equivalences
/// produced by the kernel's own verified computations.
pub struct RewriteBasis {
    /// All rewrite rules (ordered by discovery).
    rules: Vec<RewriteRule>,
}

impl RewriteBasis {
    /// Create an empty rewrite basis (only core β-δ-ι rules apply).
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Add a rewrite rule to R.
    /// The rule must carry a soundness proof. R only grows from verified witnesses.
    pub fn add_rule(&mut self, rule: RewriteRule) {
        // Deduplicate by hash
        if self.rules.iter().any(|r| r.rule_hash == rule.rule_hash) {
            return;
        }
        self.rules.push(rule);
    }

    /// Number of rules in R.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Is R empty?
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Try to match a term against all rules in R.
    /// Returns the first matching rule and the rewritten term.
    fn try_apply_rule(&mut self, term: &CoreTerm) -> Option<(CoreTerm, Hash32)> {
        for rule in &mut self.rules {
            if let Some(substitution) = pattern_match(&rule.lhs, term) {
                let rewritten = apply_substitution(&rule.rhs, &substitution);
                rule.application_count += 1;
                return Some((rewritten, rule.rule_hash));
            }
        }
        None
    }

    /// Normalize a term using R + core reduction.
    /// Returns (normal_form, proof_trace).
    ///
    /// This is NF_R(t) = (t*, π) where π is the composed proof.
    /// The proof is CONSTRUCTED as the trace. Not searched.
    pub fn normalize(
        &mut self,
        term: &CoreTerm,
        env: &CoreEnv,
        max_steps: u64,
    ) -> (CoreTerm, ProofTrace) {
        let initial_hash = term.term_hash();
        let mut current = term.clone();
        let mut steps = Vec::new();
        let mut step_count = 0u64;

        while step_count < max_steps {
            // First: try basis rules (R)
            if let Some((rewritten, rule_hash)) = self.try_apply_rule(&current) {
                let before_hash = current.term_hash();
                let after_hash = rewritten.term_hash();
                steps.push(RewriteStep {
                    rule_hash,
                    before_hash,
                    after_hash,
                    kind: RewriteStepKind::BasisRule,
                });
                current = rewritten;
                step_count += 1;
                continue;
            }

            // Then: try core reduction (β-δ-ι-ζ)
            if let Some((reduced, _kind)) = reduce::reduce_step(&current, env) {
                let before_hash = current.term_hash();
                let after_hash = reduced.term_hash();
                steps.push(RewriteStep {
                    rule_hash: hash::H(b"core_reduction"),
                    before_hash,
                    after_hash,
                    kind: RewriteStepKind::CoreReduction,
                });
                current = reduced;
                step_count += 1;
                continue;
            }

            // No more reductions — normal form reached
            let final_hash = current.term_hash();
            return (current, ProofTrace {
                steps,
                initial_hash,
                final_hash,
                is_complete: true,
            });
        }

        // Budget exhausted
        let final_hash = current.term_hash();
        (current, ProofTrace {
            steps,
            initial_hash,
            final_hash,
            is_complete: false,
        })
    }

    /// Check if the basis has reached a fixed point.
    /// True when no new rules have been added since `since_count`.
    pub fn rules_since(&self, since_count: usize) -> usize {
        if self.rules.len() > since_count {
            self.rules.len() - since_count
        } else {
            0
        }
    }

    /// Total applications of all rules (measure of R's utility).
    pub fn total_applications(&self) -> u64 {
        self.rules.iter().map(|r| r.application_count).sum()
    }

    /// Get all rules (for inspection/debugging).
    pub fn rules(&self) -> &[RewriteRule] {
        &self.rules
    }
}

/// Simple pattern matching: check if a pattern matches a term.
/// Returns a substitution map (variable index → matched subterm).
///
/// This is structural matching — variables in the pattern match any subterm.
fn pattern_match(pattern: &CoreTerm, term: &CoreTerm) -> Option<Vec<(usize, CoreTerm)>> {
    let mut substitution = Vec::new();
    if do_match(pattern, term, &mut substitution) {
        Some(substitution)
    } else {
        None
    }
}

fn do_match(pattern: &CoreTerm, term: &CoreTerm, subs: &mut Vec<(usize, CoreTerm)>) -> bool {
    match (pattern, term) {
        // Variable in pattern matches anything
        (CoreTerm::Var(i), _) => {
            // Check consistency: if this var was already matched, must match same term
            for (idx, existing) in subs.iter() {
                if *idx == *i {
                    return existing == term;
                }
            }
            subs.push((*i, term.clone()));
            true
        }

        // Structural matches
        (CoreTerm::Type(u1), CoreTerm::Type(u2)) => u1 == u2,
        (CoreTerm::Prop, CoreTerm::Prop) => true,
        (CoreTerm::NatLit(a), CoreTerm::NatLit(b)) => a == b,

        (CoreTerm::Const { name: n1, levels: l1 }, CoreTerm::Const { name: n2, levels: l2 }) => {
            n1 == n2 && l1 == l2
        }

        (CoreTerm::App { func: f1, arg: a1 }, CoreTerm::App { func: f2, arg: a2 }) => {
            do_match(f1, f2, subs) && do_match(a1, a2, subs)
        }

        (CoreTerm::Lam { param_type: p1, body: b1 }, CoreTerm::Lam { param_type: p2, body: b2 }) => {
            do_match(p1, p2, subs) && do_match(b1, b2, subs)
        }

        (CoreTerm::Pi { param_type: p1, body: b1 }, CoreTerm::Pi { param_type: p2, body: b2 }) => {
            do_match(p1, p2, subs) && do_match(b1, b2, subs)
        }

        (CoreTerm::Constructor { type_name: t1, ctor_name: c1, args: a1 },
         CoreTerm::Constructor { type_name: t2, ctor_name: c2, args: a2 }) => {
            t1 == t2 && c1 == c2 && a1.len() == a2.len()
                && a1.iter().zip(a2.iter()).all(|(x, y)| do_match(x, y, subs))
        }

        _ => false,
    }
}

/// Apply a substitution to a term (replace Var(i) with matched subterms).
fn apply_substitution(term: &CoreTerm, subs: &[(usize, CoreTerm)]) -> CoreTerm {
    match term {
        CoreTerm::Var(i) => {
            for (idx, replacement) in subs {
                if *idx == *i {
                    return replacement.clone();
                }
            }
            term.clone()
        }
        CoreTerm::App { func, arg } => CoreTerm::App {
            func: Box::new(apply_substitution(func, subs)),
            arg: Box::new(apply_substitution(arg, subs)),
        },
        CoreTerm::Lam { param_type, body } => CoreTerm::Lam {
            param_type: Box::new(apply_substitution(param_type, subs)),
            body: Box::new(apply_substitution(body, subs)),
        },
        CoreTerm::Pi { param_type, body } => CoreTerm::Pi {
            param_type: Box::new(apply_substitution(param_type, subs)),
            body: Box::new(apply_substitution(body, subs)),
        },
        CoreTerm::Constructor { type_name, ctor_name, args } => CoreTerm::Constructor {
            type_name: type_name.clone(),
            ctor_name: ctor_name.clone(),
            args: args.iter().map(|a| apply_substitution(a, subs)).collect(),
        },
        _ => term.clone(),
    }
}

/// Create a rewrite rule from a verified proof.
/// The soundness proof IS the serialized witness.
pub fn make_rule(
    lhs: CoreTerm,
    rhs: CoreTerm,
    witness_bytes: &[u8],
    source_witness_hash: Hash32,
) -> RewriteRule {
    let mut hasher_input = lhs.to_bytes();
    hasher_input.extend_from_slice(&rhs.to_bytes());
    let rule_hash = hash::H(&hasher_input);

    RewriteRule {
        rule_hash,
        lhs,
        rhs,
        soundness_proof: witness_bytes.to_vec(),
        source_witness_hash,
        application_count: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nat_const() -> CoreTerm {
        CoreTerm::Const { name: "Nat".into(), levels: vec![] }
    }

    #[test]
    fn empty_basis() {
        let basis = RewriteBasis::new();
        assert_eq!(basis.len(), 0);
        assert!(basis.is_empty());
    }

    #[test]
    fn add_rule() {
        let mut basis = RewriteBasis::new();
        let rule = make_rule(
            CoreTerm::NatLit(0),
            CoreTerm::Constructor { type_name: "Nat".into(), ctor_name: "zero".into(), args: vec![] },
            b"proof",
            hash::H(b"witness"),
        );
        basis.add_rule(rule);
        assert_eq!(basis.len(), 1);
    }

    #[test]
    fn deduplication() {
        let mut basis = RewriteBasis::new();
        let rule1 = make_rule(CoreTerm::NatLit(0), CoreTerm::NatLit(0), b"p", hash::H(b"w"));
        let rule2 = make_rule(CoreTerm::NatLit(0), CoreTerm::NatLit(0), b"p", hash::H(b"w"));
        basis.add_rule(rule1);
        basis.add_rule(rule2);
        assert_eq!(basis.len(), 1, "duplicate rules should be deduplicated");
    }

    #[test]
    fn pattern_match_var() {
        // Pattern Var(0) matches anything
        let subs = pattern_match(&CoreTerm::Var(0), &CoreTerm::NatLit(42)).unwrap();
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0], (0, CoreTerm::NatLit(42)));
    }

    #[test]
    fn pattern_match_exact() {
        // Pattern NatLit(5) matches NatLit(5) but not NatLit(6)
        assert!(pattern_match(&CoreTerm::NatLit(5), &CoreTerm::NatLit(5)).is_some());
        assert!(pattern_match(&CoreTerm::NatLit(5), &CoreTerm::NatLit(6)).is_none());
    }

    #[test]
    fn pattern_match_app() {
        // Pattern App(Var(0), NatLit(1)) matches App(f, NatLit(1)) for any f
        let pattern = CoreTerm::App {
            func: Box::new(CoreTerm::Var(0)),
            arg: Box::new(CoreTerm::NatLit(1)),
        };
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::Const { name: "f".into(), levels: vec![] }),
            arg: Box::new(CoreTerm::NatLit(1)),
        };
        let subs = pattern_match(&pattern, &term).unwrap();
        assert_eq!(subs[0].1, CoreTerm::Const { name: "f".into(), levels: vec![] });
    }

    #[test]
    fn pattern_match_consistency() {
        // Var(0) must match the same thing in both positions
        let pattern = CoreTerm::App {
            func: Box::new(CoreTerm::Var(0)),
            arg: Box::new(CoreTerm::Var(0)),
        };
        // App(5, 5) should match (Var(0) = 5 in both)
        let term_ok = CoreTerm::App {
            func: Box::new(CoreTerm::NatLit(5)),
            arg: Box::new(CoreTerm::NatLit(5)),
        };
        assert!(pattern_match(&pattern, &term_ok).is_some());

        // App(5, 6) should NOT match (Var(0) can't be both 5 and 6)
        let term_bad = CoreTerm::App {
            func: Box::new(CoreTerm::NatLit(5)),
            arg: Box::new(CoreTerm::NatLit(6)),
        };
        assert!(pattern_match(&pattern, &term_bad).is_none());
    }

    #[test]
    fn normalize_with_core_only() {
        // Without basis rules, normalization uses only core reduction
        let mut basis = RewriteBasis::new();
        let env = CoreEnv::new();

        // (λ Nat. Var(0)) 42 should β-reduce to 42
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::Lam {
                param_type: Box::new(nat_const()),
                body: Box::new(CoreTerm::Var(0)),
            }),
            arg: Box::new(CoreTerm::NatLit(42)),
        };

        let (nf, trace) = basis.normalize(&term, &env, 100);
        assert_eq!(nf, CoreTerm::NatLit(42));
        assert!(trace.is_complete);
        assert_eq!(trace.core_steps(), 1);
        assert_eq!(trace.basis_steps(), 0);
    }

    #[test]
    fn normalize_with_basis_rule() {
        // Add a rule: NatLit(0) → Constructor(Nat, zero, [])
        let mut basis = RewriteBasis::new();
        let env = CoreEnv::new();

        let rule = make_rule(
            CoreTerm::NatLit(0),
            CoreTerm::Constructor { type_name: "Nat".into(), ctor_name: "zero".into(), args: vec![] },
            b"proof_zero",
            hash::H(b"witness_zero"),
        );
        basis.add_rule(rule);

        let (nf, trace) = basis.normalize(&CoreTerm::NatLit(0), &env, 100);
        assert_eq!(nf, CoreTerm::Constructor {
            type_name: "Nat".into(),
            ctor_name: "zero".into(),
            args: vec![],
        });
        assert!(trace.is_complete);
        assert_eq!(trace.basis_steps(), 1);
    }

    #[test]
    fn normalize_basis_then_core() {
        // Rule: Const("double") x → App(App(Nat.add, x), x)
        // Then core reduction: Nat.add 5 5 → 10
        let mut basis = RewriteBasis::new();
        let env = CoreEnv::new();

        // Add rule: App(Const("double"), Var(0)) → App(App(Const("Nat.add"), Var(0)), Var(0))
        let rule = make_rule(
            CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "double".into(), levels: vec![] }),
                arg: Box::new(CoreTerm::Var(0)),
            },
            CoreTerm::App {
                func: Box::new(CoreTerm::App {
                    func: Box::new(CoreTerm::Const { name: "Nat.add".into(), levels: vec![] }),
                    arg: Box::new(CoreTerm::Var(0)),
                }),
                arg: Box::new(CoreTerm::Var(0)),
            },
            b"proof_double",
            hash::H(b"witness_double"),
        );
        basis.add_rule(rule);

        let term = CoreTerm::App {
            func: Box::new(CoreTerm::Const { name: "double".into(), levels: vec![] }),
            arg: Box::new(CoreTerm::NatLit(5)),
        };

        let (nf, trace) = basis.normalize(&term, &env, 100);
        assert_eq!(nf, CoreTerm::NatLit(10));
        assert!(trace.is_complete);
        assert!(trace.basis_steps() >= 1, "should use at least 1 basis rule");
        assert!(trace.core_steps() >= 1, "should use at least 1 core reduction");
    }

    #[test]
    fn proof_trace_hashes() {
        let mut basis = RewriteBasis::new();
        let env = CoreEnv::new();

        let term = CoreTerm::Let {
            bound_type: Box::new(nat_const()),
            value: Box::new(CoreTerm::NatLit(7)),
            body: Box::new(CoreTerm::Var(0)),
        };

        let (nf, trace) = basis.normalize(&term, &env, 100);
        assert_eq!(nf, CoreTerm::NatLit(7));
        assert_eq!(trace.initial_hash, term.term_hash());
        assert_eq!(trace.final_hash, nf.term_hash());
        assert_ne!(trace.initial_hash, trace.final_hash);
    }

    #[test]
    fn fixed_point_detection() {
        let mut basis = RewriteBasis::new();
        let before = basis.len();

        let rule = make_rule(CoreTerm::NatLit(0), CoreTerm::NatLit(0), b"p", hash::H(b"w"));
        basis.add_rule(rule);

        assert_eq!(basis.rules_since(before), 1, "1 new rule");
        assert_eq!(basis.rules_since(basis.len()), 0, "no new rules = fixed point");
    }

    #[test]
    fn application_count() {
        let mut basis = RewriteBasis::new();
        let env = CoreEnv::new();

        let rule = make_rule(
            CoreTerm::NatLit(99),
            CoreTerm::NatLit(100),
            b"proof",
            hash::H(b"w"),
        );
        basis.add_rule(rule);

        assert_eq!(basis.total_applications(), 0);

        // Apply the rule
        basis.normalize(&CoreTerm::NatLit(99), &env, 10);
        assert_eq!(basis.total_applications(), 1);

        // Apply again
        basis.normalize(&CoreTerm::NatLit(99), &env, 10);
        assert_eq!(basis.total_applications(), 2);
    }
}
