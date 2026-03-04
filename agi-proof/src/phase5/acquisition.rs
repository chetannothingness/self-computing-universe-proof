// Phase 5: Self-Directed Knowledge Acquisition
//
// Proves that the system can identify knowledge gaps, choose efficient
// acquisition channels, learn from responses, and never hallucinate.
//
// All arithmetic is integer (i64/u64), zero floats.
// BTreeMap used where ordering matters for determinism.

use kernel_types::{Hash32, hash};
use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Channel types with fixed costs
// ---------------------------------------------------------------------------

/// Acquisition channels available to the system.
/// Each channel has a fixed cost (in abstract units).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AcquisitionChannel {
    /// Run an experiment. Cost = 10.
    Experiment { spec: Vec<u8> },
    /// Retrieve a data slice. Cost = 5.
    DataSlice { query: Vec<u8> },
    /// Read tool documentation. Cost = 2.
    ToolDoc { tool_name: String },
    /// Ask for clarification from an oracle. Cost = 50.
    Clarification { question: String },
}

impl AcquisitionChannel {
    /// Fixed cost for this channel type.
    pub fn cost(&self) -> u64 {
        match self {
            AcquisitionChannel::Experiment { .. } => 10,
            AcquisitionChannel::DataSlice { .. } => 5,
            AcquisitionChannel::ToolDoc { .. } => 2,
            AcquisitionChannel::Clarification { .. } => 50,
        }
    }

    /// Deterministic tag byte for hashing.
    fn tag(&self) -> u8 {
        match self {
            AcquisitionChannel::Experiment { .. } => 0,
            AcquisitionChannel::DataSlice { .. } => 1,
            AcquisitionChannel::ToolDoc { .. } => 2,
            AcquisitionChannel::Clarification { .. } => 3,
        }
    }

    /// Deterministic content bytes for hashing (tag + payload).
    fn content_bytes(&self) -> Vec<u8> {
        let mut buf = vec![self.tag()];
        match self {
            AcquisitionChannel::Experiment { spec } => buf.extend_from_slice(spec),
            AcquisitionChannel::DataSlice { query } => buf.extend_from_slice(query),
            AcquisitionChannel::ToolDoc { tool_name } => buf.extend_from_slice(tool_name.as_bytes()),
            AcquisitionChannel::Clarification { question } => buf.extend_from_slice(question.as_bytes()),
        }
        buf
    }
}

// ---------------------------------------------------------------------------
// Log entry and log
// ---------------------------------------------------------------------------

/// A single acquisition action and its observed result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionEntry {
    pub channel: AcquisitionChannel,
    /// Step number (monotonically increasing).
    pub step: u64,
    /// Hash of the response received from this channel.
    pub response_hash: Hash32,
    /// True if this acquisition yielded no new information
    /// (response was already observed in a prior entry).
    pub was_redundant: bool,
}

/// The full acquisition log for a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcquisitionLog {
    pub entries: Vec<AcquisitionEntry>,
    /// Total cost = sum of channel costs across all entries.
    pub total_cost: u64,
}

impl AcquisitionLog {
    /// Build an AcquisitionLog, computing total_cost and redundancy flags
    /// from a sequence of (channel, step, response_hash) triples.
    ///
    /// An entry is redundant iff its response_hash was already seen in a
    /// prior entry (BTreeMap for determinism).
    pub fn from_raw(
        raw: Vec<(AcquisitionChannel, u64, Hash32)>,
    ) -> Self {
        let mut seen_hashes: BTreeMap<Hash32, u64> = BTreeMap::new();
        let mut entries = Vec::with_capacity(raw.len());
        let mut total_cost: u64 = 0;

        for (channel, step, response_hash) in raw {
            let was_redundant = seen_hashes.contains_key(&response_hash);
            if !was_redundant {
                seen_hashes.insert(response_hash, step);
            }
            total_cost += channel.cost();
            entries.push(AcquisitionEntry {
                channel,
                step,
                response_hash,
                was_redundant,
            });
        }

        AcquisitionLog {
            entries,
            total_cost,
        }
    }

    /// Recompute total_cost from entries (for validation).
    pub fn recompute_cost(&self) -> u64 {
        self.entries.iter().map(|e| e.channel.cost()).sum()
    }
}

// ---------------------------------------------------------------------------
// Score
// ---------------------------------------------------------------------------

