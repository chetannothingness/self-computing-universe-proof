// Phase 8B: Social Reasoning

use kernel_types::hash;
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

/// Generate a deterministic social task from seed and episode.
pub fn generate_social_task(seed: &[u8; 32], episode: u32) -> SocialTask {
    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    let task_type = ep_seed[0] % 3;
    match task_type {
        0 => SocialTask::FalseBelief {
            hider: format!("Agent{}", ep_seed[1] % 10),
            object: format!("Object{}", ep_seed[2] % 5),
            location_a: format!("Loc{}", ep_seed[3] % 3),
            location_b: format!("Loc{}", 3 + ep_seed[4] % 3),
            observer_present: ep_seed[5] % 2 == 0,
        },
        1 => {
            let num_claims = 2 + (ep_seed[1] as usize % 3);
            let truthful_idx = ep_seed[2] as usize % num_claims;
            let claims = (0..num_claims).map(|i| Claim {
                source: format!("Source{}", i),
                statement: format!("Stmt{}", i),
                is_truthful: i == truthful_idx,
            }).collect();
            SocialTask::ReliabilityJudgment {
                claims,
                ground_truth: format!("Truth{}", ep_seed[3] % 5),
            }
        }
        _ => SocialTask::NormViolation {
            action: format!("Action{}", ep_seed[1] % 10),
            context: format!("Context{}", ep_seed[2] % 5),
            is_violation: ep_seed[3] % 2 == 0,
        },
    }
}

/// Derive the correct answer for a social task.
pub fn solve_social(task: &SocialTask) -> SocialAnswer {
    match task {
        SocialTask::FalseBelief { observer_present, .. } => {
            SocialAnswer {
                answer: if *observer_present { "knows" } else { "does_not_know" }.into(),
            }
        }
        SocialTask::ReliabilityJudgment { claims, .. } => {
            let truthful_source = claims.iter()
                .find(|c| c.is_truthful)
                .map(|c| c.source.clone())
                .unwrap_or_default();
            SocialAnswer { answer: truthful_source }
        }
        SocialTask::NormViolation { is_violation, .. } => {
            SocialAnswer {
                answer: if *is_violation { "violation" } else { "acceptable" }.into(),
            }
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

    #[test]
    fn generate_social_task_deterministic() {
        let seed = [42u8; 32];
        let t1 = serde_json::to_string(&generate_social_task(&seed, 0)).unwrap();
        let t2 = serde_json::to_string(&generate_social_task(&seed, 0)).unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn solve_social_always_correct() {
        for ep in 0..30u32 {
            let seed = [ep as u8; 32];
            let task = generate_social_task(&seed, ep);
            let answer = solve_social(&task);
            assert_eq!(judge_social(&task, &answer), JudgeVerdict::Pass);
        }
    }
}
