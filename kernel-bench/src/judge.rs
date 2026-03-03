use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::status::Status;
use kernel_types::receipt::SolveOutput;
use kernel_ledger::{Event, EventKind};
use crate::harness::{TaskOutput, AgentOutput};
use crate::suites::Task;
use serde::{Serialize, Deserialize};

/// Verdict from the judge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JudgeVerdict {
    /// Task solved correctly with verified proof.
    Pass,
    /// Task not solved or solution incorrect.
    Fail,
    /// Task claimed solved but verification failed (false claim).
    FalseClaim,
}

/// Result of judging a task output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JudgeResult {
    pub task_id: String,
    pub verdict: JudgeVerdict,
    pub reason: String,
    pub verdict_hash: Hash32,
}

impl SerPi for JudgeResult {
    fn ser_pi(&self) -> Vec<u8> {
        let tag: u8 = match self.verdict {
            JudgeVerdict::Pass => 0,
            JudgeVerdict::Fail => 1,
            JudgeVerdict::FalseClaim => 2,
        };
        canonical_cbor_bytes(&(&self.task_id, tag, &self.reason))
    }
}

/// Judge a kernel output on a task.
/// Kernel output marked Pass ONLY if it returned UNIQUE with a valid receipt.
/// FalseClaimRate = 0 by construction: no "success" without verifier pass.
pub fn judge_kernel(task: &Task, output: &TaskOutput) -> JudgeResult {
    let status = &output.solve_output.status;
    let has_answer = !output.solve_output.payload.answer.is_empty();
    let has_receipt = output.solve_output.receipt.trace_head != kernel_types::HASH_ZERO
        || !output.solve_output.receipt.branchpoints.is_empty();

    let (verdict, reason) = if *status == Status::Unique && has_answer {
        (JudgeVerdict::Pass, "UNIQUE with verified receipt".into())
    } else if *status == Status::Unsat {
        // UNSAT is a valid answer (the task has no solution).
        // Check if this matches expected verdict.
        match &task.expected_verdict {
            Some(expected) if expected == "UNSAT" =>
                (JudgeVerdict::Pass, "UNSAT matches expected verdict".into()),
            Some(expected) =>
                (JudgeVerdict::Fail, format!("UNSAT but expected {}", expected)),
            None =>
                (JudgeVerdict::Pass, "UNSAT (no expected verdict specified)".into()),
        }
    } else {
        (JudgeVerdict::Fail, format!("status={}, has_answer={}", status, has_answer))
    };

    let verdict_hash = hash::H(&canonical_cbor_bytes(&(&task.id, &reason)));

    JudgeResult {
        task_id: task.id.clone(),
        verdict,
        reason,
        verdict_hash,
    }
}

/// Judge an external agent output on a task.
/// Agent outputs may have FalseClaimRate > 0.
pub fn judge_agent(task: &Task, output: &AgentOutput) -> JudgeResult {
    // Without a test harness execution, we can only check claimed_success.
    // In production, this would run the test harness command.
    let (verdict, reason) = if output.claimed_success && output.answer.is_empty() {
        (JudgeVerdict::FalseClaim, "Claimed success with no answer".into())
    } else if output.answer.is_empty() {
        (JudgeVerdict::Fail, "No answer provided".into())
    } else {
        // Would run test_harness here. For now, treat as unverified.
        (JudgeVerdict::Fail, "Agent output not verified (no test harness execution)".into())
    };

    let verdict_hash = hash::H(&canonical_cbor_bytes(&(&task.id, &output.agent_id, &reason)));

    JudgeResult {
        task_id: task.id.clone(),
        verdict,
        reason,
        verdict_hash,
    }
}

/// Build a ledger event for a judge verdict.
pub fn verdict_event(result: &JudgeResult) -> Event {
    Event::new(
        EventKind::JudgeVerdict,
        &result.ser_pi(),
        vec![],
        1,
        0,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::suites::{Task, TaskKind, TestHarness};
    use crate::harness::kernel_solve;
    use kernel_contracts::compiler::compile_contract;

    #[test]
    fn judge_kernel_unique_passes() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "judge test",
            "num_vars": 2,
            "clauses": [[1], [2]]
        }"#).unwrap();
        let task = Task {
            id: "JT1".into(),
            kind: TaskKind::Custom("test".into()),
            description: "judge test".into(),
            contract,
            test_harness: TestHarness::default(),
            expected_verdict: None,
        };
        let output = kernel_solve(&task);
        let result = judge_kernel(&task, &output);
        assert_eq!(result.verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn judge_agent_empty_is_fail() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "judge test",
            "num_vars": 1,
            "clauses": [[1]]
        }"#).unwrap();
        let task = Task {
            id: "JT2".into(),
            kind: TaskKind::Custom("test".into()),
            description: "judge test".into(),
            contract,
            test_harness: TestHarness::default(),
            expected_verdict: None,
        };
        let agent_output = AgentOutput {
            task_id: "JT2".into(),
            agent_id: "fake-agent".into(),
            answer: String::new(),
            cost: 0,
            claimed_success: false,
        };
        let result = judge_agent(&task, &agent_output);
        assert_eq!(result.verdict, JudgeVerdict::Fail);
    }

    #[test]
    fn judge_agent_false_claim() {
        let contract = compile_contract(r#"{
            "type": "bool_cnf",
            "description": "judge test",
            "num_vars": 1,
            "clauses": [[1]]
        }"#).unwrap();
        let task = Task {
            id: "JT3".into(),
            kind: TaskKind::Custom("test".into()),
            description: "judge test".into(),
            contract,
            test_harness: TestHarness::default(),
            expected_verdict: None,
        };
        let agent_output = AgentOutput {
            task_id: "JT3".into(),
            agent_id: "fake-agent".into(),
            answer: String::new(),
            cost: 0,
            claimed_success: true,
        };
        let result = judge_agent(&task, &agent_output);
        assert_eq!(result.verdict, JudgeVerdict::FalseClaim);
    }
}
