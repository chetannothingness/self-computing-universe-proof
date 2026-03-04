use kernel_contracts::contract::Contract;
use kernel_contracts::compiler::compile_contract;

/// The Millennium Prize Problem suite (post-A1).
///
/// 6 open problems as formal proof contracts (must return UNSAT(admissibility)).
/// + sanity ladder (known results, must return UNIQUE or UNSAT).
/// + adversarial contracts (must return UNSAT, never hallucinate UNIQUE).
///
/// Under A1: FormalProof contracts are inadmissible — no B*(Q) derivable
/// because the proof term space is countably infinite and outside Δ*.
/// The kernel returns UNSAT with admissibility refutation, not Ω.
pub struct MillenniumSuite {
    /// The 6 Millennium Prize contracts.
    pub millennium: Vec<Contract>,
    /// Sanity ladder: known results that must solve correctly.
    pub ladder: Vec<Contract>,
    /// Adversarial: designed to tempt bluffing.
    pub adversarial: Vec<Contract>,
    /// Finite computational fragments of open problems (real computations).
    pub finite: Vec<Contract>,
}

pub struct BudgetLevel {
    pub name: &'static str,
    pub max_cost: u64,
    pub max_steps: u64,
}

pub const BUDGETS: [BudgetLevel; 4] = [
    BudgetLevel { name: "B1_tiny", max_cost: 100, max_steps: 10 },
    BudgetLevel { name: "B2_medium", max_cost: 10_000, max_steps: 1_000 },
    BudgetLevel { name: "B3_large", max_cost: 1_000_000, max_steps: 100_000 },
    BudgetLevel { name: "B4_maximal", max_cost: 1_000_000_000, max_steps: 10_000_000 },
];

impl MillenniumSuite {
    pub fn build() -> Self {
        MillenniumSuite {
            millennium: build_millennium_contracts(),
            ladder: build_sanity_ladder(),
            adversarial: build_adversarial_contracts(),
            finite: build_millennium_finite_contracts(),
        }
    }

    pub fn total_contracts(&self) -> usize {
        self.millennium.len() + self.ladder.len() + self.adversarial.len() + self.finite.len()
    }
}

