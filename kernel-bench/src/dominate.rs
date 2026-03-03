use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::{Ledger, Event, EventKind};
use serde::{Serialize, Deserialize};

/// Scoring rule: lexicographic VerifiedSuccess@1 > FalseClaimRate > Cost.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoringRule {
    pub primary: ScoringMetric,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ScoringMetric {
    Lexicographic,
}

impl SerPi for ScoringRule {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&("ScoringRule", 0u8))
    }
}

/// The DOMINATE(S, M) meta-contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DominateContract {
    pub suite_id: String,
    pub suite_hash: Hash32,
    pub competitor_id: String,
    pub scoring: ScoringRule,
}

impl SerPi for DominateContract {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.suite_id.ser_pi());
        buf.extend_from_slice(&self.suite_hash.ser_pi());
        buf.extend_from_slice(&self.competitor_id.ser_pi());
        buf.extend_from_slice(&self.scoring.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Score from running a suite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Score {
    pub verified_success: u64,
    pub total_tasks: u64,
    pub false_claims: u64,
    pub total_cost: u64,
}

impl Score {
    /// Lexicographic comparison: VerifiedSuccess@1 > FalseClaimRate > Cost.
    pub fn dominates(&self, other: &Score) -> bool {
        if self.verified_success != other.verified_success {
            return self.verified_success > other.verified_success;
        }
        if self.false_claims != other.false_claims {
            return self.false_claims < other.false_claims;
        }
        self.total_cost < other.total_cost
    }
}

impl SerPi for Score {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(
            self.verified_success,
            self.total_tasks,
            self.false_claims,
            self.total_cost,
        ))
    }
}

/// Per-task result from a dominance evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_id: String,
    pub kernel_passed: bool,
    pub competitor_passed: bool,
    pub kernel_cost: u64,
    pub competitor_cost: u64,
}

/// Result of DOMINATE(S, M).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DominateResult {
    pub suite_id: String,
    pub competitor_id: String,
    pub kernel_score: Score,
    pub competitor_score: Score,
    pub dominant: bool,
    pub per_task: Vec<TaskResult>,
    pub receipt_hash: Hash32,
}

impl DominateResult {
    /// Emit ledger events for the dominance evaluation.
    pub fn emit_events(&self, ledger: &mut Ledger) {
        let start_event = Event::new(
            EventKind::DominateStart,
            &self.suite_id.as_bytes().to_vec(),
            vec![],
            1,
            0,
        );
        ledger.commit(start_event);

        for task in &self.per_task {
            let verdict_payload = canonical_cbor_bytes(&(
                &task.task_id,
                task.kernel_passed,
                task.competitor_passed,
            ));
            let verdict_event = Event::new(
                EventKind::DominateVerdict,
                &verdict_payload,
                vec![],
                1,
                0,
            );
            ledger.commit(verdict_event);
        }

        let complete_payload = canonical_cbor_bytes(&(
            &self.suite_id,
            &self.competitor_id,
            self.dominant,
        ));
        let complete_event = Event::new(
            EventKind::DominateComplete,
            &complete_payload,
            vec![],
            1,
            0,
        );
        ledger.commit(complete_event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;

    #[test]
    fn score_dominates_by_success() {
        let a = Score { verified_success: 10, total_tasks: 10, false_claims: 0, total_cost: 100 };
        let b = Score { verified_success: 8, total_tasks: 10, false_claims: 0, total_cost: 50 };
        assert!(a.dominates(&b));
        assert!(!b.dominates(&a));
    }

    #[test]
    fn score_dominates_by_false_claims() {
        let a = Score { verified_success: 10, total_tasks: 10, false_claims: 0, total_cost: 100 };
        let b = Score { verified_success: 10, total_tasks: 10, false_claims: 2, total_cost: 50 };
        assert!(a.dominates(&b));
    }

    #[test]
    fn score_dominates_by_cost() {
        let a = Score { verified_success: 10, total_tasks: 10, false_claims: 0, total_cost: 50 };
        let b = Score { verified_success: 10, total_tasks: 10, false_claims: 0, total_cost: 100 };
        assert!(a.dominates(&b));
    }

    #[test]
    fn dominate_contract_serpi_deterministic() {
        let c = DominateContract {
            suite_id: "test".into(),
            suite_hash: HASH_ZERO,
            competitor_id: "gpt-4".into(),
            scoring: ScoringRule { primary: ScoringMetric::Lexicographic },
        };
        assert_eq!(c.ser_pi(), c.ser_pi());
    }
}
