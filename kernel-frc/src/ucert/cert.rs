//! Certificate types — mirrors lean/KernelVm/UCert/Cert.lean exactly.
//!
//! Every certificate is a finite, enumerable, checkable data structure.
//! No functions, no oracles — just data.
//!
//! v2: StepCert and LinkCert carry real `Expr` ASTs, not strings.
//! The checker calls structural_step_check / structural_link_check on these.

use serde::{Serialize, Deserialize};
use kernel_types::hash;
use crate::invsyn::ast::Expr;

/// Base case certificate — proves I(init) holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BaseCert {
    /// Evaluate I(n) for n = init..init+bound, all must hold.
    DirectCheck(u64),
    /// I(init) is trivially true.
    Trivial,
}

// ─── Typed certificate structures for advanced step proofs ───

/// Interval arithmetic certificate — proves value stays in [lo, hi].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IntervalCert {
    /// Lower bound expression.
    pub lo: Expr,
    /// Upper bound expression.
    pub hi: Expr,
    /// Subdivision/evaluation steps justifying the enclosure.
    pub proof_steps: Vec<IntervalStep>,
}

/// A single step in an interval arithmetic proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum IntervalStep {
    Eval { point: i64, value_lo: i64, value_hi: i64 },
    Subdivide { mid: i64 },
    MonotoneOn { lo: i64, hi: i64 },
}

/// Sieve-theoretic bound certificate.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SieveCert {
    /// Sieve parameter (sieve level).
    pub sieve_level: u64,
    /// Bound on the remainder sum.
    pub remainder_bound: Expr,
    /// Main term of asymptotic.
    pub main_term: Expr,
}

/// Certified sum bound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SumCert {
    /// The sum expression.
    pub sum_expr: Expr,
    /// Upper/lower bound.
    pub bound: Expr,
    /// Error term bound.
    pub error_bound: Expr,
}

/// A single step in a monotone inequality chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonoStep {
    /// Inequality: from ≤ to.
    pub from: Expr,
    /// Inequality: from ≤ to.
    pub to: Expr,
    /// Justification for this step.
    pub justification: MonoJustification,
}

/// Justification for a monotone step.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MonoJustification {
    Algebraic(AlgebraicCert),
    Monotonicity,
    CauchySchwarz,
    AmGm,
}

/// Algebraic identity certificate (Gröbner basis / SOS decomposition).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlgebraicCert {
    /// The algebraic identity.
    pub identity: Expr,
    /// Witnesses: Gröbner basis elements or SOS decomposition terms.
    pub witnesses: Vec<Expr>,
}

/// Step certificate — proves ∀n, I(n) → I(n+δ).
/// The critical component: this is where mathematical content lives.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StepCert {
    /// Reference to a known theorem (verified against registry).
    KnownProof(String),
    /// Structural verification via InvSyn engine — carries the REAL invariant Expr.
    Structural(Expr),
    /// Evaluate step up to bound (for base cases only, NEVER proves ∀).
    DirectEval(u64),
    /// Interval enclosure proof.
    IntervalBound(IntervalCert),
    /// Sieve-theoretic bound certificate.
    SieveBound(SieveCert),
    /// Finite sum bound certificate.
    SumBound(SumCert),
    /// Monotone inequality chain.
    MonotoneChain(Vec<MonoStep>),
    /// Algebraic identity certificate.
    AlgebraicId(AlgebraicCert),
    /// Compose two step proofs.
    Composition(Box<StepCert>, Box<StepCert>),
}

/// Link certificate — proves ∀n, I(n) → P(n).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LinkCert {
    /// Link is trivially true (invariant contains property).
    Trivial,
    /// Direct logical implication.
    DirectImplication,
    /// Structural verification via InvSyn engine — carries (invariant, property) Exprs.
    Structural(Expr, Expr),
}

/// Invariant certificate — the IRC bridge.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct InvCert {
    /// The actual invariant expression (real Expr AST).
    pub invariant: Expr,
    /// Description of the invariant predicate I(n).
    pub invariant_desc: String,
    /// Invariant hash (deterministic).
    pub invariant_hash: u64,
    /// Certificate that I(init) holds.
    pub base_cert: BaseCert,
    /// Certificate that ∀n, I(n) → I(n+δ).
    pub step_cert: StepCert,
    /// Certificate that ∀n, I(n) → P(n).
    pub link_cert: LinkCert,
}

/// Universal certificate type — the kernel's proof witness.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Cert {
    /// IRC certificate: invariant + base + step + link.
    InvariantCert(InvCert),
    /// Existential witness: concrete n satisfying ∃.
    WitnessCert(u64),
    /// Composite: chain of sub-certificates.
    CompositeCert(Vec<Cert>),
    /// Proof trace: sequence of rewrite steps.
    ProofTrace(Vec<String>),
}

impl Cert {
    /// Certificate node count — used for canonical enumeration ordering.
    pub fn size(&self) -> usize {
        match self {
            Cert::InvariantCert(_) => 1,
            Cert::WitnessCert(_) => 1,
            Cert::CompositeCert(cs) => 1 + cs.iter().map(|c| c.size()).sum::<usize>(),
            Cert::ProofTrace(steps) => 1 + steps.len(),
        }
    }

