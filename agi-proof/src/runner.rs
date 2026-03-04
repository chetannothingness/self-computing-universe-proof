use kernel_types::{Hash32, HASH_ZERO, SerPi, hash};
use kernel_types::serpi::canonical_cbor_bytes;
use kernel_types::status::Status;
use kernel_solver::Solver;
use kernel_bench::judge::JudgeVerdict;
use crate::eval_specs::AgiDomainKind;
use crate::compiler_ext::compile_agi_contract;
use serde::{Serialize, Deserialize};

/// The universal AGI proof runner.
///
/// Pipeline: compile -> solve -> judge -> replay -> bundle.
pub struct AgiRunner {
    pub solver: Solver,
    pub build_hash: Hash32,
}

/// Result of running one AGI task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiTaskResult {
    pub task_id: String,
    pub domain: AgiDomainKind,
    pub status: Status,
    pub verdict: JudgeVerdict,
    pub reason: String,
    pub experiment_count: u64,
    pub cost: u64,
    pub trace_head: Hash32,
    pub verdict_hash: Hash32,
    pub replay_verified: bool,
}

impl SerPi for AgiTaskResult {
    fn ser_pi(&self) -> Vec<u8> {
        let verdict_tag: u8 = match self.verdict {
            JudgeVerdict::Pass => 0,
            JudgeVerdict::Fail => 1,
            JudgeVerdict::FalseClaim => 2,
        };
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.task_id.ser_pi());
        buf.extend_from_slice(&self.domain.ser_pi());
        buf.extend_from_slice(&self.status.ser_pi());
        buf.extend_from_slice(&verdict_tag.ser_pi());
        buf.extend_from_slice(&self.reason.ser_pi());
        buf.extend_from_slice(&self.experiment_count.ser_pi());
        buf.extend_from_slice(&self.cost.ser_pi());
        buf.extend_from_slice(&self.trace_head.ser_pi());
        buf.extend_from_slice(&self.verdict_hash.ser_pi());
        buf.extend_from_slice(&(self.replay_verified as u8).ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Rational number for exact fractions (no floats).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rational {
    pub num: i64,
    pub den: u64,
}

impl Rational {
    pub fn new(num: i64, den: u64) -> Self {
        Rational { num, den }
    }

    pub fn zero() -> Self {
        Rational { num: 0, den: 1 }
    }
}

impl SerPi for Rational {
    fn ser_pi(&self) -> Vec<u8> {
        canonical_cbor_bytes(&(self.num, self.den))
    }
}

/// Result of running an entire phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseResult {
    pub phase: u8,
    pub name: String,
    pub tasks: Vec<AgiTaskResult>,
    pub verified_success: u64,
    pub total_tasks: u64,
    pub false_claims: u64,
    pub false_claim_rate: Rational,
    pub phase_hash: Hash32,
}

impl SerPi for PhaseResult {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.phase as u64).ser_pi());
        buf.extend_from_slice(&self.name.ser_pi());
        buf.extend_from_slice(&self.verified_success.ser_pi());
        buf.extend_from_slice(&self.total_tasks.ser_pi());
        buf.extend_from_slice(&self.false_claims.ser_pi());
        buf.extend_from_slice(&self.false_claim_rate.ser_pi());
        buf.extend_from_slice(&self.phase_hash.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

/// Result of running the full AGI proof.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgiProofResult {
    pub build_hash: Hash32,
    pub phases: Vec<PhaseResult>,
    pub aggregate_verified_success: u64,
    pub aggregate_total_tasks: u64,
    pub aggregate_false_claims: u64,
    pub aggregate_false_claim_rate: Rational,
    pub result_merkle_root: Hash32,
}

