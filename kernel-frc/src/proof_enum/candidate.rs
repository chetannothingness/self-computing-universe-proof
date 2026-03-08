//! Proof candidate generator — tactic enumeration.
//!
//! A proof candidate is a Lean tactic script. The enumerator generates candidates
//! in canonical order by complexity (number and type of tactics).
//!
//! Enumeration levels (tried in order):
//! 1. Single decision procedures: decide, native_decide, simp, omega, trivial
//! 2. Intro + single tactic
//! 3. Two-tactic sequences
//! 4. Induction strategies
//! 5. Deeper sequences (3+ tactics)

/// A Lean tactic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Tactic {
    /// `decide` — decision procedure for decidable Props
    Decide,
    /// `native_decide` — native code decision procedure
    NativeDecide,
    /// `simp` — simplification
    Simp,
    /// `simp [lemma1, lemma2, ...]`
    SimpWith(Vec<String>),
    /// `omega` — linear arithmetic over Nat/Int
    Omega,
    /// `ring` — ring equalities
    Ring,
    /// `norm_num` — numeric normalization
    NormNum,
    /// `linarith` — linear arithmetic
    Linarith,
    /// `intro x`
    Intro(String),
    /// `apply lemma`
    Apply(String),
    /// `exact term`
    Exact(String),
    /// `constructor` — split conjunction/exists
    Constructor,
    /// `cases x`
    Cases(String),
    /// `induction x with | ... => ...`
    Induction(String),
    /// `have h : type := by <sub_tactics>`
    Have(String, Box<Vec<Tactic>>),
    /// `use witness`
    Use(String),
    /// `trivial`
    Trivial,
    /// `contradiction`
    Contradiction,
    /// `exfalso`
    Exfalso,
    /// `tauto` — propositional tautology
    Tauto,
    /// `aesop` — automated proof search
    Aesop,
    /// Arbitrary tactic string (escape hatch)
    Custom(String),
}

impl Tactic {
    /// Render this tactic as a Lean tactic string.
    pub fn to_lean(&self) -> String {
        match self {
            Tactic::Decide => "decide".into(),
            Tactic::NativeDecide => "native_decide".into(),
            Tactic::Simp => "simp".into(),
            Tactic::SimpWith(lemmas) => {
                if lemmas.is_empty() {
                    "simp".into()
                } else {
                    format!("simp [{}]", lemmas.join(", "))
                }
            }
            Tactic::Omega => "omega".into(),
            Tactic::Ring => "ring".into(),
            Tactic::NormNum => "norm_num".into(),
            Tactic::Linarith => "linarith".into(),
            Tactic::Intro(name) => format!("intro {}", name),
            Tactic::Apply(lemma) => format!("apply {}", lemma),
            Tactic::Exact(term) => format!("exact {}", term),
            Tactic::Constructor => "constructor".into(),
            Tactic::Cases(target) => format!("cases {}", target),
            Tactic::Induction(var) => format!("induction {}", var),
            Tactic::Have(stmt, sub_tactics) => {
                let sub = sub_tactics.iter().map(|t| t.to_lean()).collect::<Vec<_>>().join("\n    ");
                format!("have {} := by\n    {}", stmt, sub)
            }
            Tactic::Use(witness) => format!("use {}", witness),
            Tactic::Trivial => "trivial".into(),
            Tactic::Contradiction => "contradiction".into(),
            Tactic::Exfalso => "exfalso".into(),
            Tactic::Tauto => "tauto".into(),
            Tactic::Aesop => "aesop".into(),
            Tactic::Custom(s) => s.clone(),
        }
    }

