use kernel_types::Hash32;
use kernel_types::receipt::SolveOutput;
use kernel_solver::Solver;
use crate::suites::Task;
use serde::{Serialize, Deserialize};

/// Output from running the kernel on a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub task_id: String,
    pub solve_output: SolveOutput,
    pub cost: u64,
}

/// Output from running an external agent on a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentOutput {
    pub task_id: String,
    pub agent_id: String,
    pub answer: String,
    pub cost: u64,
    pub claimed_success: bool,
}

/// Result of replaying a kernel output.
#[derive(Debug, Clone)]
pub struct ReplayResult {
    pub task_id: String,
    pub replay_matched: bool,
    pub trace_head_original: Hash32,
    pub trace_head_replay: Hash32,
}

/// Run the kernel solver on a task.
pub fn kernel_solve(task: &Task) -> TaskOutput {
    let mut solver = Solver::new();
    let output = solver.solve(&task.contract);
    TaskOutput {
        task_id: task.id.clone(),
        solve_output: output,
        cost: solver.ledger.total_energy(),
    }
}

/// Replay and verify a kernel output.
pub fn kernel_replay(task: &Task, original: &TaskOutput) -> ReplayResult {
    let mut solver = Solver::new();
    let replay_ok = solver.replay_verify(&task.contract, &original.solve_output);
    let replay_output = solver.solve(&task.contract);
    ReplayResult {
        task_id: task.id.clone(),
        replay_matched: replay_ok,
        trace_head_original: original.solve_output.receipt.trace_head,
        trace_head_replay: replay_output.receipt.trace_head,
    }
}

/// Run an external agent on a task (simulated -- agents are untrusted processes).
/// In production, this would execute a subprocess with timeout.
pub fn agent_run(agent_id: &str, task: &Task) -> AgentOutput {
    // External agents are opaque. We cannot predict their output.
    // For now, simulate with empty output (untrusted).
    AgentOutput {
        task_id: task.id.clone(),
        agent_id: agent_id.into(),
        answer: String::new(),
        cost: 0,
        claimed_success: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suites::{Task, TaskKind, TestHarness};
    use kernel_contracts::compiler::compile_contract;

    fn make_test_task() -> Task {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "harness test",
            "num_vars": 2,
            "clauses": [[1, 2]]
        }"#).unwrap();
        Task {
            id: "T0".into(),
            kind: TaskKind::Custom("test".into()),
            description: "test task".into(),
            contract,
            test_harness: TestHarness::default(),
            expected_verdict: None,
        }
    }

    #[test]
    fn kernel_solve_returns_output() {
        let task = make_test_task();
        let output = kernel_solve(&task);
        assert_eq!(output.task_id, "T0");
    }

    #[test]
    fn kernel_replay_matches() {
        let task = make_test_task();
        let output = kernel_solve(&task);
        let replay = kernel_replay(&task, &output);
        assert!(replay.replay_matched);
    }
}