    /// Deterministic hash.
    pub fn cert_hash(&self) -> [u8; 32] {
        let bytes = serde_json::to_vec(self).unwrap_or_default();
        hash::H(&bytes)
    }

    /// Convert to Lean representation.
    pub fn to_lean(&self) -> String {
        match self {
            Cert::InvariantCert(ic) => {
                format!(
                    "Cert.invariantCert {{ invariantDesc := \"{}\", invariantHash := {}, baseCert := {}, stepCert := {}, linkCert := {} }}",
                    ic.invariant_desc,
                    ic.invariant_hash,
                    ic.base_cert.to_lean(),
                    ic.step_cert.to_lean(),
                    ic.link_cert.to_lean(),
                )
            }
            Cert::WitnessCert(n) => format!("Cert.witnessCert {}", n),
            Cert::CompositeCert(cs) => {
                let items: Vec<String> = cs.iter().map(|c| c.to_lean()).collect();
                format!("Cert.compositeCert [{}]", items.join(", "))
            }
            Cert::ProofTrace(_) => "Cert.proofTrace []".to_string(),
        }
    }
}

impl BaseCert {
    pub fn to_lean(&self) -> String {
        match self {
            BaseCert::DirectCheck(bound) => format!("BaseCert.directCheck {}", bound),
            BaseCert::Trivial => "BaseCert.trivial".to_string(),
        }
    }
}

impl StepCert {
    pub fn size(&self) -> usize {
        match self {
            StepCert::Composition(a, b) => 1 + a.size() + b.size(),
            StepCert::MonotoneChain(steps) => 1 + steps.len(),
            _ => 1,
        }
    }

    pub fn to_lean(&self) -> String {
        match self {
            StepCert::KnownProof(name) => format!("StepCert.knownProof \"{}\"", name),
            StepCert::Structural(inv) => format!("StepCert.structural ({})", inv.to_lean()),
            StepCert::DirectEval(bound) => format!("StepCert.directEval {}", bound),
            StepCert::IntervalBound(_) => "StepCert.intervalBound sorry".to_string(),
            StepCert::SieveBound(_) => "StepCert.sieveBound sorry".to_string(),
            StepCert::SumBound(_) => "StepCert.sumBound sorry".to_string(),
            StepCert::MonotoneChain(_) => "StepCert.monotoneChain []".to_string(),
            StepCert::AlgebraicId(_) => "StepCert.algebraicId sorry".to_string(),
            StepCert::Composition(a, b) => {
                format!("StepCert.composition ({}) ({})", a.to_lean(), b.to_lean())
            }
        }
    }
}

impl LinkCert {
    pub fn to_lean(&self) -> String {
        match self {
            LinkCert::Trivial => "LinkCert.trivial".to_string(),
            LinkCert::DirectImplication => "LinkCert.directImplication".to_string(),
            LinkCert::Structural(inv, prop) => {
                format!("LinkCert.structural ({}) ({})", inv.to_lean(), prop.to_lean())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cert_size() {
        let c = Cert::InvariantCert(InvCert {
            invariant: Expr::Const(1),
            invariant_desc: "I(n) = True".to_string(),
            invariant_hash: 0,
            base_cert: BaseCert::Trivial,
            step_cert: StepCert::KnownProof("test".to_string()),
            link_cert: LinkCert::Trivial,
        });
        assert_eq!(c.size(), 1);
    }

    #[test]
    fn cert_hash_deterministic() {
        let c1 = Cert::InvariantCert(InvCert {
            invariant: Expr::Const(1),
            invariant_desc: "test".to_string(),
            invariant_hash: 42,
            base_cert: BaseCert::Trivial,
            step_cert: StepCert::KnownProof("thm".to_string()),
            link_cert: LinkCert::Trivial,
        });
        let c2 = c1.clone();
        assert_eq!(c1.cert_hash(), c2.cert_hash());
    }

    #[test]
    fn step_cert_size() {
        let s = StepCert::Composition(
            Box::new(StepCert::KnownProof("a".to_string())),
            Box::new(StepCert::Structural(Expr::Const(1))),
        );
        assert_eq!(s.size(), 3);
    }

    #[test]
    fn cert_to_lean() {
        let c = Cert::WitnessCert(42);
        assert_eq!(c.to_lean(), "Cert.witnessCert 42");
    }

    #[test]
    fn structural_step_cert_to_lean() {
        let sc = StepCert::Structural(Expr::Le(
            Box::new(Expr::Const(4)),
            Box::new(Expr::Var(0)),
        ));
        let lean = sc.to_lean();
        assert!(lean.contains("StepCert.structural"));
        assert!(lean.contains("Expr.le"));
    }

    #[test]
    fn structural_link_cert_to_lean() {
        let lc = LinkCert::Structural(
            Expr::Const(1),
            Expr::Const(1),
        );
        let lean = lc.to_lean();
        assert!(lean.contains("LinkCert.structural"));
    }
}