/// Computed score for an acquisition log.
#[derive(Debug, Clone)]
pub struct AcquisitionScore {
    /// Gap identification: what fraction of non-redundant entries were
    /// directed at genuinely unknown information.
    /// Represented as num / den (integer fraction).
    pub gap_identification_num: i64,
    pub gap_identification_den: u64,
    /// Efficiency: fraction of cost spent on non-redundant entries.
    /// efficiency = non_redundant_cost / total_cost
    pub efficiency_num: i64,
    pub efficiency_den: u64,
    /// True iff there are at least 2 non-redundant entries AND
    /// the non-redundant entries span at least 2 distinct channels.
    pub learning: bool,
    /// Number of hallucinations detected.
    /// A hallucination is an entry where response_hash == HASH_ZERO,
    /// meaning the system fabricated a response rather than observing one.
    pub hallucination_count: u64,
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Compute acquisition score from a log.
///
/// - gap_identification = non_redundant_count / total_count
/// - efficiency = non_redundant_cost / total_cost
/// - learning = (non_redundant_count >= 2) AND (distinct_channels >= 2)
/// - hallucination_count = entries where response_hash == [0u8; 32]
pub fn compute_acquisition_score(log: &AcquisitionLog) -> AcquisitionScore {
    let total_count = log.entries.len() as u64;

    if total_count == 0 {
        return AcquisitionScore {
            gap_identification_num: 0,
            gap_identification_den: 1,
            efficiency_num: 0,
            efficiency_den: 1,
            learning: false,
            hallucination_count: 0,
        };
    }

    let mut non_redundant_count: u64 = 0;
    let mut non_redundant_cost: u64 = 0;
    let mut hallucination_count: u64 = 0;
    let mut distinct_channels: BTreeMap<u8, bool> = BTreeMap::new();

    let zero_hash: Hash32 = [0u8; 32];

    for entry in &log.entries {
        if !entry.was_redundant {
            non_redundant_count += 1;
            non_redundant_cost += entry.channel.cost();
            distinct_channels.insert(entry.channel.tag(), true);
        }
        if entry.response_hash == zero_hash {
            hallucination_count += 1;
        }
    }

    let gap_identification_num = non_redundant_count as i64;
    let gap_identification_den = total_count;

    let efficiency_num = non_redundant_cost as i64;
    let efficiency_den = if log.total_cost == 0 { 1u64 } else { log.total_cost };

    let learning = non_redundant_count >= 2 && distinct_channels.len() >= 2;

    AcquisitionScore {
        gap_identification_num,
        gap_identification_den,
        efficiency_num,
        efficiency_den,
        learning,
        hallucination_count,
    }
}

// ---------------------------------------------------------------------------
// Judge
// ---------------------------------------------------------------------------

/// Judge an acquisition score:
///   - FalseClaim if hallucinations > 0 (fabricated data)
///   - PASS if efficiency >= 50% AND learning == true
///   - FAIL otherwise
pub fn judge_acquisition(score: &AcquisitionScore) -> JudgeVerdict {
    if score.hallucination_count > 0 {
        return JudgeVerdict::FalseClaim;
    }

    // efficiency >= 50%  <=>  efficiency_num / efficiency_den >= 1/2
    // <=>  efficiency_num * 2 >= efficiency_den (cross-multiply, all positive)
    let eff_sufficient = score.efficiency_num * 2 >= score.efficiency_den as i64;

    if eff_sufficient && score.learning {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

// ---------------------------------------------------------------------------
// Deterministic log generation (for testing / proof runs)
// ---------------------------------------------------------------------------

/// Generate a deterministic acquisition log from a seed and step count.
///
/// Cycles through channels deterministically. Response hashes are derived
/// from H(seed || step_bytes || channel_content_bytes).
pub fn generate_acquisition_log(seed: &[u8; 32], num_steps: u64) -> AcquisitionLog {
    let channel_sequence: Vec<Box<dyn Fn(u64) -> AcquisitionChannel>> = vec![
        Box::new(|step| AcquisitionChannel::ToolDoc {
            tool_name: format!("tool_{}", step),
        }),
        Box::new(|step| AcquisitionChannel::DataSlice {
            query: {
                let mut buf = Vec::new();
                buf.extend_from_slice(b"query_");
                buf.extend_from_slice(&step.to_le_bytes());
                buf
            },
        }),
        Box::new(|step| AcquisitionChannel::Experiment {
            spec: {
                let mut buf = Vec::new();
                buf.extend_from_slice(b"exp_");
                buf.extend_from_slice(&step.to_le_bytes());
                buf
            },
        }),
        Box::new(|_step| AcquisitionChannel::Clarification {
            question: "what_is_the_goal".to_string(),
        }),
    ];

    let mut raw = Vec::with_capacity(num_steps as usize);

    for step in 0..num_steps {
        let channel_idx = (step as usize) % channel_sequence.len();
        let channel = channel_sequence[channel_idx](step);

        let mut hash_buf = Vec::new();
        hash_buf.extend_from_slice(seed);
        hash_buf.extend_from_slice(&step.to_le_bytes());
        hash_buf.extend_from_slice(&channel.content_bytes());
        let response_hash = hash::H(&hash_buf);

        raw.push((channel, step, response_hash));
    }

    AcquisitionLog::from_raw(raw)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquisition_log_deterministic() {
        let seed = [99u8; 32];
        let log_a = generate_acquisition_log(&seed, 12);
        let log_b = generate_acquisition_log(&seed, 12);
        assert_eq!(log_a.entries.len(), log_b.entries.len());
        assert_eq!(log_a.total_cost, log_b.total_cost);
        for (a, b) in log_a.entries.iter().zip(log_b.entries.iter()) {
            assert_eq!(a.step, b.step);
            assert_eq!(a.response_hash, b.response_hash);
            assert_eq!(a.was_redundant, b.was_redundant);
        }
    }

    #[test]
    fn acquisition_cost_computed_correctly() {
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::Experiment { spec: vec![1] }, 0, [1u8; 32]),
            (AcquisitionChannel::DataSlice { query: vec![2] }, 1, [2u8; 32]),
            (AcquisitionChannel::ToolDoc { tool_name: "t".into() }, 2, [3u8; 32]),
            (AcquisitionChannel::Clarification { question: "q".into() }, 3, [4u8; 32]),
        ]);
        // 10 + 5 + 2 + 50 = 67
        assert_eq!(log.total_cost, 67);
        assert_eq!(log.recompute_cost(), 67);
    }

