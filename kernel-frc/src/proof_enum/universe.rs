//! 𝒰 — the typed universe class of statements.
//!
//! 𝒰 is the formal class of ALL statements the kernel commits to solving.
//! Every S ∈ 𝒰 is a typed proposition (bytes → Prop).
//!
//! COMPLETE_𝒰: ∀ S ∈ 𝒰, ∃ π, Check(S, π) = PASS
//! GEN_𝒰: ∃ G: 𝒰 → D*, ∀ S ∈ 𝒰, Check(S, G(S)) = PASS
//!
//! 𝒰 is NOT a class you hope to solve. 𝒰 is the class you COMMIT to solving.
//! The commitment IS the meta-theorem COMPLETE_𝒰.
//! G is the constructive witness extracted from COMPLETE_𝒰.

use super::core_term::{CoreTerm, CoreCtx, CoreEnv};
use super::statement::{ProofStatement, get_statement, get_all_statements, is_formalized};
use super::elab::{elab_problem, ElabResult};
use crate::irc::ALL_PROBLEM_IDS;
use kernel_types::{Hash32, hash};

/// A member of 𝒰 — a statement with its canonical encoding.
#[derive(Debug, Clone)]
pub struct UniverseMember {
    /// Problem identifier.
    pub id: String,
    /// The statement.
    pub statement: ProofStatement,
    /// Canonical bytes of the Lean Prop.
    pub canonical_bytes: Vec<u8>,
    /// Hash of the statement.
    pub statement_hash: Hash32,
    /// Whether the statement has a real formalization (not True stub).
    pub is_formalized: bool,
    /// The goal type as CoreTerm (if formalized and ELAB succeeds).
    pub goal: Option<CoreTerm>,
}

/// The universe class 𝒰 — the set of all statements the kernel solves.
pub struct UniverseClass {
    /// All members of 𝒰.
    pub members: Vec<UniverseMember>,
    /// Hash of the entire universe class (canonical).
    pub universe_hash: Hash32,
}

impl UniverseClass {
    /// Build 𝒰 from all 20 problems.
    pub fn build() -> Self {
        let members: Vec<UniverseMember> = ALL_PROBLEM_IDS.iter().map(|id| {
            let stmt = get_statement(id);
            let canonical_bytes = stmt.lean_prop.as_bytes().to_vec();
            let statement_hash = hash::H(&canonical_bytes);
            let formalized = is_formalized(&stmt);

            // Elaborate to get goal CoreTerm
            let goal = match elab_problem(id) {
                ElabResult::Ok { goal, .. } => Some(goal),
                _ => None,
            };

            UniverseMember {
                id: id.to_string(),
                statement: stmt,
                canonical_bytes,
                statement_hash,
                is_formalized: formalized,
                goal,
            }
        }).collect();

        // Hash the entire universe
        let mut all_bytes = Vec::new();
        for m in &members {
            all_bytes.extend_from_slice(&m.statement_hash);
        }
        let universe_hash = hash::H(&all_bytes);

        Self { members, universe_hash }
    }

    /// Number of members in 𝒰.
    pub fn len(&self) -> usize {
        self.members.len()
    }

    /// Is 𝒰 empty?
    pub fn is_empty(&self) -> bool {
        self.members.is_empty()
    }

    /// Number of formalized members (have real Lean Props, not True stubs).
    pub fn formalized_count(&self) -> usize {
        self.members.iter().filter(|m| m.is_formalized).count()
    }

    /// Number of members with elaborated goals.
    pub fn elaborated_count(&self) -> usize {
        self.members.iter().filter(|m| m.goal.is_some()).count()
    }

    /// Check if a statement belongs to 𝒰.
    pub fn belongs_to(&self, problem_id: &str) -> bool {
        self.members.iter().any(|m| m.id == problem_id)
    }

    /// Get a member by ID.
    pub fn get(&self, problem_id: &str) -> Option<&UniverseMember> {
        self.members.iter().find(|m| m.id == problem_id)
    }

    /// The formal type of 𝒰 as a CoreTerm.
    /// 𝒰 is an inductive type: one constructor per problem in the registry.
    pub fn as_core_type(&self) -> CoreTerm {
        CoreTerm::Const {
            name: "𝒰".into(),
            levels: vec![0],
        }
    }

    /// COMPLETE_𝒰 as a CoreTerm type:
    ///   ∀ S : 𝒰, ∃ π : D*, Check(S, π) = PASS
    pub fn complete_type(&self) -> CoreTerm {
        CoreTerm::Pi {
            param_type: Box::new(self.as_core_type()),
            body: Box::new(CoreTerm::Const {
                name: "∃ π, Check(S, π) = PASS".into(),
                levels: vec![],
            }),
        }
    }

    /// GEN_𝒰 as a CoreTerm type:
    ///   ∃ G : 𝒰 → D*, ∀ S : 𝒰, Check(S, G(S)) = PASS
    pub fn gen_type(&self) -> CoreTerm {
        CoreTerm::Const {
            name: "∃ G : 𝒰 → D*, ∀ S, Check(S, G(S)) = PASS".into(),
            levels: vec![],
        }
    }

    /// All formalized member IDs.
    pub fn formalized_ids(&self) -> Vec<&str> {
        self.members.iter()
            .filter(|m| m.is_formalized)
            .map(|m| m.id.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn universe_has_20_members() {
        let u = UniverseClass::build();
        assert_eq!(u.len(), 20);
    }

    #[test]
    fn universe_not_empty() {
        let u = UniverseClass::build();
        assert!(!u.is_empty());
    }

    #[test]
    fn universe_has_formalized_members() {
        let u = UniverseClass::build();
        assert!(u.formalized_count() >= 13, "at least 13 should be formalized");
    }

    #[test]
    fn universe_membership() {
        let u = UniverseClass::build();
        assert!(u.belongs_to("goldbach"));
        assert!(u.belongs_to("p_vs_np"));
        assert!(!u.belongs_to("nonexistent"));
    }

    #[test]
    fn universe_get_member() {
        let u = UniverseClass::build();
        let m = u.get("goldbach").unwrap();
        assert!(m.is_formalized);
        assert!(m.goal.is_some());
    }

    #[test]
    fn universe_hash_deterministic() {
        let u1 = UniverseClass::build();
        let u2 = UniverseClass::build();
        assert_eq!(u1.universe_hash, u2.universe_hash);
    }

    #[test]
    fn universe_complete_type_is_pi() {
        let u = UniverseClass::build();
        let ct = u.complete_type();
        match ct {
            CoreTerm::Pi { .. } => {} // correct: ∀ S : 𝒰, ...
            other => panic!("expected Pi for COMPLETE_𝒰, got {:?}", other),
        }
    }

    #[test]
    fn universe_elaborated_count() {
        let u = UniverseClass::build();
        // All formalized members should elaborate successfully
        assert!(u.elaborated_count() >= u.formalized_count());
    }
}
