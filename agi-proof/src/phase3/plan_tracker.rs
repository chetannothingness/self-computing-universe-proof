// Phase 3: Plan Tracker
//
// Tracks milestones, dependencies, predictions, and revisions for an agent's
// plan execution. Computes alignment, accuracy, and revision quality scores.
// All values use integer arithmetic (i64/u64), zero floats.

use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// A milestone in the plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// Unique identifier for this milestone.
    pub id: u32,
    /// Human-readable description.
    pub description: String,
    /// The step by which this milestone should be completed (deadline).
    pub deadline_step: u64,
    /// Whether the milestone has been completed.
    pub completed: bool,
    /// The step at which it was completed, if any.
    pub completed_step: Option<u64>,
}

/// A prediction the agent made about a future metric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    /// The step at which the prediction was made.
    pub step: u64,
    /// The metric being predicted (e.g. "revenue_cents", "phenotype_milli").
    pub metric: String,
    /// The predicted value (integer).
    pub predicted_value: i64,
    /// The actual value once observed (filled in later).
    pub actual_value: Option<i64>,
}

/// A revision to the plan, recording when and why the agent changed course.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRevision {
    /// The step at which the revision was made.
    pub step: u64,
    /// Reason for the revision.
    pub reason: String,
    /// List of milestone IDs that were re-scheduled or added.
    pub affected_milestones: Vec<u32>,
}

/// An entry in the action log, recording what the agent did at each step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionLogEntry {
    /// The step at which the action was taken.
    pub step: u64,
    /// The milestone ID this action was intended to advance (if any).
    pub target_milestone: Option<u32>,
    /// Description of the action taken.
    pub action_description: String,
    /// Whether the action succeeded.
    pub succeeded: bool,
}

/// The complete plan object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanObject {
    /// Ordered list of milestones.
    pub milestones: Vec<Milestone>,
    /// Dependencies: (prerequisite_milestone_id, dependent_milestone_id).
    /// A dependent milestone should not be started before its prerequisite is completed.
    pub dependencies: Vec<(u32, u32)>,
    /// Predictions the agent made during execution.
    pub predictions: Vec<Prediction>,
    /// Revisions made to the plan during execution.
    pub revisions: Vec<PlanRevision>,
}

/// Score computed by the plan judge.
#[derive(Debug, Clone)]
pub struct PlanScore {
    /// How well actions aligned with the plan's milestones (0..1000 milli).
    /// 1000 = every action targeted an active, incomplete milestone.
    pub action_alignment_milli: i64,
    /// How accurate the agent's predictions were (0..1000 milli).
    /// 1000 = all predictions exactly matched actuals.
    pub prediction_accuracy_milli: i64,
    /// Whether revisions were well-justified: true iff every revision
    /// was followed by improved outcomes (milestones completed faster or
    /// predictions became more accurate).
    pub revision_quality: bool,
}

// ---------------------------------------------------------------------------
// Plan validation helpers
// ---------------------------------------------------------------------------

/// Check that milestone ordering respects dependencies.
/// Returns true iff for every dependency (prereq, dependent), the prereq's
/// deadline is strictly before the dependent's deadline.
pub fn milestones_respect_dependencies(plan: &PlanObject) -> bool {
    // Build milestone ID -> deadline map
    let deadlines: BTreeMap<u32, u64> = plan
        .milestones
        .iter()
        .map(|m| (m.id, m.deadline_step))
        .collect();

    for (prereq, dependent) in &plan.dependencies {
        match (deadlines.get(prereq), deadlines.get(dependent)) {
            (Some(pre_deadline), Some(dep_deadline)) => {
                if pre_deadline >= dep_deadline {
                    return false;
                }
            }
            _ => {
                // Missing milestone reference => invalid
                return false;
            }
        }
    }

    true
}

