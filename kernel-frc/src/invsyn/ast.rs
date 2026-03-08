//! InvSyn AST — mirrors lean/KernelVm/InvSyn.lean exactly.
//!
//! Every variant here corresponds 1:1 to a constructor in the Lean Expr inductive type.
//! The serialization format must be deterministic for hashing.

use serde::{Deserialize, Serialize};

/// InvSyn expression — mirrors lean/KernelVm/InvSyn.lean exactly.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    // Atomic
    Var(usize),
    Const(i64),
    // Arithmetic
    Add(Box<Expr>, Box<Expr>),
    Sub(Box<Expr>, Box<Expr>),
    Mul(Box<Expr>, Box<Expr>),
    Neg(Box<Expr>),
    Mod(Box<Expr>, Box<Expr>),
    Div(Box<Expr>, Box<Expr>),
    Pow(Box<Expr>, u32),
    Abs(Box<Expr>),
    Sqrt(Box<Expr>),
    // Comparison (result: 1 for true, 0 for false)
    Le(Box<Expr>, Box<Expr>),
    Lt(Box<Expr>, Box<Expr>),
    Eq(Box<Expr>, Box<Expr>),
    Ne(Box<Expr>, Box<Expr>),
    // Logic (on 0/1 values)
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
    Implies(Box<Expr>, Box<Expr>),
    // Bounded quantifiers (lo, hi are Expr so bounds can reference variables)
    ForallBounded(Box<Expr>, Box<Expr>, Box<Expr>),
    ExistsBounded(Box<Expr>, Box<Expr>, Box<Expr>),
    // Number theory primitives
    IsPrime(Box<Expr>),
    DivisorSum(Box<Expr>),
    MoebiusFn(Box<Expr>),
    // Computation primitives (efficient native implementations)
    CollatzReaches1(Box<Expr>),
    ErdosStrausHolds(Box<Expr>),   // ∃x,y,z: 4/n = 1/x + 1/y + 1/z
    FourSquares(Box<Expr>),         // ∃a,b,c,d: n = a² + b² + c² + d²
    MertensBelow(Box<Expr>),        // |M(n)| < √n where M(n) = Σμ(k)
    FltHolds(Box<Expr>),            // ∀a,b,c > 0: a^n + b^n ≠ c^n (n≥3)
    // Structural bound primitives (monotone non-decreasing)
    PrimeCount(Box<Expr>),          // π(n): count of primes ≤ n
    GoldbachRepCount(Box<Expr>),    // G(n): number of ways n = p + q with p,q prime
    PrimeGapMax(Box<Expr>),         // max prime gap up to n
    // Analytic (Layer D)
    IntervalBound(Box<Expr>, Box<Expr>),
    CertifiedSum(Box<Expr>, Box<Expr>, Box<Expr>),
}

impl Expr {
    /// AST node count — used for canonical enumeration ordering.
    pub fn size(&self) -> usize {
        match self {
            Expr::Var(_) | Expr::Const(_) => 1,
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mul(l, r)
            | Expr::Mod(l, r)
            | Expr::Le(l, r)
            | Expr::Lt(l, r)
            | Expr::Eq(l, r)
            | Expr::Ne(l, r)
            | Expr::And(l, r)
            | Expr::Or(l, r)
            | Expr::Implies(l, r)
            | Expr::Div(l, r)
            | Expr::IntervalBound(l, r) => 1 + l.size() + r.size(),
            Expr::Neg(e) | Expr::Not(e) | Expr::Abs(e) | Expr::Sqrt(e)
            | Expr::IsPrime(e) | Expr::DivisorSum(e) | Expr::MoebiusFn(e)
            | Expr::CollatzReaches1(e)
            | Expr::ErdosStrausHolds(e) | Expr::FourSquares(e)
            | Expr::MertensBelow(e) | Expr::FltHolds(e)
            | Expr::PrimeCount(e) | Expr::GoldbachRepCount(e)
            | Expr::PrimeGapMax(e) => {
                1 + e.size()
            }
            Expr::Pow(base, _) => 1 + base.size(),
            Expr::ForallBounded(lo, hi, body)
            | Expr::ExistsBounded(lo, hi, body)
            | Expr::CertifiedSum(lo, hi, body) => 1 + lo.size() + hi.size() + body.size(),
        }
    }

