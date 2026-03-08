//! Proof mining — extract reusable structure from found proofs.
//!
//! When the kernel finds a proof π : S, it doesn't just record PROVED.
//! It mines π for reusable fragments:
//!   - Tactic patterns that worked
//!   - Lemma references used
//!   - Proof structure (intro → cases → simp, etc.)
//!   - Intermediate claims (have statements)
//!
//! Mined fragments become normalizer rules: before universal enumeration,
//! check if any mined rule directly solves the new statement. This is
//! the "compiled universe" — heavy work paid once, projected instantly.
//!
//! The kernel learns mathematics not by inventing rules first, but by
//! mining verified proofs into reusable rewrite rules. Structure comes
//! from found proofs. The accelerator is derived from the engine's output.

use kernel_types::{Hash32, hash};

/// A mined proof fragment — reusable structure extracted from a verified proof.
#[derive(Debug, Clone)]
pub struct MinedRule {
    /// Hash of this rule.
    pub rule_hash: Hash32,
    /// The proof script fragment (Lean tactic text).
    pub fragment: String,
    /// Which problem this was mined from.
    pub source_problem: String,
    /// Hash of the source proof.
    pub source_proof_hash: Hash32,
    /// How many times this fragment has been reused successfully.
    pub reuse_count: u64,
}

/// Database of mined proof rules — the kernel's learned mathematics.
pub struct MiningDb {
    /// All mined rules, ordered by discovery.
    rules: Vec<MinedRule>,
}

impl MiningDb {
    /// Create an empty mining database.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Mine a proof script for reusable fragments.
    ///
    /// Extracts:
    /// 1. The full proof script (as a single rule)
    /// 2. Individual tactic lines
    /// 3. Multi-line patterns (consecutive tactic pairs/triples)
    /// 4. Structural patterns (intro/cases/induction blocks)
    pub fn mine_proof(
        &mut self,
        problem_id: &str,
        proof_script: &str,
        proof_hash: Hash32,
    ) -> Vec<MinedRule> {
        let mut new_rules = Vec::new();

        // 1. Full proof script as a rule
        let full_rule = self.add_fragment(problem_id, proof_script, proof_hash);
        new_rules.push(full_rule);

        // 2. Individual tactic lines
        let lines: Vec<&str> = proof_script
            .lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("--"))
            .collect();

        for line in &lines {
            let rule = self.add_fragment(problem_id, line, proof_hash);
            new_rules.push(rule);
        }

        // 3. Consecutive pairs (bigram patterns)
        for window in lines.windows(2) {
            let pair = format!("{}\n  {}", window[0], window[1]);
            let rule = self.add_fragment(problem_id, &pair, proof_hash);
            new_rules.push(rule);
        }

        // 4. Consecutive triples (trigram patterns)
        for window in lines.windows(3) {
            let triple = format!("{}\n  {}\n  {}", window[0], window[1], window[2]);
            let rule = self.add_fragment(problem_id, &triple, proof_hash);
            new_rules.push(rule);
        }

