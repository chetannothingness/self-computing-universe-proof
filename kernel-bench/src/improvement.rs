use kernel_types::{Hash32, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_ledger::Ledger;
use crate::dominate::Score;
use serde::{Serialize, Deserialize};

/// A step in the self-improvement log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementStep {
    pub step_id: u64,
    pub change_description: String,
    pub score_before: Score,
    pub score_after: Score,
    pub accepted: bool,
    pub step_hash: Hash32,
}

/// The self-improvement log: monotone record of improvement attempts.
pub struct ImprovementLog {
    pub baseline: Score,
    pub current: Score,
    pub history: Vec<ImprovementStep>,
    pub ledger: Ledger,
}

impl ImprovementLog {
    pub fn new(baseline: Score) -> Self {
        ImprovementLog {
            current: baseline.clone(),
            baseline,
            history: Vec::new(),
            ledger: Ledger::new(),
        }
    }

    /// Number of improvement steps attempted.
    pub fn steps(&self) -> usize {
        self.history.len()
    }

    /// Number of accepted improvements.
    pub fn accepted_count(&self) -> usize {
        self.history.iter().filter(|s| s.accepted).count()
    }
}

/// Result of an improvement attempt.
#[derive(Debug, Clone)]
pub enum ImprovementResult {
    /// Improvement accepted: new score strictly dominates old.
    Accepted {
        step: ImprovementStep,
    },
    /// Improvement rejected: new score does not dominate old.
    Rejected {
        reason: String,
        step: ImprovementStep,
    },
}

/// Try an improvement: accept iff score(new) >_lex score(old)
/// AND false_claims(new) <= false_claims(old).
/// Monotone guarantee by construction.
pub fn try_improvement(
    log: &mut ImprovementLog,
    change_description: String,
    new_score: Score,
) -> ImprovementResult {
    let step_id = log.history.len() as u64;
    let score_before = log.current.clone();

    let monotone_safe = new_score.false_claims <= score_before.false_claims;
    let dominates = new_score.dominates(&score_before);
    let accepted = monotone_safe && dominates;

    let step_hash = hash::H(&canonical_cbor_bytes(&(
        step_id,
        &change_description,
        accepted,
    )));

    let step = ImprovementStep {
        step_id,
        change_description,
        score_before,
        score_after: new_score.clone(),
        accepted,
        step_hash,
    };

    if accepted {
        log.current = new_score;
        log.history.push(step.clone());
        ImprovementResult::Accepted { step }
    } else {
        let reason = if !monotone_safe {
            "Rejected: false claims increased (monotone violation)".into()
        } else {
            "Rejected: new score does not dominate current".into()
        };
        log.history.push(step.clone());
        ImprovementResult::Rejected { reason, step }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn improvement_accepted() {
        let baseline = Score { verified_success: 5, total_tasks: 10, false_claims: 0, total_cost: 100 };
        let mut log = ImprovementLog::new(baseline);
        let new_score = Score { verified_success: 7, total_tasks: 10, false_claims: 0, total_cost: 90 };
        let result = try_improvement(&mut log, "better solver".into(), new_score);
        assert!(matches!(result, ImprovementResult::Accepted { .. }));
        assert_eq!(log.accepted_count(), 1);
    }

    #[test]
    fn improvement_rejected_regression() {
        let baseline = Score { verified_success: 8, total_tasks: 10, false_claims: 0, total_cost: 100 };
        let mut log = ImprovementLog::new(baseline);
        let new_score = Score { verified_success: 6, total_tasks: 10, false_claims: 0, total_cost: 50 };
        let result = try_improvement(&mut log, "worse solver".into(), new_score);
        assert!(matches!(result, ImprovementResult::Rejected { .. }));
        assert_eq!(log.accepted_count(), 0);
    }

    #[test]
    fn improvement_rejected_false_claims() {
        let baseline = Score { verified_success: 8, total_tasks: 10, false_claims: 0, total_cost: 100 };
        let mut log = ImprovementLog::new(baseline);
        let new_score = Score { verified_success: 10, total_tasks: 10, false_claims: 1, total_cost: 50 };
        let result = try_improvement(&mut log, "cheating solver".into(), new_score);
        assert!(matches!(result, ImprovementResult::Rejected { .. }));
    }
}