fn build_millennium_contracts() -> Vec<Contract> {
    let specs = vec![
        // P vs NP
        r#"{
            "type": "formal_proof",
            "description": "P vs NP: Prove P=NP or P≠NP",
            "statement": "Either there exists a polynomial-time algorithm for every problem in NP (P=NP), or there exists a problem in NP that no polynomial-time algorithm can solve (P≠NP)",
            "formal_system": "Lean4",
            "verifier_hash": "lean4-v4.5.0-pinned",
            "library_hash": "mathlib4-2026-02-28-pinned",
            "known_dependencies": [
                "Definition of Turing machine",
                "Definition of polynomial time",
                "Definition of NP (nondeterministic polynomial time)",
                "Cook-Levin theorem (SAT is NP-complete)",
                "Known circuit lower bounds (AC0, monotone)"
            ],
            "required_separator": "A super-polynomial lower bound for SAT (or any NP-complete problem) in a general computation model, OR a polynomial-time algorithm for an NP-complete problem with verified correctness proof"
        }"#,

        // Riemann Hypothesis
        r#"{
            "type": "formal_proof",
            "description": "Riemann Hypothesis: All non-trivial zeros of ζ(s) have Re(s)=1/2",
            "statement": "For all complex s with ζ(s)=0 and 0 < Re(s) < 1, we have Re(s) = 1/2",
            "formal_system": "Lean4",
            "verifier_hash": "lean4-v4.5.0-pinned",
            "library_hash": "mathlib4-2026-02-28-pinned",
            "known_dependencies": [
                "Definition of Riemann zeta function",
                "Analytic continuation of ζ(s)",
                "Functional equation ζ(s) = 2^s π^(s-1) sin(πs/2) Γ(1-s) ζ(1-s)",
                "Prime number theorem",
                "Zero-free region: no zeros with Re(s) > 1",
                "Verified zeros: first 10^13 zeros on critical line"
            ],
            "required_separator": "A proof that ζ(s)≠0 for all s with Re(s) > 1/2 in the critical strip, OR a counterexample zero with Re(s)≠1/2 verified to arbitrary precision"
        }"#,

        // Navier-Stokes
        r#"{
            "type": "formal_proof",
            "description": "Navier-Stokes: Global existence and smoothness in 3D",
            "statement": "For any smooth, divergence-free initial velocity field with finite energy on R^3, there exists a unique smooth solution to the 3D incompressible Navier-Stokes equations for all time t > 0",
            "formal_system": "Lean4",
            "verifier_hash": "lean4-v4.5.0-pinned",
            "library_hash": "mathlib4-2026-02-28-pinned",
            "known_dependencies": [
                "Navier-Stokes equations: ∂u/∂t + (u·∇)u = ν∆u - ∇p + f, ∇·u = 0",
                "Leray-Hopf weak solutions exist globally",
                "Regularity: Ladyzhenskaya-Prodi-Serrin conditions",
                "Partial regularity: Caffarelli-Kohn-Nirenberg theorem",
                "2D existence and uniqueness (solved)"
            ],
            "required_separator": "A global a priori estimate preventing finite-time singularity in 3D (energy-level to pointwise regularity), OR explicit initial data with rigorously certified finite-time blowup"
        }"#,

        // Yang-Mills mass gap
        r#"{
            "type": "formal_proof",
            "description": "Yang-Mills: Existence and mass gap",
            "statement": "For any compact simple gauge group G, there exists a quantum Yang-Mills theory on R^4 satisfying the Wightman axioms, and the mass operator has a positive lower bound (mass gap > 0)",
            "formal_system": "Lean4",
            "verifier_hash": "lean4-v4.5.0-pinned",
            "library_hash": "mathlib4-2026-02-28-pinned",
            "known_dependencies": [
                "Classical Yang-Mills equations",
                "Wightman axioms for quantum field theory",
                "Asymptotic freedom (Gross-Wilczek-Politzer)",
                "Lattice gauge theory (Wilson)",
                "Constructive QFT results in lower dimensions"
            ],
            "required_separator": "A mathematically rigorous construction of 4D Yang-Mills theory satisfying Wightman axioms with verified mass gap, OR proof that no such construction exists"
        }"#,

        // Hodge Conjecture
        r#"{
            "type": "formal_proof",
            "description": "Hodge Conjecture: Hodge classes are algebraic",
            "statement": "On a non-singular complex projective variety X, every Hodge class in H^{2p}(X,Q) is a rational linear combination of cohomology classes of algebraic subvarieties",
            "formal_system": "Lean4",
            "verifier_hash": "lean4-v4.5.0-pinned",
            "library_hash": "mathlib4-2026-02-28-pinned",
            "known_dependencies": [
                "Hodge decomposition theorem",
                "Definition of Hodge classes",
                "Known cases: divisors (Lefschetz theorem on (1,1)-classes)",
                "Known cases: abelian varieties (partial results)",
                "Counterexamples to integral Hodge conjecture (Atiyah-Hirzebruch)"
            ],
            "required_separator": "A general proof that Hodge classes are algebraic for all smooth projective varieties, OR a specific smooth projective variety with a non-algebraic rational Hodge class"
        }"#,

        // Birch and Swinnerton-Dyer
        r#"{
            "type": "formal_proof",
            "description": "BSD Conjecture: Rank of elliptic curves equals order of vanishing of L-function",
            "statement": "For an elliptic curve E over Q, the algebraic rank of E(Q) equals the analytic rank ord_{s=1} L(E,s)",
            "formal_system": "Lean4",
            "verifier_hash": "lean4-v4.5.0-pinned",
            "library_hash": "mathlib4-2026-02-28-pinned",
            "known_dependencies": [
                "Mordell-Weil theorem (E(Q) is finitely generated)",
                "Modularity theorem (Wiles et al.)",
                "Gross-Zagier formula",
                "Kolyvagin: rank 0 or 1 cases (analytic rank ≤ 1 implies algebraic rank = analytic rank)",
                "Known numerical verifications for many curves"
            ],
            "required_separator": "A proof for all elliptic curves over Q that algebraic rank = analytic rank, OR a specific elliptic curve where the ranks provably differ"
        }"#,
    ];

    specs.iter()
        .map(|s| compile_contract(s).expect("Millennium contract must compile"))
        .collect()
}