/// Check that completed milestones respect dependency ordering:
/// a dependent milestone was not completed before its prerequisite.
pub fn completion_respects_dependencies(plan: &PlanObject) -> bool {
    let completed_at: BTreeMap<u32, Option<u64>> = plan
        .milestones
        .iter()
        .map(|m| (m.id, m.completed_step))
        .collect();

    for (prereq, dependent) in &plan.dependencies {
        let pre_step = completed_at.get(prereq).copied().flatten();
        let dep_step = completed_at.get(dependent).copied().flatten();

        match (pre_step, dep_step) {
            (Some(ps), Some(ds)) => {
                if ds < ps {
                    // Dependent was completed before its prerequisite
                    return false;
                }
            }
            (None, Some(_)) => {
                // Dependent completed but prerequisite never completed
                return false;
            }
            _ => {
                // prerequisite completed but dependent not, or neither completed: OK
            }
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Scoring
// ---------------------------------------------------------------------------

/// Compute the action alignment score.
///
/// For each action in the log that targets a milestone:
///   - If the targeted milestone is incomplete and has no unsatisfied prerequisites,
///     it counts as "aligned".
///   - Otherwise it counts as "misaligned".
///
/// action_alignment_milli = aligned_count * 1000 / total_targeted_actions.
/// If no actions target any milestone, alignment is 0.
fn compute_action_alignment(plan: &PlanObject, action_log: &[ActionLogEntry]) -> i64 {
    // Build dependency lookup: milestone_id -> list of prerequisite IDs
    let mut prereqs: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (pre, dep) in &plan.dependencies {
        prereqs.entry(*dep).or_insert_with(Vec::new).push(*pre);
    }

    // Build milestone completion state indexed by step.
    // We need to know, at each step, which milestones were completed.
    let completed_by: BTreeMap<u32, u64> = plan
        .milestones
        .iter()
        .filter_map(|m| m.completed_step.map(|s| (m.id, s)))
        .collect();

    let targeted_actions: Vec<&ActionLogEntry> = action_log
        .iter()
        .filter(|a| a.target_milestone.is_some())
        .collect();

    if targeted_actions.is_empty() {
        return 0;
    }

    let mut aligned = 0i64;

    for action in &targeted_actions {
        let ms_id = action.target_milestone.unwrap();

        // Check: is the milestone still incomplete at this step?
        let is_incomplete = match completed_by.get(&ms_id) {
            Some(completed_step) => action.step < *completed_step,
            None => true, // never completed => still incomplete
        };

        // Check: are all prerequisites satisfied at this step?
        let prereqs_satisfied = match prereqs.get(&ms_id) {
            Some(pres) => pres.iter().all(|pre_id| {
                match completed_by.get(pre_id) {
                    Some(pre_completed) => *pre_completed <= action.step,
                    None => false,
                }
            }),
            None => true, // no prerequisites
        };

        if is_incomplete && prereqs_satisfied {
            aligned += 1;
        }
    }

    aligned * 1000 / targeted_actions.len() as i64
}

/// Compute prediction accuracy score.
///
/// For each prediction that has an actual value:
///   accuracy_i = 1000 - min(1000, |predicted - actual| * 1000 / max(|actual|, 1))
///
/// prediction_accuracy_milli = sum(accuracy_i) / count.
/// If no predictions have actuals, accuracy is 0.
fn compute_prediction_accuracy(plan: &PlanObject) -> i64 {
    let resolved: Vec<&Prediction> = plan
        .predictions
        .iter()
        .filter(|p| p.actual_value.is_some())
        .collect();

    if resolved.is_empty() {
        return 0;
    }

    let mut total_accuracy: i64 = 0;

    for pred in &resolved {
        let actual = pred.actual_value.unwrap();
        let error = (pred.predicted_value - actual).abs();
        let denominator = actual.abs().max(1);
        let relative_error_milli = (error * 1000 / denominator).min(1000);
        let accuracy = 1000 - relative_error_milli;
        total_accuracy += accuracy;
    }

    total_accuracy / resolved.len() as i64
}

/// Compute revision quality.
///
/// A revision at step S is "justified" if at least one of:
///   (a) A milestone affected by the revision was completed after the revision
///       but before its original deadline.
///   (b) Predictions made after the revision are more accurate than those before.
///
/// revision_quality = true iff ALL revisions are justified (or there are no revisions).
fn compute_revision_quality(plan: &PlanObject) -> bool {
    if plan.revisions.is_empty() {
        return true;
    }

    let completed_at: BTreeMap<u32, u64> = plan
        .milestones
        .iter()
        .filter_map(|m| m.completed_step.map(|s| (m.id, s)))
        .collect();

    let deadlines: BTreeMap<u32, u64> = plan
        .milestones
        .iter()
        .map(|m| (m.id, m.deadline_step))
        .collect();

    for revision in &plan.revisions {
        let rev_step = revision.step;

        // Check condition (a): any affected milestone completed after revision
        // but before its deadline.
        let milestone_improved = revision.affected_milestones.iter().any(|ms_id| {
            match (completed_at.get(ms_id), deadlines.get(ms_id)) {
                (Some(comp), Some(deadline)) => {
                    *comp > rev_step && *comp <= *deadline
                }
                _ => false,
            }
        });

        // Check condition (b): predictions after revision are more accurate
        // than predictions before.
        let preds_before: Vec<&Prediction> = plan
            .predictions
            .iter()
            .filter(|p| p.step < rev_step && p.actual_value.is_some())
            .collect();

        let preds_after: Vec<&Prediction> = plan
            .predictions
            .iter()
            .filter(|p| p.step >= rev_step && p.actual_value.is_some())
            .collect();

        let prediction_improved = if !preds_before.is_empty() && !preds_after.is_empty() {
            let avg_error_before: i64 = preds_before
                .iter()
                .map(|p| {
                    let actual = p.actual_value.unwrap();
                    (p.predicted_value - actual).abs()
                })
                .sum::<i64>()
                / preds_before.len() as i64;

            let avg_error_after: i64 = preds_after
                .iter()
                .map(|p| {
                    let actual = p.actual_value.unwrap();
                    (p.predicted_value - actual).abs()
                })
                .sum::<i64>()
                / preds_after.len() as i64;

            avg_error_after < avg_error_before
        } else {
            false
        };

        if !milestone_improved && !prediction_improved {
            return false;
        }
    }

    true
}

/// Judge a plan against an action log.
///
/// Computes three metrics:
/// - action_alignment_milli: how well actions target appropriate milestones
/// - prediction_accuracy_milli: how accurate the agent's predictions were
/// - revision_quality: whether all plan revisions were justified
pub fn judge_plan(plan: &PlanObject, action_log: &[ActionLogEntry]) -> PlanScore {
    let action_alignment_milli = compute_action_alignment(plan, action_log);
    let prediction_accuracy_milli = compute_prediction_accuracy(plan);
    let revision_quality = compute_revision_quality(plan);

    PlanScore {
        action_alignment_milli,
        prediction_accuracy_milli,
        revision_quality,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_plan() -> PlanObject {
        PlanObject {
            milestones: vec![
                Milestone {
                    id: 1,
                    description: "Setup infrastructure".into(),
                    deadline_step: 10,
                    completed: true,
                    completed_step: Some(5),
                },
                Milestone {
                    id: 2,
                    description: "Implement core logic".into(),
                    deadline_step: 20,
                    completed: true,
                    completed_step: Some(15),
                },
                Milestone {
                    id: 3,
                    description: "Integration testing".into(),
                    deadline_step: 30,
                    completed: true,
                    completed_step: Some(25),
                },
            ],
            dependencies: vec![
                (1, 2), // core logic depends on infrastructure
                (2, 3), // testing depends on core logic
            ],
            predictions: vec![
                Prediction {
                    step: 0,
                    metric: "tasks_done".into(),
                    predicted_value: 50,
                    actual_value: Some(48),
                },
                Prediction {
                    step: 10,
                    metric: "tasks_done".into(),
                    predicted_value: 100,
                    actual_value: Some(95),
                },
            ],
            revisions: vec![],
        }
    }

    #[test]
    fn plan_tracker_milestone_ordering() {
        let plan = make_test_plan();

        // Deadlines: 1->10, 2->20, 3->30
        // Dependencies: (1,2) means 10 < 20 => OK, (2,3) means 20 < 30 => OK
        assert!(milestones_respect_dependencies(&plan));

        // Also check completion ordering
        // Completion: 1->5, 2->15, 3->25 => 5 < 15 < 25 => OK
        assert!(completion_respects_dependencies(&plan));

        // Now break the dependency ordering
        let mut bad_plan = plan.clone();
        bad_plan.milestones[0].deadline_step = 25; // milestone 1 deadline after milestone 2
        assert!(!milestones_respect_dependencies(&bad_plan));

        // Break completion ordering
        let mut bad_completion = plan.clone();
        bad_completion.milestones[1].completed_step = Some(3); // completed before prereq
        assert!(!completion_respects_dependencies(&bad_completion));
    }

    #[test]
    fn plan_prediction_accuracy_computed() {
        let plan = make_test_plan();

        // Prediction 1: predicted 50, actual 48, error=2
        //   relative_error_milli = 2 * 1000 / 48 = 41
        //   accuracy = 1000 - 41 = 959
        //
        // Prediction 2: predicted 100, actual 95, error=5
        //   relative_error_milli = 5 * 1000 / 95 = 52
        //   accuracy = 1000 - 52 = 948
        //
        // Average: (959 + 948) / 2 = 953

        let accuracy = compute_prediction_accuracy(&plan);
        assert_eq!(accuracy, 953);
    }

    #[test]
    fn plan_action_alignment_full() {
        let plan = make_test_plan();

        // All actions target incomplete milestones with satisfied prereqs
        let action_log = vec![
            ActionLogEntry {
                step: 1,
                target_milestone: Some(1),
                action_description: "Setup step 1".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 3,
                target_milestone: Some(1),
                action_description: "Setup step 2".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 7,
                target_milestone: Some(2),
                action_description: "Implement feature A".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 16,
                target_milestone: Some(3),
                action_description: "Run integration test".into(),
                succeeded: true,
            },
        ];

        let score = judge_plan(&plan, &action_log);
        assert_eq!(score.action_alignment_milli, 1000);
    }

    #[test]
    fn plan_action_alignment_partial() {
        let plan = make_test_plan();

        // 2 aligned, 1 misaligned (targeting already-completed milestone)
        let action_log = vec![
            ActionLogEntry {
                step: 1,
                target_milestone: Some(1),
                action_description: "Good action".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 7,
                target_milestone: Some(2),
                action_description: "Good action".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 28,
                target_milestone: Some(1), // milestone 1 completed at step 5, this is step 28
                action_description: "Wasted action on completed milestone".into(),
                succeeded: true,
            },
        ];

        let score = judge_plan(&plan, &action_log);
        // 2 aligned out of 3 => 2 * 1000 / 3 = 666
        assert_eq!(score.action_alignment_milli, 666);
    }

    #[test]
    fn plan_revision_quality_justified() {
        let mut plan = make_test_plan();

        // Add a revision at step 12 that affects milestone 3
        plan.revisions.push(PlanRevision {
            step: 12,
            reason: "Discovered new dependency".into(),
            affected_milestones: vec![3],
        });

        // Milestone 3 was completed at step 25, which is after revision (12)
        // and before deadline (30) => justified
        assert!(compute_revision_quality(&plan));
    }

    #[test]
    fn plan_revision_quality_unjustified() {
        let plan = PlanObject {
            milestones: vec![
                Milestone {
                    id: 1,
                    description: "Task A".into(),
                    deadline_step: 10,
                    completed: false,
                    completed_step: None, // never completed
                },
            ],
            dependencies: vec![],
            predictions: vec![],
            revisions: vec![
                PlanRevision {
                    step: 5,
                    reason: "Changed approach".into(),
                    affected_milestones: vec![1],
                },
            ],
        };

        // Milestone 1 was never completed => revision not justified
        assert!(!compute_revision_quality(&plan));
    }

    #[test]
    fn plan_revision_quality_prediction_improvement() {
        let plan = PlanObject {
            milestones: vec![
                Milestone {
                    id: 1,
                    description: "Task A".into(),
                    deadline_step: 20,
                    completed: false,
                    completed_step: None,
                },
            ],
            dependencies: vec![],
            predictions: vec![
                // Before revision (step < 10): large error
                Prediction {
                    step: 2,
                    metric: "score".into(),
                    predicted_value: 100,
                    actual_value: Some(50), // error = 50
                },
                // After revision (step >= 10): small error
                Prediction {
                    step: 12,
                    metric: "score".into(),
                    predicted_value: 80,
                    actual_value: Some(78), // error = 2
                },
            ],
            revisions: vec![
                PlanRevision {
                    step: 10,
                    reason: "Recalibrated model".into(),
                    affected_milestones: vec![1], // milestone not completed, but predictions improved
                },
            ],
        };

        // Milestone not completed, but predictions improved after revision => justified
        assert!(compute_revision_quality(&plan));
    }

    #[test]
    fn plan_no_predictions_accuracy_zero() {
        let plan = PlanObject {
            milestones: vec![],
            dependencies: vec![],
            predictions: vec![],
            revisions: vec![],
        };

        assert_eq!(compute_prediction_accuracy(&plan), 0);
    }

    #[test]
    fn plan_unresolved_predictions_ignored() {
        let plan = PlanObject {
            milestones: vec![],
            dependencies: vec![],
            predictions: vec![
                Prediction {
                    step: 0,
                    metric: "x".into(),
                    predicted_value: 100,
                    actual_value: None, // unresolved
                },
                Prediction {
                    step: 5,
                    metric: "x".into(),
                    predicted_value: 50,
                    actual_value: Some(50), // perfect prediction
                },
            ],
            revisions: vec![],
        };

        // Only one resolved prediction: accuracy = 1000 (perfect)
        assert_eq!(compute_prediction_accuracy(&plan), 1000);
    }

    #[test]
    fn plan_empty_action_log_alignment_zero() {
        let plan = make_test_plan();
        let score = judge_plan(&plan, &[]);
        assert_eq!(score.action_alignment_milli, 0);
    }

    #[test]
    fn plan_dependency_violation_detected() {
        // Action targets milestone 2, but prerequisite 1 not yet completed
        let plan = PlanObject {
            milestones: vec![
                Milestone {
                    id: 1,
                    description: "First".into(),
                    deadline_step: 10,
                    completed: true,
                    completed_step: Some(8),
                },
                Milestone {
                    id: 2,
                    description: "Second".into(),
                    deadline_step: 20,
                    completed: false,
                    completed_step: None,
                },
            ],
            dependencies: vec![(1, 2)],
            predictions: vec![],
            revisions: vec![],
        };

        let action_log = vec![
            // Action at step 3 targets milestone 2, but milestone 1 completed at step 8
            // => at step 3, prereq not satisfied => misaligned
            ActionLogEntry {
                step: 3,
                target_milestone: Some(2),
                action_description: "Premature work on milestone 2".into(),
                succeeded: true,
            },
            // Action at step 10 targets milestone 2, milestone 1 completed at step 8
            // => prereq satisfied, milestone 2 incomplete => aligned
            ActionLogEntry {
                step: 10,
                target_milestone: Some(2),
                action_description: "Proper work on milestone 2".into(),
                succeeded: true,
            },
        ];

        let score = judge_plan(&plan, &action_log);
        // 1 aligned out of 2 => 500
        assert_eq!(score.action_alignment_milli, 500);
    }

    #[test]
    fn plan_full_judge_integration() {
        let plan = make_test_plan();

        let action_log = vec![
            ActionLogEntry {
                step: 2,
                target_milestone: Some(1),
                action_description: "Work on setup".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 8,
                target_milestone: Some(2),
                action_description: "Implement logic".into(),
                succeeded: true,
            },
            ActionLogEntry {
                step: 18,
                target_milestone: Some(3),
                action_description: "Run tests".into(),
                succeeded: true,
            },
        ];

        let score = judge_plan(&plan, &action_log);

        // All 3 actions are aligned
        assert_eq!(score.action_alignment_milli, 1000);
        // Prediction accuracy: 953 (computed above)
        assert_eq!(score.prediction_accuracy_milli, 953);
        // No revisions => revision quality is true
        assert!(score.revision_quality);
    }
}