    #[test]
    fn acquisition_redundancy_detected() {
        let same_hash = [42u8; 32];
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::ToolDoc { tool_name: "a".into() }, 0, same_hash),
            (AcquisitionChannel::ToolDoc { tool_name: "b".into() }, 1, same_hash), // redundant
            (AcquisitionChannel::DataSlice { query: vec![1] }, 2, [7u8; 32]),       // new
        ]);
        assert!(!log.entries[0].was_redundant);
        assert!(log.entries[1].was_redundant);   // same response_hash as entry 0
        assert!(!log.entries[2].was_redundant);

        let score = compute_acquisition_score(&log);
        // non_redundant = 2 (entries 0 and 2), total = 3
        assert_eq!(score.gap_identification_num, 2);
        assert_eq!(score.gap_identification_den, 3);
        // non_redundant_cost = 2 (ToolDoc) + 5 (DataSlice) = 7
        // total_cost = 2 + 2 + 5 = 9
        assert_eq!(score.efficiency_num, 7);
        assert_eq!(score.efficiency_den, 9);
    }

    #[test]
    fn acquisition_hallucination_is_false_claim() {
        let zero_hash: Hash32 = [0u8; 32];
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::Experiment { spec: vec![1] }, 0, zero_hash),
            (AcquisitionChannel::DataSlice { query: vec![2] }, 1, [5u8; 32]),
        ]);
        let score = compute_acquisition_score(&log);
        assert_eq!(score.hallucination_count, 1);
        assert_eq!(judge_acquisition(&score), JudgeVerdict::FalseClaim);
    }

    #[test]
    fn acquisition_multiple_hallucinations() {
        let zero_hash: Hash32 = [0u8; 32];
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::Experiment { spec: vec![1] }, 0, zero_hash),
            (AcquisitionChannel::DataSlice { query: vec![2] }, 1, zero_hash),
        ]);
        let score = compute_acquisition_score(&log);
        // Both entries have zero_hash, but entry 1 is also redundant (same hash as entry 0).
        // hallucination_count counts ALL entries with zero_hash regardless of redundancy.
        assert_eq!(score.hallucination_count, 2);
        assert_eq!(judge_acquisition(&score), JudgeVerdict::FalseClaim);
    }

    #[test]
    fn judge_acquisition_pass_on_efficient() {
        // 4 entries, all non-redundant, 2 distinct channels, no hallucinations
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::ToolDoc { tool_name: "a".into() }, 0, [1u8; 32]),
            (AcquisitionChannel::DataSlice { query: vec![1] }, 1, [2u8; 32]),
            (AcquisitionChannel::ToolDoc { tool_name: "b".into() }, 2, [3u8; 32]),
            (AcquisitionChannel::DataSlice { query: vec![2] }, 3, [4u8; 32]),
        ]);
        let score = compute_acquisition_score(&log);
        // all non-redundant => efficiency = total_cost / total_cost = 1/1 = 100%
        assert_eq!(score.efficiency_num as u64, score.efficiency_den);
        assert!(score.learning);
        assert_eq!(score.hallucination_count, 0);
        assert_eq!(judge_acquisition(&score), JudgeVerdict::Pass);
    }

    #[test]
    fn judge_acquisition_fail_low_efficiency() {
        let same_hash = [10u8; 32];
        // All entries have the same response_hash.
        // Only the first is non-redundant. 1 channel type => learning=false.
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::ToolDoc { tool_name: "a".into() }, 0, same_hash),
            (AcquisitionChannel::ToolDoc { tool_name: "b".into() }, 1, same_hash),
            (AcquisitionChannel::ToolDoc { tool_name: "c".into() }, 2, same_hash),
            (AcquisitionChannel::ToolDoc { tool_name: "d".into() }, 3, same_hash),
        ]);
        let score = compute_acquisition_score(&log);
        // non_redundant_cost = 2, total_cost = 8
        // efficiency = 2/8 = 25% < 50%
        assert_eq!(score.efficiency_num, 2);
        assert_eq!(score.efficiency_den, 8);
        assert!(!score.learning); // only 1 distinct channel
        assert_eq!(judge_acquisition(&score), JudgeVerdict::Fail);
    }

    #[test]
    fn judge_acquisition_fail_no_learning() {
        // All non-redundant but only 1 channel type => learning=false
        let log = AcquisitionLog::from_raw(vec![
            (AcquisitionChannel::ToolDoc { tool_name: "a".into() }, 0, [1u8; 32]),
            (AcquisitionChannel::ToolDoc { tool_name: "b".into() }, 1, [2u8; 32]),
        ]);
        let score = compute_acquisition_score(&log);
        // efficiency = 4/4 = 100% (good)
        // but learning = false (only 1 distinct channel tag)
        assert!(!score.learning);
        assert_eq!(judge_acquisition(&score), JudgeVerdict::Fail);
    }

    #[test]
    fn acquisition_empty_log() {
        let log = AcquisitionLog::from_raw(vec![]);
        let score = compute_acquisition_score(&log);
        assert_eq!(score.gap_identification_num, 0);
        assert_eq!(score.gap_identification_den, 1);
        assert_eq!(score.efficiency_num, 0);
        assert_eq!(score.efficiency_den, 1);
        assert!(!score.learning);
        assert_eq!(score.hallucination_count, 0);
        // efficiency_num * 2 = 0 >= 1 = efficiency_den? No => Fail
        assert_eq!(judge_acquisition(&score), JudgeVerdict::Fail);
    }

    #[test]
    fn acquisition_channel_costs() {
        assert_eq!(AcquisitionChannel::Experiment { spec: vec![] }.cost(), 10);
        assert_eq!(AcquisitionChannel::DataSlice { query: vec![] }.cost(), 5);
        assert_eq!(AcquisitionChannel::ToolDoc { tool_name: String::new() }.cost(), 2);
        assert_eq!(AcquisitionChannel::Clarification { question: String::new() }.cost(), 50);
    }

    #[test]
    fn different_seed_different_log() {
        let log_a = generate_acquisition_log(&[1u8; 32], 5);
        let log_b = generate_acquisition_log(&[2u8; 32], 5);
        assert_eq!(log_a.entries.len(), log_b.entries.len());
        // Response hashes should differ
        assert_ne!(
            log_a.entries[0].response_hash,
            log_b.entries[0].response_hash
        );
    }
}