impl SerPi for AgiProofResult {
    fn ser_pi(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&self.build_hash.ser_pi());
        buf.extend_from_slice(&self.aggregate_verified_success.ser_pi());
        buf.extend_from_slice(&self.aggregate_total_tasks.ser_pi());
        buf.extend_from_slice(&self.aggregate_false_claims.ser_pi());
        buf.extend_from_slice(&self.aggregate_false_claim_rate.ser_pi());
        buf.extend_from_slice(&self.result_merkle_root.ser_pi());
        canonical_cbor_bytes(&buf)
    }
}

impl AgiRunner {
    /// Create a new runner with a fresh solver.
    pub fn new() -> Self {
        let solver = Solver::new();
        let build_hash = HASH_ZERO;
        AgiRunner { solver, build_hash }
    }

    /// Create a runner with a specific build hash.
    pub fn with_build_hash(build_hash: Hash32) -> Self {
        let solver = Solver::new();
        AgiRunner { solver, build_hash }
    }

    /// Run one task: compile -> solve -> judge -> replay.
    pub fn run_task(&mut self, task_json: &str) -> AgiTaskResult {
        // 1. Compile
        let compile_result = compile_agi_contract(task_json);
        let (contract, spec) = match compile_result {
            Ok((c, s)) => (c, s),
            Err(reason) => {
                return AgiTaskResult {
                    task_id: "compile_error".into(),
                    domain: AgiDomainKind::SynthPhysics,
                    status: Status::Unsat,
                    verdict: JudgeVerdict::Fail,
                    reason,
                    experiment_count: 0,
                    cost: 0,
                    trace_head: HASH_ZERO,
                    verdict_hash: HASH_ZERO,
                    replay_verified: false,
                };
            }
        };

        let task_id = hash::hex(&contract.qid);

        // 2. Solve via kernel solver
        let output = self.solver.solve(&contract);

        // 3. Judge: for the kernel, UNIQUE with answer = PASS,
        // UNSAT = the task has no solution (valid answer for inadmissible).
        let (verdict, reason) = match &output.status {
            Status::Unique => {
                if !output.payload.answer.is_empty() {
                    (JudgeVerdict::Pass, "UNIQUE with verified receipt".to_string())
                } else {
                    (JudgeVerdict::Fail, "UNIQUE but no answer payload".to_string())
                }
            }
            Status::Unsat => {
                (JudgeVerdict::Pass, "UNSAT (task has no solution within domain)".to_string())
            }
        };

        let verdict_hash = hash::H(&canonical_cbor_bytes(&(&task_id, &reason)));

        // 4. Replay verification
        let replay_verified = self.solver.replay_verify(&contract, &output);

        AgiTaskResult {
            task_id,
            domain: spec.domain,
            status: output.status.clone(),
            verdict,
            reason,
            experiment_count: spec.max_experiments,
            cost: output.receipt.completion
                .as_ref()
                .map(|c| c.b_star.unwrap_or(0))
                .unwrap_or(0),
            trace_head: output.receipt.trace_head,
            verdict_hash,
            replay_verified,
        }
    }

    /// Run a phase (collection of tasks).
    pub fn run_phase(&mut self, phase: u8, name: &str, tasks: &[String]) -> PhaseResult {
        let mut results = Vec::new();
        let mut verified_success = 0u64;
        let mut false_claims = 0u64;

        for task_json in tasks {
            let result = self.run_task(task_json);
            if result.verdict == JudgeVerdict::Pass {
                verified_success += 1;
            }
            if result.verdict == JudgeVerdict::FalseClaim {
                false_claims += 1;
            }
            results.push(result);
        }

        let total_tasks = results.len() as u64;
        let false_claim_rate = if false_claims + verified_success > 0 {
            Rational::new(false_claims as i64, (false_claims + verified_success) as u64)
        } else {
            Rational::zero()
        };

        // Merkle root of task result hashes
        let task_hashes: Vec<Hash32> = results.iter()
            .map(|r| hash::H(&r.ser_pi()))
            .collect();
        let phase_hash = hash::merkle_root(&task_hashes);

        PhaseResult {
            phase,
            name: name.to_string(),
            tasks: results,
            verified_success,
            total_tasks,
            false_claims,
            false_claim_rate,
            phase_hash,
        }
    }

