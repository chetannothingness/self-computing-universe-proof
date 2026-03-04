// Phase 8B: Social Reasoning
// Full implementation in Week 2.

use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SocialTask {
    FalseBelief {
        hider: String,
        object: String,
        location_a: String,
        location_b: String,
        observer_present: bool,
    },
    ReliabilityJudgment {
        claims: Vec<Claim>,
        ground_truth: String,
    },
    NormViolation {
        action: String,
        context: String,
        is_violation: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claim {
    pub source: String,
    pub statement: String,
    pub is_truthful: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SocialAnswer {
    pub answer: String,
}

pub fn judge_social(task: &SocialTask, agent_answer: &SocialAnswer) -> JudgeVerdict {
    match task {
        SocialTask::FalseBelief { observer_present, .. } => {
            let correct = if *observer_present { "knows" } else { "does_not_know" };
            if agent_answer.answer == correct { JudgeVerdict::Pass } else { JudgeVerdict::Fail }
        }
        SocialTask::ReliabilityJudgment { claims, ground_truth: _ } => {
            // Agent should identify the truthful source
            let truthful_sources: Vec<&str> = claims.iter()
                .filter(|c| c.is_truthful)
                .map(|c| c.source.as_str())
                .collect();
            if truthful_sources.contains(&agent_answer.answer.as_str()) {
                JudgeVerdict::Pass
            } else {
                JudgeVerdict::Fail
            }
        }
        SocialTask::NormViolation { is_violation, .. } => {
            let correct = if *is_violation { "violation" } else { "acceptable" };
            if agent_answer.answer == correct { JudgeVerdict::Pass } else { JudgeVerdict::Fail }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn social_false_belief_absent_observer() {
        let task = SocialTask::FalseBelief {
            hider: "Alice".into(),
            object: "ball".into(),
            location_a: "basket".into(),
            location_b: "box".into(),
            observer_present: false,
        };
        let answer = SocialAnswer { answer: "does_not_know".into() };
        assert_eq!(judge_social(&task, &answer), JudgeVerdict::Pass);
    }

    #[test]
    fn social_false_belief_present_observer() {
        let task = SocialTask::FalseBelief {
            hider: "Alice".into(),
            object: "ball".into(),
            location_a: "basket".into(),
            location_b: "box".into(),
            observer_present: true,
        };
        let answer = SocialAnswer { answer: "knows".into() };
        assert_eq!(judge_social(&task, &answer), JudgeVerdict::Pass);
    }

    #[test]
    fn social_reliability_truthful_preferred() {
        let task = SocialTask::ReliabilityJudgment {
            claims: vec![
                Claim { source: "Alice".into(), statement: "It's blue".into(), is_truthful: true },
                Claim { source: "Bob".into(), statement: "It's red".into(), is_truthful: false },
            ],
            ground_truth: "blue".into(),
        };
        let answer = SocialAnswer { answer: "Alice".into() };
        assert_eq!(judge_social(&task, &answer), JudgeVerdict::Pass);
    }
}