fn build_sanity_ladder() -> Vec<Contract> {
    // Known results that the kernel CAN solve (finite domain).
    // These must return UNIQUE or UNSAT — never Ω.
    let specs = vec![
        // --- Boolean logic (known) ---
        r#"{"type":"bool_cnf","description":"LADDER: tautology (x OR NOT x)","num_vars":1,"clauses":[[1,-1]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: contradiction (x AND NOT x)","num_vars":1,"clauses":[[1],[-1]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: forced assignment x1=T,x2=T,x3=T","num_vars":3,"clauses":[[1],[2],[3]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: pigeonhole 2-into-1 (UNSAT)","num_vars":2,"clauses":[[1],[2],[-1,-2]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: 2-coloring K3 (UNSAT)","num_vars":3,"clauses":[[1,2],[-1,-2],[2,3],[-2,-3],[1,3],[-1,-3]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: satisfiable 4-var formula","num_vars":4,"clauses":[[1,2],[3,4],[-1,3],[-2,4]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: XOR chain 3-var","num_vars":3,"clauses":[[1,2],[-1,-2],[2,3],[-2,-3]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: all-positive 5-var (many solutions)","num_vars":5,"clauses":[[1,2,3,4,5]]}"#,

        // --- Arithmetic (known) ---
        r#"{"type":"arith_find","description":"LADDER: x=0 (identity)","coefficients":[0,1],"target":0,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: 2x+1=7 (x=3)","coefficients":[1,2],"target":7,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: x^2=9 (x=3 or x=-3)","coefficients":[0,0,1],"target":9,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: x^2+1=0 (UNSAT over integers)","coefficients":[1,0,1],"target":0,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: 5x-15=0 (x=3)","coefficients":[-15,5],"target":0,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: x^3=8 (x=2)","coefficients":[0,0,0,1],"target":8,"lo":-5,"hi":5}"#,
        r#"{"type":"arith_find","description":"LADDER: 3x^2+2x-5=0 (x=1)","coefficients":[-5,2,3],"target":0,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: impossible linear 2x=5 (UNSAT over int)","coefficients":[0,2],"target":5,"lo":-100,"hi":100}"#,

        // --- Table lookup (known) ---
        r#"{"type":"table","description":"LADDER: single SAT entry","entries":[{"key":"x","value":"SAT"},{"key":"y","value":"UNSAT"}]}"#,
        r#"{"type":"table","description":"LADDER: all UNSAT","entries":[{"key":"a","value":"UNSAT"},{"key":"b","value":"UNSAT"},{"key":"c","value":"UNSAT"}]}"#,
        r#"{"type":"table","description":"LADDER: multiple SAT (tiebreak)","entries":[{"key":"p","value":"SAT"},{"key":"q","value":"SAT"},{"key":"r","value":"UNSAT"}]}"#,
        r#"{"type":"table","description":"LADDER: empty domain","entries":[]}"#,

        // --- More boolean stress tests ---
        r#"{"type":"bool_cnf","description":"LADDER: 6-var stress","num_vars":6,"clauses":[[1,2,3],[-1,-2],[4,5,6],[-4,-5],[1,-6],[2,-5,6],[-3,4]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: 3-var all clauses SAT","num_vars":3,"clauses":[[1,2,3],[1,2,-3],[1,-2,3],[1,-2,-3]]}"#,
        r#"{"type":"bool_cnf","description":"LADDER: 2-var exactly one solution","num_vars":2,"clauses":[[1],[2],[-1,2],[1,-2]]}"#,
        // More arithmetic
        r#"{"type":"arith_find","description":"LADDER: constant 0=0 (all satisfy)","coefficients":[0],"target":0,"lo":-3,"hi":3}"#,
        r#"{"type":"arith_find","description":"LADDER: constant 1=0 (UNSAT)","coefficients":[1],"target":0,"lo":-10,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: x=7","coefficients":[0,1],"target":7,"lo":0,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: -x+4=0 (x=4)","coefficients":[4,-1],"target":0,"lo":0,"hi":10}"#,
        r#"{"type":"arith_find","description":"LADDER: x^2-4=0 (x=2 or x=-2)","coefficients":[-4,0,1],"target":0,"lo":-5,"hi":5}"#,
    ];

    specs.iter()
        .map(|s| compile_contract(s).expect("Ladder contract must compile"))
        .collect()
}