    /// Complexity score (lower = simpler, tried first).
    pub fn complexity(&self) -> u32 {
        match self {
            Tactic::Trivial | Tactic::Decide => 1,
            Tactic::NativeDecide | Tactic::Simp | Tactic::Omega
            | Tactic::NormNum | Tactic::Tauto => 2,
            Tactic::Ring | Tactic::Linarith | Tactic::Constructor
            | Tactic::Contradiction | Tactic::Exfalso | Tactic::Aesop => 3,
            Tactic::Intro(_) | Tactic::Apply(_) | Tactic::Exact(_)
            | Tactic::Cases(_) | Tactic::Use(_) => 4,
            Tactic::SimpWith(_) => 5,
            Tactic::Induction(_) => 6,
            Tactic::Have(_, _) => 8,
            Tactic::Custom(_) => 10,
        }
    }
}

/// A proof candidate — a sequence of Lean tactics.
#[derive(Debug, Clone)]
pub struct ProofCandidate {
    /// Ordered sequence of tactics.
    pub tactics: Vec<Tactic>,
    /// Canonical enumeration position.
    pub rank: u64,
}

impl ProofCandidate {
    /// Total complexity score of this candidate.
    pub fn complexity(&self) -> u32 {
        self.tactics.iter().map(|t| t.complexity()).sum()
    }

    /// Render the tactics as a Lean tactic block (newline-separated).
    pub fn to_lean_tactics(&self) -> String {
        self.tactics.iter().map(|t| t.to_lean()).collect::<Vec<_>>().join("\n  ")
    }
}

/// Canonical tactic-script enumerator.
///
/// Generates proof candidates in order of increasing complexity:
/// 1. Single automation tactics (decide, simp, omega, trivial, etc.)
/// 2. Intro + single tactic
/// 3. Two-tactic sequences from vocabulary
/// 4. Induction strategies
/// 5. Deeper sequences
pub struct CandidateEnumerator {
    candidates: Vec<Vec<Tactic>>,
}

impl CandidateEnumerator {
    /// Build the canonical enumeration of tactic scripts.
    pub fn new() -> Self {
        let mut candidates = Vec::new();

        // ── Level 1: Single automation tactics ──────────────────────
        let singles = vec![
            Tactic::Trivial,
            Tactic::Decide,
            Tactic::NativeDecide,
            Tactic::Simp,
            Tactic::Omega,
            Tactic::NormNum,
            Tactic::Tauto,
            Tactic::Ring,
            Tactic::Linarith,
            Tactic::Contradiction,
            Tactic::Exfalso,
            Tactic::Aesop,
            Tactic::Constructor,
        ];

        for t in &singles {
            candidates.push(vec![t.clone()]);
        }

        // ── Level 2: Intro + single tactic ──────────────────────────
        let intro_names = ["n", "h", "x", "p", "a", "b"];
        for name in &intro_names {
            for t in &singles {
                candidates.push(vec![
                    Tactic::Intro(name.to_string()),
                    t.clone(),
                ]);
            }
        }

        // ── Level 3: Two intro + single tactic ─────────────────────
        for n1 in &intro_names {
            for n2 in &intro_names {
                if n1 != n2 {
                    for t in &singles {
                        candidates.push(vec![
                            Tactic::Intro(n1.to_string()),
                            Tactic::Intro(n2.to_string()),
                            t.clone(),
                        ]);
                    }
                }
            }
        }

        // ── Level 4: Simp with common lemmas ────────────────────────
        let simp_lemmas = vec![
            vec!["Nat.Prime".into()],
            vec!["Nat.add_comm".into()],
            vec!["Nat.mul_comm".into()],
        ];
        for lemmas in &simp_lemmas {
            candidates.push(vec![Tactic::SimpWith(lemmas.clone())]);
        }

        // ── Level 5: Induction on common variables ──────────────────
        let induction_vars = ["n", "x", "k"];
        for var in &induction_vars {
            // Induction + simp
            candidates.push(vec![
                Tactic::Induction(var.to_string()),
                Tactic::Custom("all_goals simp".into()),
            ]);
            // Induction + omega
            candidates.push(vec![
                Tactic::Induction(var.to_string()),
                Tactic::Custom("all_goals omega".into()),
            ]);
            // Induction + aesop
            candidates.push(vec![
                Tactic::Induction(var.to_string()),
                Tactic::Custom("all_goals aesop".into()),
            ]);
        }

        // ── Level 6: Intro + constructor patterns ───────────────────
        for name in &intro_names[..3] {
            candidates.push(vec![
                Tactic::Intro(name.to_string()),
                Tactic::Constructor,
                Tactic::Trivial,
            ]);
            candidates.push(vec![
                Tactic::Intro(name.to_string()),
                Tactic::Constructor,
                Tactic::Simp,
            ]);
            candidates.push(vec![
                Tactic::Intro(name.to_string()),
                Tactic::Constructor,
                Tactic::Omega,
            ]);
        }

        // ── Level 7: Exact with common terms ────────────────────────
        let exact_terms = [
            "Nat.zero_ne_one",
            "Nat.zero_ne_one.symm",
            "absurd",
            "rfl",
            "True.intro",
        ];
        for term in &exact_terms {
            candidates.push(vec![Tactic::Exact(term.to_string())]);
        }

        // ── Level 8: Use with small witnesses ───────────────────────
        for i in 0..10u32 {
            candidates.push(vec![Tactic::Use(i.to_string())]);
        }

        // ── Level 9: Intro + use + simp patterns ────────────────────
        for name in &intro_names[..3] {
            for i in 0..5u32 {
                candidates.push(vec![
                    Tactic::Intro(name.to_string()),
                    Tactic::Use(i.to_string()),
                    Tactic::Simp,
                ]);
            }
        }

        Self { candidates }
    }