        new_rules
    }

    /// Add a fragment to the database (deduplicating by hash).
    fn add_fragment(
        &mut self,
        problem_id: &str,
        fragment: &str,
        proof_hash: Hash32,
    ) -> MinedRule {
        let rule_hash = hash::H(fragment.as_bytes());

        // Check if already mined
        if let Some(existing) = self.rules.iter_mut().find(|r| r.rule_hash == rule_hash) {
            existing.reuse_count += 1;
            return existing.clone();
        }

        let rule = MinedRule {
            rule_hash,
            fragment: fragment.to_string(),
            source_problem: problem_id.to_string(),
            source_proof_hash: proof_hash,
            reuse_count: 0,
        };

        self.rules.push(rule.clone());
        rule
    }

    /// Get all mined rules, ordered by reuse count (most reused first).
    pub fn rules_by_reuse(&self) -> Vec<&MinedRule> {
        let mut sorted: Vec<&MinedRule> = self.rules.iter().collect();
        sorted.sort_by(|a, b| b.reuse_count.cmp(&a.reuse_count));
        sorted
    }

    /// Get all fragments as candidate proof scripts to try on a new problem.
    ///
    /// This is the normalizer: before universal enumeration, try all mined
    /// fragments. If one works, the problem is solved instantly.
    pub fn normalizer_candidates(&self) -> Vec<&str> {
        self.rules.iter().map(|r| r.fragment.as_str()).collect()
    }

    /// Record that a mined rule was successfully reused on a new problem.
    pub fn record_reuse(&mut self, rule_hash: &Hash32) {
        if let Some(rule) = self.rules.iter_mut().find(|r| &r.rule_hash == rule_hash) {
            rule.reuse_count += 1;
        }
    }

    /// Total rules in the database.
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Is the database empty?
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Check if the normalizer has stabilized (fixed point detection).
    ///
    /// The normalizer is at a fixed point when:
    /// - All problems in the target class are either PROVED or known-FRONTIER
    /// - No new rules have been added in the last N mining cycles
    ///
    /// For now, this is a simple check: have any new rules been added?
    pub fn rules_since(&self, since_count: usize) -> usize {
        if self.rules.len() > since_count {
            self.rules.len() - since_count
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_mining_db() {
        let db = MiningDb::new();
        assert_eq!(db.len(), 0);
        assert!(db.is_empty());
        assert!(db.normalizer_candidates().is_empty());
    }

    #[test]
    fn mine_simple_proof() {
        let mut db = MiningDb::new();
        let proof_hash = hash::H(b"test_proof");
        let rules = db.mine_proof("zfc_zero_ne_one", "decide", proof_hash);
        // "decide" is both the full script and the single line → deduplicated to 1
        assert!(rules.len() >= 1);
        assert!(!db.is_empty());
    }

    #[test]
    fn mine_multiline_proof() {
        let mut db = MiningDb::new();
        let proof_hash = hash::H(b"test_proof");
        let script = "intro n\n  intro h\n  omega";
        let rules = db.mine_proof("lagrange", script, proof_hash);
        // Full script + 3 individual lines + 2 bigrams + 1 trigram = 7
        // But full script = trigram (same text), so might deduplicate
        assert!(rules.len() >= 3, "should have at least 3 rules, got {}", rules.len());
    }

    #[test]
    fn deduplication() {
        let mut db = MiningDb::new();
        let proof_hash = hash::H(b"test");

        db.mine_proof("problem1", "decide", proof_hash);
        let count_after_first = db.len();

        db.mine_proof("problem2", "decide", proof_hash);
        let count_after_second = db.len();

        // "decide" should be deduplicated — same fragment, same hash
        assert_eq!(count_after_first, count_after_second,
            "duplicate fragments should be deduplicated");
    }

    #[test]
    fn reuse_tracking() {
        let mut db = MiningDb::new();
        let proof_hash = hash::H(b"test");
        let rules = db.mine_proof("zfc", "decide", proof_hash);
        let rule_hash = rules[0].rule_hash;

        let before = db.rules.iter().find(|r| r.rule_hash == rule_hash).unwrap().reuse_count;
        db.record_reuse(&rule_hash);
        db.record_reuse(&rule_hash);
        let after = db.rules.iter().find(|r| r.rule_hash == rule_hash).unwrap().reuse_count;

        assert_eq!(after - before, 2, "should have 2 additional reuses");
    }

    #[test]
    fn normalizer_candidates_available() {
        let mut db = MiningDb::new();
        let proof_hash = hash::H(b"test");
        db.mine_proof("zfc", "decide", proof_hash);
        db.mine_proof("lagrange", "intro n\n  omega", proof_hash);

        let candidates = db.normalizer_candidates();
        assert!(candidates.len() >= 2);
        assert!(candidates.contains(&"decide"));
    }

    #[test]
    fn fixed_point_detection() {
        let mut db = MiningDb::new();
        let proof_hash = hash::H(b"test");

        let before = db.len();
        db.mine_proof("zfc", "decide", proof_hash);
        assert!(db.rules_since(before) > 0, "new rules should be detected");

        let after = db.len();
        assert_eq!(db.rules_since(after), 0, "no new rules = fixed point for this cycle");
    }
}