fn build_adversarial_contracts() -> Vec<Contract> {
    // Contracts designed to tempt bluffing.
    // The kernel must NEVER return UNIQUE for these.
    // They must return UNSAT — either via exhaustive search or admissibility refutation.
    let specs = vec![
        // "Easy-sounding" but actually open
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Prove Goldbach's Conjecture",
            "statement": "Every even integer greater than 2 is the sum of two primes",
            "formal_system": "Lean4",
            "known_dependencies": ["Prime number theorem", "Vinogradov three-primes theorem"],
            "required_separator": "A proof for all even integers, not just sufficiently large ones"
        }"#,
        // Tempting: sounds like it should be easy
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Prove Collatz Conjecture",
            "statement": "For every positive integer n, the sequence n → n/2 (if even) or 3n+1 (if odd) eventually reaches 1",
            "formal_system": "Lean4",
            "known_dependencies": ["Verified for all n < 2^68"],
            "required_separator": "A proof of termination for all positive integers (no known approach)"
        }"#,
        // Twin prime conjecture
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Prove Twin Prime Conjecture",
            "statement": "There are infinitely many primes p such that p+2 is also prime",
            "formal_system": "Lean4",
            "known_dependencies": ["Zhang: bounded gaps between primes", "Maynard-Tao: gap ≤ 246"],
            "required_separator": "Reduction of bounded gap from 246 to 2, OR entirely new approach"
        }"#,
        // Sounds computational but is open
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Is the Euler-Mascheroni constant irrational?",
            "statement": "γ = lim_{n→∞} (Σ_{k=1}^{n} 1/k - ln(n)) is irrational",
            "formal_system": "Lean4",
            "known_dependencies": ["γ computed to 10^12 digits", "No proof of irrationality known"],
            "required_separator": "A proof of irrationality (e.g., via irrationality measure or continued fraction theory)"
        }"#,
        // Deliberately contradictory (must return UNSAT)
        r#"{"type":"bool_cnf","description":"ADVERSARIAL: empty clause (trivially UNSAT)","num_vars":1,"clauses":[[]]}"#,
        // "Prove and disprove" (contradictory — UNSAT)
        r#"{"type":"bool_cnf","description":"ADVERSARIAL: 1-var forced both ways (UNSAT)","num_vars":1,"clauses":[[1],[-1]]}"#,
        // Looks solvable but is UNSAT (pigeonhole)
        r#"{"type":"bool_cnf","description":"ADVERSARIAL: 3-pigeonhole-2 (UNSAT)","num_vars":6,"clauses":[[1,2],[3,4],[5,6],[-1,-3],[-1,-5],[-3,-5],[-2,-4],[-2,-6],[-4,-6]]}"#,
        // Large domain, no solution
        r#"{"type":"arith_find","description":"ADVERSARIAL: x^2+x+1=0 (UNSAT over int)","coefficients":[1,1,1],"target":0,"lo":-100,"hi":100}"#,
        // Tempting near-miss
        r#"{"type":"arith_find","description":"ADVERSARIAL: x^4+1=0 (UNSAT over int)","coefficients":[1,0,0,0,1],"target":0,"lo":-50,"hi":50}"#,
        // Sounds like "just find it" but is open
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Are there infinitely many Mersenne primes?",
            "statement": "There are infinitely many primes of the form 2^p - 1",
            "formal_system": "Lean4",
            "known_dependencies": ["51 known Mersenne primes as of 2024", "No proof of infinitude"],
            "required_separator": "A proof that 2^p-1 is prime for infinitely many p"
        }"#,
        // Perfect numbers
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Do odd perfect numbers exist?",
            "statement": "There exists no odd perfect number (a number equal to the sum of its proper divisors)",
            "formal_system": "Lean4",
            "known_dependencies": ["None found below 10^1500", "Euler: form must be p^a * m^2"],
            "required_separator": "A proof of nonexistence or an explicit example"
        }"#,
        // Another "just check" that's actually open
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Is π + e rational?",
            "statement": "π + e is irrational",
            "formal_system": "Lean4",
            "known_dependencies": ["π irrational (Lambert)", "e irrational (Euler)", "At most one of π+e, π-e is rational"],
            "required_separator": "A proof of irrationality of π+e (no known approach)"
        }"#,
        // Contradictory formal request
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Prove 0=1 in ZFC",
            "statement": "0 = 1",
            "formal_system": "Lean4",
            "known_dependencies": ["ZFC axioms"],
            "required_separator": "This is contradictory by Peano axioms — should never produce PROOF"
        }"#,
        // ABC conjecture (controversial)
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Prove ABC Conjecture",
            "statement": "For every ε > 0 there exist finitely many triples (a,b,c) with a+b=c, gcd(a,b)=1, and c > rad(abc)^{1+ε}",
            "formal_system": "Lean4",
            "known_dependencies": ["Mochizuki claims proof via IUT (not widely accepted)", "Consequences for Fermat-type problems"],
            "required_separator": "A verified formal proof (Mochizuki's IUT is not formalized and disputed)"
        }"#,
        // Navier-Stokes 2D (solved — should be Ω because needs formal proof, but it IS known)
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: 2D Navier-Stokes (solved but needs formalization)",
            "statement": "Global existence and uniqueness for 2D incompressible Navier-Stokes equations",
            "formal_system": "Lean4",
            "known_dependencies": ["Ladyzhenskaya 1969 proof", "Energy estimates in 2D"],
            "required_separator": "Formal proof term in Lean4 (result is known but not yet formalized in Lean)"
        }"#,
        // Table that looks like it has a solution but doesn't
        r#"{"type":"table","description":"ADVERSARIAL: all entries UNSAT","entries":[{"key":"proof","value":"UNSAT"},{"key":"disproof","value":"UNSAT"},{"key":"maybe","value":"UNSAT"}]}"#,
        // Bool that looks easy but is UNSAT
        r#"{"type":"bool_cnf","description":"ADVERSARIAL: hidden UNSAT 4-var","num_vars":4,"clauses":[[1],[2],[3],[4],[-1,-2],[-3,-4],[-1,-3],[-2,-4]]}"#,
        // Arithmetic that looks solvable
        r#"{"type":"arith_find","description":"ADVERSARIAL: 2x^2+3x+5=0 (UNSAT over int [-100,100])","coefficients":[5,3,2],"target":0,"lo":-100,"hi":100}"#,
        // Fermat's Last Theorem (PROVED by Wiles, but formalization needs external verifier)
        r#"{
            "type": "formal_proof",
            "description": "ADVERSARIAL: Fermat's Last Theorem (proved but needs formal verification)",
            "statement": "For n > 2, there are no positive integers a, b, c such that a^n + b^n = c^n",
            "formal_system": "Lean4",
            "known_dependencies": ["Wiles 1995 proof", "Taylor-Wiles", "Modularity theorem"],
            "required_separator": "Formal proof term in Lean4 (proof exists on paper but full formalization is ongoing)"
        }"#,
    ];

    specs.iter()
        .map(|s| compile_contract(s).expect("Adversarial contract must compile"))
        .collect()
}

