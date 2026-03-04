// Phase 8C: Multi-Step Planning

use kernel_types::hash;
use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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

/// Generate a deterministic planning world from seed and episode.
///
/// Generates a small STRIPS-style planning problem with 3-6 variables
/// and 2-5 actions. The goal is always reachable from the initial state.
pub fn generate_planning_world(seed: &[u8; 32], episode: u32) -> PlanningWorld {
    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    let num_vars = 3 + (ep_seed[0] as usize % 4);
    let num_actions = 2 + (ep_seed[1] as usize % 4);

    // Initial state: derive from seed
    let mut initial_state = BTreeMap::new();
    for i in 0..num_vars {
        let val = ep_seed[(i + 2) % 32] % 3 == 0;
        initial_state.insert(format!("v{}", i), val);
    }
    // Ensure at least one variable is true (needed for preconditions)
    if !initial_state.values().any(|v| *v) {
        initial_state.insert("v0".into(), true);
    }

    // Generate actions that chain together to form a reachable path
    let mut actions = Vec::with_capacity(num_actions);
    let mut current_state = initial_state.clone();

    for a in 0..num_actions {
        let a_hash_idx = (a * 3 + 10) % 32;

        // Find a precondition variable that is currently true
        let true_vars: Vec<usize> = (0..num_vars)
            .filter(|&i| *current_state.get(&format!("v{}", i)).unwrap_or(&false))
            .collect();

        if true_vars.is_empty() {
            break;
        }

        let pre_var = true_vars[ep_seed[a_hash_idx] as usize % true_vars.len()];
        let eff_var = ep_seed[(a_hash_idx + 1) % 32] as usize % num_vars;

        let mut preconditions = BTreeMap::new();
        preconditions.insert(format!("v{}", pre_var), true);

        let mut effects = BTreeMap::new();
        effects.insert(format!("v{}", eff_var), true);

        actions.push(PlanAction {
            name: format!("act{}", a),
            preconditions,
            effects,
        });

        // Track state forward
        current_state.insert(format!("v{}", eff_var), true);
    }

    // Goal: pick a variable that changed from false to true
    let mut goal_state = BTreeMap::new();
    for i in 0..num_vars {
        let key = format!("v{}", i);
        let init_val = *initial_state.get(&key).unwrap_or(&false);
        let curr_val = *current_state.get(&key).unwrap_or(&false);
        if curr_val && !init_val {
            goal_state.insert(key, true);
            break;
        }
    }

    // If no variable changed, force a reachable goal
    if goal_state.is_empty() {
        let goal_var = format!("v{}", num_vars - 1);
        goal_state.insert(goal_var.clone(), true);

        // Find a true initial variable to use as precondition
        let pre_var = (0..num_vars)
            .find(|&i| *initial_state.get(&format!("v{}", i)).unwrap_or(&false))
            .unwrap_or(0);
        initial_state.insert(format!("v{}", pre_var), true);

        let mut preconditions = BTreeMap::new();
        preconditions.insert(format!("v{}", pre_var), true);
        let mut effects = BTreeMap::new();
        effects.insert(goal_var, true);

        actions.push(PlanAction {
            name: "ensure_goal".into(),
            preconditions,
            effects,
        });
    }

    PlanningWorld {
        initial_state,
        goal_state,
        actions,
    }
}

/// BFS solver for planning: find a shortest action sequence reaching the goal.
pub fn solve_planning(world: &PlanningWorld) -> Option<Vec<String>> {
    let mut visited: BTreeSet<BTreeMap<String, bool>> = BTreeSet::new();
    let mut queue: VecDeque<(BTreeMap<String, bool>, Vec<String>)> = VecDeque::new();

    visited.insert(world.initial_state.clone());
    queue.push_back((world.initial_state.clone(), Vec::new()));

    while let Some((state, path)) = queue.pop_front() {
        if path.len() > 20 {
            continue;
        }

        // Check goal
        let goal_met = world.goal_state.iter().all(|(k, v)| {
            state.get(k).unwrap_or(&false) == v
        });
        if goal_met {
            return Some(path);
        }

        for action in &world.actions {
            let applicable = action.preconditions.iter().all(|(k, v)| {
                state.get(k).unwrap_or(&false) == v
            });
            if applicable {
                let mut new_state = state.clone();
                for (k, v) in &action.effects {
                    new_state.insert(k.clone(), *v);
                }
                if !visited.contains(&new_state) {
                    visited.insert(new_state.clone());
                    let mut new_path = path.clone();
                    new_path.push(action.name.clone());
                    queue.push_back((new_state, new_path));
                }
            }
        }
    }

    None
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

    #[test]
    fn generate_planning_world_deterministic() {
        let seed = [42u8; 32];
        let w1 = generate_planning_world(&seed, 0);
        let w2 = generate_planning_world(&seed, 0);
        assert_eq!(w1.initial_state, w2.initial_state);
        assert_eq!(w1.goal_state, w2.goal_state);
        assert_eq!(w1.actions.len(), w2.actions.len());
    }

    #[test]
    fn generated_planning_world_solvable() {
        for ep in 0..20u32 {
            let seed = [ep as u8; 32];
            let world = generate_planning_world(&seed, ep);
            let solution = solve_planning(&world);
            assert!(solution.is_some(),
                "Episode {} should be solvable", ep);
            let verdict = judge_plan_execution(&world, &solution.unwrap());
            assert_eq!(verdict, JudgeVerdict::Pass,
                "Episode {} solution should pass judge", ep);
        }
    }

    #[test]
    fn solve_planning_finds_shortest_path() {
        let world = make_blocks_world();
        let solution = solve_planning(&world);
        assert!(solution.is_some());
        assert_eq!(solution.unwrap().len(), 1); // one action: stack_A_on_B
    }
}