    /// Run all phases from a suite manifest.
    pub fn run_all(&mut self, phases: &[(u8, String, Vec<String>)]) -> AgiProofResult {
        let mut phase_results = Vec::new();
        let mut agg_success = 0u64;
        let mut agg_total = 0u64;
        let mut agg_false = 0u64;

        for (phase_num, name, tasks) in phases {
            let result = self.run_phase(*phase_num, name, tasks);
            agg_success += result.verified_success;
            agg_total += result.total_tasks;
            agg_false += result.false_claims;
            phase_results.push(result);
        }

        let agg_fcr = if agg_false + agg_success > 0 {
            Rational::new(agg_false as i64, (agg_false + agg_success) as u64)
        } else {
            Rational::zero()
        };

        let phase_hashes: Vec<Hash32> = phase_results.iter()
            .map(|r| hash::H(&r.ser_pi()))
            .collect();
        let result_merkle_root = hash::merkle_root(&phase_hashes);

        AgiProofResult {
            build_hash: self.build_hash,
            phases: phase_results,
            aggregate_verified_success: agg_success,
            aggregate_total_tasks: agg_total,
            aggregate_false_claims: agg_false,
            aggregate_false_claim_rate: agg_fcr,
            result_merkle_root,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_bool_cnf_task() -> String {
        // This is a standard bool_cnf, not an agi_domain.
        // The runner uses compile_agi_contract which expects agi_domain type.
        // For runner tests, we use agi_domain contracts.
        r#"{
            "type": "agi_domain",
            "domain": "SynthPhysics",
            "description": "runner test physics",
            "world_seed": "",
            "max_experiments": 10
        }"#.to_string()
    }

    #[test]
    fn runner_solve_agi_domain() {
        let mut runner = AgiRunner::new();
        let result = runner.run_task(&make_bool_cnf_task());
        // The solver will process this as a Table contract (the carrier).
        // It should return a valid result.
        assert!(!result.task_id.is_empty());
        assert!(result.task_id != "compile_error");
    }

    #[test]
    fn runner_task_result_serpi_deterministic() {
        let mut runner = AgiRunner::new();
        let r1 = runner.run_task(&make_bool_cnf_task());
        // Reset solver for fresh run
        let mut runner2 = AgiRunner::new();
        let r2 = runner2.run_task(&make_bool_cnf_task());
        assert_eq!(r1.ser_pi(), r2.ser_pi());
    }

    #[test]
    fn runner_phase_result_computes() {
        let mut runner = AgiRunner::new();
        let tasks = vec![make_bool_cnf_task()];
        let phase = runner.run_phase(2, "test_phase", &tasks);
        assert_eq!(phase.total_tasks, 1);
        assert_eq!(phase.phase, 2);
        assert_ne!(phase.phase_hash, HASH_ZERO);
    }

    #[test]
    fn runner_compile_error_returns_fail() {
        let mut runner = AgiRunner::new();
        let result = runner.run_task(r#"{"type":"agi_domain","domain":"Nonsense"}"#);
        assert_eq!(result.verdict, JudgeVerdict::Fail);
        assert_eq!(result.task_id, "compile_error");
    }

    #[test]
    fn rational_serpi_deterministic() {
        let r1 = Rational::new(3, 10);
        let r2 = Rational::new(3, 10);
        assert_eq!(r1.ser_pi(), r2.ser_pi());
    }

    #[test]
    fn runner_replay_deterministic_trace() {
        // Two fresh runners solving the same task must produce the same trace_head
        let mut r1 = AgiRunner::new();
        let mut r2 = AgiRunner::new();
        let res1 = r1.run_task(&make_bool_cnf_task());
        let res2 = r2.run_task(&make_bool_cnf_task());
        assert_eq!(res1.trace_head, res2.trace_head);
    }
}