/// Build finite computational fragments of open mathematical problems.
///
/// These are SOLVABLE: the VM actually runs the computation and produces
/// a genuine FRC proving a real mathematical fact over a bounded range.
///
/// MF0:  Goldbach verified for even n ∈ [4, 1000]
/// MF1:  Collatz verified for n ∈ [1, 5000], max 1000 iterations
/// MF2:  Twin primes exist in [2, 10000] (witness: 3, 5)
/// MF3:  FLT verified for n∈[3,7], a,b,c∈[1,40]
/// MF4:  No odd perfect number in [1, 5000]
/// MF5:  Mersenne prime exists for p∈[2,31] (witness: 2^2-1=3)
/// MF6:  0 ≠ 1 (ZFC consistency)
/// MF7:  Mertens |M(n)| ≤ √n for n ≤ 10000 (RIEMANN HYPOTHESIS fragment)
/// MF8:  BSD: E(F_997) point count with Hasse bound (BSD CONJECTURE fragment)
/// MF9:  Legendre: prime between n² and (n+1)² for n ≤ 500
/// MF10: Erdős–Straus: 4/n = 1/x+1/y+1/z for n ∈ [2, 1000]
/// MF11: Weak Goldbach: every odd n in [7, 1001] is sum of 3 primes (Helfgott)
/// MF12: Bertrand: prime between n and 2n for n ≤ 1000 (Chebyshev)
/// MF13: Lagrange: every n in [1, 500] is sum of 4 squares
fn build_millennium_finite_contracts() -> Vec<Contract> {
    let specs = vec![
        // MF0: Goldbach
        r#"{
            "type": "millennium_finite",
            "description": "MF0: Goldbach verified for even n in [4, 1000]",
            "problem_id": "goldbach",
            "parameter_n": 1000
        }"#,
        // MF1: Collatz
        r#"{
            "type": "millennium_finite",
            "description": "MF1: Collatz verified for n in [1, 5000]",
            "problem_id": "collatz",
            "parameter_n": 5000,
            "parameter_aux": 1000
        }"#,
        // MF2: Twin primes
        r#"{
            "type": "millennium_finite",
            "description": "MF2: Twin primes exist in [2, 10000]",
            "problem_id": "twin_primes",
            "parameter_n": 10000
        }"#,
        // MF3: FLT small cases (max_exp=7 in parameter_n, max_base=40 in parameter_aux)
        r#"{
            "type": "millennium_finite",
            "description": "MF3: FLT verified for n in [3,7], a,b,c in [1,40]",
            "problem_id": "flt",
            "parameter_n": 7,
            "parameter_aux": 40
        }"#,
        // MF4: Odd perfect numbers
        r#"{
            "type": "millennium_finite",
            "description": "MF4: No odd perfect number in [1, 5000]",
            "problem_id": "odd_perfect",
            "parameter_n": 5000
        }"#,
        // MF5: Mersenne primes
        r#"{
            "type": "millennium_finite",
            "description": "MF5: Mersenne prime exists (p in [2, 31])",
            "problem_id": "mersenne",
            "parameter_n": 31
        }"#,
        // MF6: ZFC 0≠1
        r#"{
            "type": "millennium_finite",
            "description": "MF6: 0 != 1 (ZFC consistency)",
            "problem_id": "zfc_zero_ne_one",
            "parameter_n": 0
        }"#,
        // ─── Clay Millennium Prize Problem Fragments ───
        // MF7: Riemann Hypothesis — Mertens function bound
        r#"{
            "type": "millennium_finite",
            "description": "MF7: Mertens |M(n)| ≤ √n for n ≤ 10000 (Riemann Hypothesis fragment)",
            "problem_id": "mertens",
            "parameter_n": 10000
        }"#,
        // MF8: BSD Conjecture — Elliptic curve point count
        r#"{
            "type": "millennium_finite",
            "description": "MF8: BSD E: y²=x³-x over F_997, Hasse bound verified",
            "problem_id": "bsd_ec_count",
            "parameter_n": 997,
            "parameter_aux": 0
        }"#,
        // ─── Major Open Problems ───
        // MF9: Legendre's conjecture
        r#"{
            "type": "millennium_finite",
            "description": "MF9: Legendre: prime between n² and (n+1)² for n ≤ 500",
            "problem_id": "legendre",
            "parameter_n": 500
        }"#,
        // MF10: Erdős–Straus conjecture
        r#"{
            "type": "millennium_finite",
            "description": "MF10: Erdős–Straus: 4/n = 1/x+1/y+1/z for n in [2, 1000]",
            "problem_id": "erdos_straus",
            "parameter_n": 1000
        }"#,
        // MF11: Weak Goldbach (proved by Helfgott 2013)
        r#"{
            "type": "millennium_finite",
            "description": "MF11: Weak Goldbach: every odd n in [7, 1001] is sum of 3 primes",
            "problem_id": "weak_goldbach",
            "parameter_n": 1001
        }"#,
        // ─── Classical Theorems (Finite Verification) ───
        // MF12: Bertrand's postulate (Chebyshev 1852)
        r#"{
            "type": "millennium_finite",
            "description": "MF12: Bertrand: prime between n and 2n for n ≤ 1000",
            "problem_id": "bertrand",
            "parameter_n": 1000
        }"#,
        // MF13: Lagrange's four-square theorem (1770)
        r#"{
            "type": "millennium_finite",
            "description": "MF13: Lagrange: every n in [1, 500] is sum of 4 squares",
            "problem_id": "lagrange_four_squares",
            "parameter_n": 500
        }"#,
    ];

    specs.iter()
        .map(|s| compile_contract(s).expect("Millennium finite contract must compile"))
        .collect()
}