    /// Total number of candidates in the enumeration.
    pub fn total(&self) -> u64 {
        self.candidates.len() as u64
    }

    /// Iterate over candidates up to the given rank.
    pub fn iter_up_to(&self, max_rank: u64) -> impl Iterator<Item = ProofCandidate> + '_ {
        let limit = (max_rank as usize).min(self.candidates.len());
        self.candidates[..limit].iter().enumerate().map(|(i, tactics)| {
            ProofCandidate {
                tactics: tactics.clone(),
                rank: i as u64,
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tactic_render() {
        assert_eq!(Tactic::Decide.to_lean(), "decide");
        assert_eq!(Tactic::Intro("n".into()).to_lean(), "intro n");
        assert_eq!(Tactic::SimpWith(vec!["Nat.Prime".into()]).to_lean(), "simp [Nat.Prime]");
        assert_eq!(Tactic::Use("42".into()).to_lean(), "use 42");
    }

    #[test]
    fn candidate_render() {
        let c = ProofCandidate {
            tactics: vec![Tactic::Intro("n".into()), Tactic::Omega],
            rank: 0,
        };
        assert_eq!(c.to_lean_tactics(), "intro n\n  omega");
    }

    #[test]
    fn enumerator_produces_candidates() {
        let e = CandidateEnumerator::new();
        assert!(e.total() > 0, "enumerator must produce candidates");
        // First candidate should be trivial
        let first: Vec<_> = e.iter_up_to(1).collect();
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].tactics, vec![Tactic::Trivial]);
    }

    #[test]
    fn enumerator_rank_monotone() {
        let e = CandidateEnumerator::new();
        let mut prev_rank = None;
        for c in e.iter_up_to(e.total()) {
            if let Some(pr) = prev_rank {
                assert!(c.rank > pr, "ranks must be strictly increasing");
            }
            prev_rank = Some(c.rank);
        }
    }

    #[test]
    fn enumerator_has_decide_early() {
        let e = CandidateEnumerator::new();
        let first_10: Vec<_> = e.iter_up_to(10).collect();
        let has_decide = first_10.iter().any(|c| c.tactics == vec![Tactic::Decide]);
        assert!(has_decide, "decide should be in the first 10 candidates");
    }

    #[test]
    fn enumerator_substantial_count() {
        let e = CandidateEnumerator::new();
        // Should have a reasonable number of candidates for exploration
        assert!(e.total() >= 100, "enumerator should have ≥100 candidates, got {}", e.total());
    }
}
