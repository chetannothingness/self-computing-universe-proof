// Phase 8C: Multi-Step Planning
// Full implementation in Week 2.

use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningWorld {
    pub initial_state: BTreeMap<String, bool>,
    pub goal_state: BTreeMap<String, bool>,
    pub actions: Vec<PlanAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanAction {
    pub name: String,
    pub preconditions: BTreeMap<String, bool>,
    pub effects: BTreeMap<String, bool>,
}

/// Judge: does action sequence reach goal from initial state?
pub fn judge_plan_execution(
    world: &PlanningWorld,
    action_sequence: &[String],
) -> JudgeVerdict {
    let mut state = world.initial_state.clone();

    for action_name in action_sequence {
        let action = world.actions.iter().find(|a| &a.name == action_name);
        match action {
            None => return JudgeVerdict::Fail,
            Some(a) => {
                // Check preconditions
                for (pred, required) in &a.preconditions {
                    if state.get(pred).unwrap_or(&false) != required {
                        return JudgeVerdict::Fail;
                    }
                }
                // Apply effects
                for (pred, value) in &a.effects {
                    state.insert(pred.clone(), *value);
                }
            }
        }
    }

    // Check goal
    for (pred, required) in &world.goal_state {
        if state.get(pred).unwrap_or(&false) != required {
            return JudgeVerdict::Fail;
        }
    }

    JudgeVerdict::Pass
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_blocks_world() -> PlanningWorld {
        let mut initial = BTreeMap::new();
        initial.insert("on_table_A".into(), true);
        initial.insert("on_table_B".into(), true);
        initial.insert("clear_A".into(), true);
        initial.insert("clear_B".into(), true);
        initial.insert("A_on_B".into(), false);

        let mut goal = BTreeMap::new();
        goal.insert("A_on_B".into(), true);

        let mut pre = BTreeMap::new();
        pre.insert("clear_A".into(), true);
        pre.insert("clear_B".into(), true);
        pre.insert("on_table_A".into(), true);

        let mut eff = BTreeMap::new();
        eff.insert("A_on_B".into(), true);
        eff.insert("on_table_A".into(), false);
        eff.insert("clear_B".into(), false);

        let actions = vec![
            PlanAction {
                name: "stack_A_on_B".into(),
                preconditions: pre,
                effects: eff,
            },
        ];

        PlanningWorld {
            initial_state: initial,
            goal_state: goal,
            actions,
        }
    }

    #[test]
    fn planning_valid_sequence_passes() {
        let world = make_blocks_world();
        let seq = vec!["stack_A_on_B".to_string()];
        assert_eq!(judge_plan_execution(&world, &seq), JudgeVerdict::Pass);
    }

    #[test]
    fn planning_invalid_action_fails() {
        let world = make_blocks_world();
        let seq = vec!["nonexistent_action".to_string()];
        assert_eq!(judge_plan_execution(&world, &seq), JudgeVerdict::Fail);
    }

    #[test]
    fn planning_incomplete_goal_fails() {
        let world = make_blocks_world();
        let seq: Vec<String> = vec![]; // empty plan
        assert_eq!(judge_plan_execution(&world, &seq), JudgeVerdict::Fail);
    }
}
