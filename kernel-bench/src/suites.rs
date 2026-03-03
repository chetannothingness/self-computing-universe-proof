use kernel_types::{Hash32, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_contracts::contract::Contract;
use kernel_contracts::compiler::compile_contract;
use serde::{Serialize, Deserialize};

/// A task kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskKind {
    SWEBench,
    LiveCodeBench,
    HumanEval,
    MBPP,
    Custom(String),
}

impl SerPi for TaskKind {
    fn ser_pi(&self) -> Vec<u8> {
        match self {
            TaskKind::SWEBench => canonical_cbor_bytes(&("SWEBench", 0u8)),
            TaskKind::LiveCodeBench => canonical_cbor_bytes(&("LiveCodeBench", 0u8)),
            TaskKind::HumanEval => canonical_cbor_bytes(&("HumanEval", 0u8)),
            TaskKind::MBPP => canonical_cbor_bytes(&("MBPP", 0u8)),
            TaskKind::Custom(s) => canonical_cbor_bytes(&("Custom", s.as_str())),
        }
    }
}

/// Test harness for verifying task outputs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestHarness {
    pub command: String,
    pub args: Vec<String>,
    pub timeout_ms: u64,
    pub working_dir: String,
}

impl Default for TestHarness {
    fn default() -> Self {
        TestHarness {
            command: "true".into(),
            args: vec![],
            timeout_ms: 30_000,
            working_dir: ".".into(),
        }
    }
}

impl SerPi for TestHarness {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(
            &self.command,
            &self.args,
            self.timeout_ms,
            &self.working_dir,
        ))
    }
}

/// A single benchmark task.
#[derive(Debug, Clone)]
pub struct Task {
    pub id: String,
    pub kind: TaskKind,
    pub description: String,
    pub contract: Contract,
    pub test_harness: TestHarness,
    pub expected_verdict: Option<String>,
}

/// A suite of benchmark tasks.
#[derive(Debug, Clone)]
pub struct TaskSuite {
    pub id: String,
    pub tasks: Vec<Task>,
    pub suite_hash: Hash32,
}

impl TaskSuite {
    pub fn new(id: String, tasks: Vec<Task>) -> Self {
        let mut hash_buf = Vec::new();
        hash_buf.extend_from_slice(id.as_bytes());
        for task in &tasks {
            hash_buf.extend_from_slice(task.id.as_bytes());
            hash_buf.extend_from_slice(&task.contract.ser_pi());
        }
        let suite_hash = hash::H(&hash_buf);
        TaskSuite { id, tasks, suite_hash }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }
}

/// Build a HumanEval-style suite from simple arithmetic contracts.
pub fn build_humaneval_suite() -> TaskSuite {
    let specs = vec![
        (r#"{"type":"arith_find","description":"HE-0: x=5","coefficients":[0,1],"target":5,"lo":0,"hi":10}"#, "UNIQUE"),
        (r#"{"type":"arith_find","description":"HE-1: 2x=10","coefficients":[0,2],"target":10,"lo":0,"hi":10}"#, "UNIQUE"),
        (r#"{"type":"arith_find","description":"HE-2: x+1=4","coefficients":[1,1],"target":4,"lo":0,"hi":10}"#, "UNIQUE"),
    ];

    let tasks: Vec<Task> = specs.iter().enumerate().map(|(i, (spec, verdict))| {
        let contract = compile_contract(spec).unwrap();
        Task {
            id: format!("HE-{}", i),
            kind: TaskKind::HumanEval,
            description: contract.description.clone(),
            contract,
            test_harness: TestHarness::default(),
            expected_verdict: Some(verdict.to_string()),
        }
    }).collect();

    TaskSuite::new("humaneval".into(), tasks)
}

/// Build an MBPP-style suite from boolean SAT contracts.
pub fn build_mbpp_suite() -> TaskSuite {
    let specs = vec![
        (r#"{"type":"bool_cnf","description":"MBPP-0: simple SAT","num_vars":2,"clauses":[[1,2]]}"#, "UNIQUE"),
        (r#"{"type":"bool_cnf","description":"MBPP-1: UNSAT","num_vars":1,"clauses":[[1],[-1]]}"#, "UNSAT"),
        (r#"{"type":"bool_cnf","description":"MBPP-2: forced","num_vars":2,"clauses":[[1],[2]]}"#, "UNIQUE"),
    ];

    let tasks: Vec<Task> = specs.iter().enumerate().map(|(i, (spec, verdict))| {
        let contract = compile_contract(spec).unwrap();
        Task {
            id: format!("MBPP-{}", i),
            kind: TaskKind::MBPP,
            description: contract.description.clone(),
            contract,
            test_harness: TestHarness::default(),
            expected_verdict: Some(verdict.to_string()),
        }
    }).collect();

    TaskSuite::new("mbpp".into(), tasks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humaneval_suite_builds() {
        let suite = build_humaneval_suite();
        assert_eq!(suite.len(), 3);
        assert_eq!(suite.id, "humaneval");
    }

    #[test]
    fn mbpp_suite_builds() {
        let suite = build_mbpp_suite();
        assert_eq!(suite.len(), 3);
        assert_eq!(suite.id, "mbpp");
    }

    #[test]
    fn suite_hash_deterministic() {
        let s1 = build_humaneval_suite();
        let s2 = build_humaneval_suite();
        assert_eq!(s1.suite_hash, s2.suite_hash);
    }

    #[test]
    fn task_kind_serpi_differs() {
        assert_ne!(TaskKind::SWEBench.ser_pi(), TaskKind::HumanEval.ser_pi());
    }
}