    /// Convert to Lean4 InvSyn.Expr representation.
    pub fn to_lean(&self) -> String {
        match self {
            Expr::Var(idx) => format!("Expr.var {}", idx),
            Expr::Const(val) => format!("Expr.const {}", val),
            Expr::Add(l, r) => format!("Expr.add ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Sub(l, r) => format!("Expr.sub ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Mul(l, r) => format!("Expr.mul ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Neg(e) => format!("Expr.neg ({})", e.to_lean()),
            Expr::Mod(l, r) => format!("Expr.modE ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Div(l, r) => format!("Expr.divE ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Pow(base, exp) => format!("Expr.pow ({}) {}", base.to_lean(), exp),
            Expr::Abs(e) => format!("Expr.abs ({})", e.to_lean()),
            Expr::Sqrt(e) => format!("Expr.sqrt ({})", e.to_lean()),
            Expr::Le(l, r) => format!("Expr.le ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Lt(l, r) => format!("Expr.lt ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Eq(l, r) => format!("Expr.eq ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Ne(l, r) => format!("Expr.ne ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::And(l, r) => format!("Expr.andE ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Or(l, r) => format!("Expr.orE ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::Not(e) => format!("Expr.notE ({})", e.to_lean()),
            Expr::Implies(l, r) => format!("Expr.implies ({}) ({})", l.to_lean(), r.to_lean()),
            Expr::ForallBounded(lo, hi, body) => {
                format!("Expr.forallBounded ({}) ({}) ({})", lo.to_lean(), hi.to_lean(), body.to_lean())
            }
            Expr::ExistsBounded(lo, hi, body) => {
                format!("Expr.existsBounded ({}) ({}) ({})", lo.to_lean(), hi.to_lean(), body.to_lean())
            }
            Expr::IsPrime(e) => format!("Expr.isPrime ({})", e.to_lean()),
            Expr::DivisorSum(e) => format!("Expr.divisorSum ({})", e.to_lean()),
            Expr::MoebiusFn(e) => format!("Expr.moebiusFn ({})", e.to_lean()),
            Expr::CollatzReaches1(e) => format!("Expr.collatzReaches1 ({})", e.to_lean()),
            Expr::ErdosStrausHolds(e) => format!("Expr.erdosStrausHolds ({})", e.to_lean()),
            Expr::FourSquares(e) => format!("Expr.fourSquares ({})", e.to_lean()),
            Expr::MertensBelow(e) => format!("Expr.mertensBelow ({})", e.to_lean()),
            Expr::FltHolds(e) => format!("Expr.fltHolds ({})", e.to_lean()),
            Expr::PrimeCount(e) => format!("Expr.primeCount ({})", e.to_lean()),
            Expr::GoldbachRepCount(e) => format!("Expr.goldbachRepCount ({})", e.to_lean()),
            Expr::PrimeGapMax(e) => format!("Expr.primeGapMax ({})", e.to_lean()),
            Expr::IntervalBound(lo, hi) => {
                format!("Expr.intervalBound ({}) ({})", lo.to_lean(), hi.to_lean())
            }
            Expr::CertifiedSum(lo, hi, body) => {
                format!("Expr.certifiedSum ({}) ({}) ({})", lo.to_lean(), hi.to_lean(), body.to_lean())
            }
        }
    }

    /// Layer classification.
    pub fn layer(&self) -> Layer {
        match self {
            Expr::Pow(_, _) => Layer::Polynomial,
            Expr::Mul(l, r) => {
                // mul with at least one const is LIA
                match (l.as_ref(), r.as_ref()) {
                    (Expr::Const(_), _) | (_, Expr::Const(_)) => {
                        Layer::LIA.max(l.layer()).max(r.layer())
                    }
                    _ => Layer::Polynomial, // mul of two variables is polynomial
                }
            }
            Expr::IsPrime(_) | Expr::DivisorSum(_) | Expr::MoebiusFn(_)
            | Expr::CollatzReaches1(_) | Expr::ErdosStrausHolds(_)
            | Expr::FourSquares(_) | Expr::MertensBelow(_)
            | Expr::FltHolds(_)
            | Expr::PrimeCount(_) | Expr::GoldbachRepCount(_)
            | Expr::PrimeGapMax(_) => Layer::Algebraic,
            Expr::IntervalBound(_, _) | Expr::CertifiedSum(_, _, _) => Layer::Analytic,
            Expr::Add(l, r)
            | Expr::Sub(l, r)
            | Expr::Mod(l, r)
            | Expr::Div(l, r)
            | Expr::Le(l, r)
            | Expr::Lt(l, r)
            | Expr::Eq(l, r)
            | Expr::Ne(l, r)
            | Expr::And(l, r)
            | Expr::Or(l, r)
            | Expr::Implies(l, r) => l.layer().max(r.layer()),
            Expr::Neg(e) | Expr::Not(e) | Expr::Abs(e) | Expr::Sqrt(e) => e.layer(),
            Expr::ForallBounded(lo, hi, body) | Expr::ExistsBounded(lo, hi, body) => {
                lo.layer().max(hi.layer()).max(body.layer())
            }
            Expr::Var(_) | Expr::Const(_) => Layer::LIA,
        }
    }
}

/// Layer classification for invariant expressions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum Layer {
    LIA = 0,
    Polynomial = 1,
    Algebraic = 2,
    Analytic = 3,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expr_size() {
        let e = Expr::Add(
            Box::new(Expr::Var(0)),
            Box::new(Expr::Const(1)),
        );
        assert_eq!(e.size(), 3);
    }

    #[test]
    fn expr_to_lean() {
        let e = Expr::Le(
            Box::new(Expr::Var(0)),
            Box::new(Expr::Const(100)),
        );
        assert_eq!(e.to_lean(), "Expr.le (Expr.var 0) (Expr.const 100)");
    }

    #[test]
    fn layer_classification() {
        assert_eq!(Expr::Var(0).layer(), Layer::LIA);
        assert_eq!(
            Expr::Mul(
                Box::new(Expr::Var(0)),
                Box::new(Expr::Var(1))
            ).layer(),
            Layer::Polynomial
        );
        assert_eq!(Expr::IsPrime(Box::new(Expr::Var(0))).layer(), Layer::Algebraic);
    }

    #[test]
    fn expr_serialize_deterministic() {
        let e1 = Expr::Add(Box::new(Expr::Var(0)), Box::new(Expr::Const(1)));
        let e2 = Expr::Add(Box::new(Expr::Var(0)), Box::new(Expr::Const(1)));
        let s1 = serde_json::to_string(&e1).unwrap();
        let s2 = serde_json::to_string(&e2).unwrap();
        assert_eq!(s1, s2);
    }
}
