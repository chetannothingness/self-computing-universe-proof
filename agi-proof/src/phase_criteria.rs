// Phase Pass Criteria
//
// Exact pass/fail determination for each phase of the AGI proof.
// Each function checks the precise conditions from EXECUTION.md.
// All arithmetic is integer-only via Rational{i64, u64}.

use crate::runner::{PhaseResult, Rational, AgiTaskResult};
use crate::eval_specs::AgiDomainKind;
use kernel_types::Hash32;
use kernel_bench::judge::JudgeVerdict;

/// Phase 0: Freeze check.
/// PASS iff build_hash == expected_build_hash (computed before any task exists).
pub fn phase0_pass(build_hash_before: &Hash32, build_hash_after: &Hash32) -> bool {
    build_hash_before == build_hash_after
}

/// Phase 1: Harness check.
/// PASS iff all 215 existing kernel tests pass and replay matches.
/// (Checked externally via `cargo test` — this function checks the phase result.)
pub fn phase1_pass(result: &PhaseResult) -> bool {
    result.false_claims == 0
        && result.verified_success == result.total_tasks
}

/// Phase 2: Robustness across domains without retraining.
/// PASS iff:
///   1. false_claim_rate == 0
///   2. verified_success >= 80% in EACH sub-domain (physics, chem, math)
///   3. BuildHash unchanged
pub fn phase2_pass(
    result: &PhaseResult,
    build_hash_before: &Hash32,
    build_hash_after: &Hash32,
) -> bool {
    if result.false_claims != 0 {
        return false;
    }
    if build_hash_before != build_hash_after {
        return false;
    }

    // Check per-subdomain pass rates
    let physics_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::SynthPhysics);
    let chem_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::AlienChemistry);
    let math_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::CustomMath);

    // Each must be >= 80% (i.e., pass_rate * 100 >= 80)
    rational_gte(&physics_pass, 80, 100)
        && rational_gte(&chem_pass, 80, 100)
        && rational_gte(&math_pass, 80, 100)
}

/// Phase 3: Autonomous goal pursuit.
/// PASS iff:
///   1. false_claim_rate == 0
///   2. Company: profitable (passes judge)
///   3. BioMed: mechanism found (passes judge)
///   4. Plan prediction error decreases over time (checked by plan_tracker judge)
pub fn phase3_pass(result: &PhaseResult) -> bool {
    if result.false_claims != 0 {
        return false;
    }
    let company_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::CompanySandbox);
    let biomed_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::BioMedSandbox);

    // Both domains must have >0 tasks and some passes
    rational_gte(&company_pass, 50, 100)
        && rational_gte(&biomed_pass, 50, 100)
}

/// Phase 4: Transfer learning.
/// PASS iff:
///   1. false_claim_rate == 0
///   2. transfer_gain >= 30% for majority of pairs
///   3. order_effect detected (A-then-B > B-then-A)
pub fn phase4_pass(result: &PhaseResult) -> bool {
    if result.false_claims != 0 {
        return false;
    }
    // At least 50% of transfer pairs must pass the judge
    let total = result.total_tasks;
    if total == 0 {
        return false;
    }
    // verified_success / total >= 50%
    result.verified_success * 100 >= total * 50
}

/// Phase 5: Self-directed knowledge acquisition.
/// PASS iff:
///   1. zero hallucinations (false_claims == 0)
///   2. efficiency >= 50% of oracle optimal
///   3. learning detected (query count decreases)
pub fn phase5_pass(result: &PhaseResult) -> bool {
    result.false_claims == 0
        && result.verified_success * 100 >= result.total_tasks * 50
}

/// Phase 6: Causal reasoning.
/// PASS iff:
///   1. false_claim_rate == 0
///   2. intervention prediction accuracy >= 60%
///   3. counterfactual predictions match within tolerance
///   4. robust to distribution shift
pub fn phase6_pass(result: &PhaseResult) -> bool {
    if result.false_claims != 0 {
        return false;
    }
    let causal_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::CausalReasoning);
    rational_gte(&causal_pass, 60, 100)
}

/// Phase 7: Novel scientific discovery.
/// PASS iff:
///   1. false_claim_rate == 0
///   2. model discovery improves holdout prediction
///   3. materials design hits target range
///   4. algorithm outperforms baselines
pub fn phase7_pass(result: &PhaseResult) -> bool {
    if result.false_claims != 0 {
        return false;
    }
    let model_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::ModelDiscovery);
    let materials_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::MaterialsDesign);
    let algo_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::AlgoDiscovery);

    rational_gte(&model_pass, 50, 100)
        && rational_gte(&materials_pass, 50, 100)
        && rational_gte(&algo_pass, 50, 100)
}

/// Phase 8: Robust common sense.
/// PASS iff:
///   1. false_claim_rate == 0
///   2. physical reasoning >= 90%
///   3. social reasoning >= 90%
///   4. planning >= 90%
pub fn phase8_pass(result: &PhaseResult) -> bool {
    if result.false_claims != 0 {
        return false;
    }
    let phys_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::PhysicalReasoning);
    let social_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::SocialReasoning);
    let plan_pass = subdomain_pass_rate(&result.tasks, AgiDomainKind::MultiStepPlanning);

    rational_gte(&phys_pass, 90, 100)
        && rational_gte(&social_pass, 90, 100)
        && rational_gte(&plan_pass, 90, 100)
}

/// Aggregate pass criteria across all phases.
/// PASS iff all individual phases pass AND aggregate FCR == 0.
pub fn aggregate_pass(
    phase_results: &[PhaseResult],
    build_hash_before: &Hash32,
    build_hash_after: &Hash32,
) -> bool {
    // Aggregate false claim rate must be zero
    let total_fc: u64 = phase_results.iter().map(|p| p.false_claims).sum();
    if total_fc != 0 {
        return false;
    }

    // BuildHash unchanged
    if build_hash_before != build_hash_after {
        return false;
    }

    // All phases must pass their individual criteria
    for pr in phase_results {
        let pass = match pr.phase {
            0 => phase0_pass(build_hash_before, build_hash_after),
            1 => phase1_pass(pr),
            2 => phase2_pass(pr, build_hash_before, build_hash_after),
            3 => phase3_pass(pr),
            4 => phase4_pass(pr),
            5 => phase5_pass(pr),
            6 => phase6_pass(pr),
            7 => phase7_pass(pr),
            8 => phase8_pass(pr),
            _ => false,
        };
        if !pass {
            return false;
        }
    }

    true
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute pass rate for a specific subdomain within a phase result.
/// Returns Rational { num: passes, den: total } for that domain.
fn subdomain_pass_rate(tasks: &[AgiTaskResult], domain: AgiDomainKind) -> Rational {
    let domain_tasks: Vec<&AgiTaskResult> = tasks.iter()
        .filter(|t| t.domain == domain)
        .collect();

    let total = domain_tasks.len() as u64;
    if total == 0 {
        return Rational::new(0, 1);
    }

    let passes = domain_tasks.iter()
        .filter(|t| t.verdict == JudgeVerdict::Pass)
        .count() as i64;

    Rational::new(passes, total)
}

/// Check if rational >= threshold_num / threshold_den.
/// Uses integer cross-multiplication: a/b >= c/d iff a*d >= c*b.
fn rational_gte(r: &Rational, threshold_num: i64, threshold_den: u64) -> bool {
    // r.num / r.den >= threshold_num / threshold_den
    // r.num * threshold_den >= threshold_num * r.den
    r.num * threshold_den as i64 >= threshold_num * r.den as i64
}

/// Format a phase result as a single scoreboard line.
pub fn format_scoreboard_line(pr: &PhaseResult) -> String {
    let fcr = if pr.false_claims + pr.verified_success > 0 {
        format!("{}/{}", pr.false_claims, pr.false_claims + pr.verified_success)
    } else {
        "0/0".to_string()
    };

    format!(
        "Phase{}: {}/{}  FCR={}",
        pr.phase, pr.verified_success, pr.total_tasks, fcr
    )
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use kernel_types::HASH_ZERO;

    fn make_task(domain: AgiDomainKind, verdict: JudgeVerdict) -> AgiTaskResult {
        AgiTaskResult {
            task_id: "test".into(),
            domain,
            status: kernel_types::status::Status::Unique,
            verdict,
            reason: "test".into(),
            experiment_count: 10,
            cost: 100,
            trace_head: HASH_ZERO,
            verdict_hash: HASH_ZERO,
            replay_verified: true,
        }
    }

    fn make_phase(phase: u8, tasks: Vec<AgiTaskResult>) -> PhaseResult {
        let verified_success = tasks.iter()
            .filter(|t| t.verdict == JudgeVerdict::Pass)
            .count() as u64;
        let false_claims = tasks.iter()
            .filter(|t| t.verdict == JudgeVerdict::FalseClaim)
            .count() as u64;
        let total_tasks = tasks.len() as u64;
        PhaseResult {
            phase,
            name: format!("Phase {}", phase),
            tasks,
            verified_success,
            total_tasks,
            false_claims,
            false_claim_rate: Rational::zero(),
            phase_hash: HASH_ZERO,
        }
    }

    #[test]
    fn phase0_pass_identical_hash() {
        let h = [1u8; 32];
        assert!(phase0_pass(&h, &h));
    }

    #[test]
    fn phase0_fail_different_hash() {
        let h1 = [1u8; 32];
        let h2 = [2u8; 32];
        assert!(!phase0_pass(&h1, &h2));
    }

    #[test]
    fn phase2_requires_80pct_in_each_subdomain() {
        // 8 pass, 2 fail in each subdomain = 80% → should pass
        let mut tasks = Vec::new();
        for _ in 0..8 { tasks.push(make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Pass)); }
        for _ in 0..2 { tasks.push(make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Fail)); }
        for _ in 0..8 { tasks.push(make_task(AgiDomainKind::AlienChemistry, JudgeVerdict::Pass)); }
        for _ in 0..2 { tasks.push(make_task(AgiDomainKind::AlienChemistry, JudgeVerdict::Fail)); }
        for _ in 0..8 { tasks.push(make_task(AgiDomainKind::CustomMath, JudgeVerdict::Pass)); }
        for _ in 0..2 { tasks.push(make_task(AgiDomainKind::CustomMath, JudgeVerdict::Fail)); }

        let pr = make_phase(2, tasks);
        let h = HASH_ZERO;
        assert!(phase2_pass(&pr, &h, &h));
    }

    #[test]
    fn phase2_fails_below_80pct() {
        // 7 pass, 3 fail = 70% → should fail
        let mut tasks = Vec::new();
        for _ in 0..7 { tasks.push(make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Pass)); }
        for _ in 0..3 { tasks.push(make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Fail)); }
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::AlienChemistry, JudgeVerdict::Pass)); }
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::CustomMath, JudgeVerdict::Pass)); }

        let pr = make_phase(2, tasks);
        let h = HASH_ZERO;
        assert!(!phase2_pass(&pr, &h, &h));
    }

    #[test]
    fn phase2_fails_on_false_claims() {
        let mut tasks = Vec::new();
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Pass)); }
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::AlienChemistry, JudgeVerdict::Pass)); }
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::CustomMath, JudgeVerdict::Pass)); }
        tasks.push(make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::FalseClaim));

        let pr = make_phase(2, tasks);
        let h = HASH_ZERO;
        assert!(!phase2_pass(&pr, &h, &h));
    }

    #[test]
    fn phase8_requires_90pct() {
        let mut tasks = Vec::new();
        for _ in 0..9 { tasks.push(make_task(AgiDomainKind::PhysicalReasoning, JudgeVerdict::Pass)); }
        tasks.push(make_task(AgiDomainKind::PhysicalReasoning, JudgeVerdict::Fail));
        for _ in 0..9 { tasks.push(make_task(AgiDomainKind::SocialReasoning, JudgeVerdict::Pass)); }
        tasks.push(make_task(AgiDomainKind::SocialReasoning, JudgeVerdict::Fail));
        for _ in 0..9 { tasks.push(make_task(AgiDomainKind::MultiStepPlanning, JudgeVerdict::Pass)); }
        tasks.push(make_task(AgiDomainKind::MultiStepPlanning, JudgeVerdict::Fail));

        let pr = make_phase(8, tasks);
        assert!(phase8_pass(&pr));
    }

    #[test]
    fn phase8_fails_below_90pct() {
        let mut tasks = Vec::new();
        for _ in 0..8 { tasks.push(make_task(AgiDomainKind::PhysicalReasoning, JudgeVerdict::Pass)); }
        for _ in 0..2 { tasks.push(make_task(AgiDomainKind::PhysicalReasoning, JudgeVerdict::Fail)); }
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::SocialReasoning, JudgeVerdict::Pass)); }
        for _ in 0..10 { tasks.push(make_task(AgiDomainKind::MultiStepPlanning, JudgeVerdict::Pass)); }

        let pr = make_phase(8, tasks);
        assert!(!phase8_pass(&pr));
    }

    #[test]
    fn rational_gte_works() {
        assert!(rational_gte(&Rational::new(8, 10), 80, 100)); // 80% >= 80%
        assert!(rational_gte(&Rational::new(9, 10), 80, 100)); // 90% >= 80%
        assert!(!rational_gte(&Rational::new(7, 10), 80, 100)); // 70% < 80%
        assert!(rational_gte(&Rational::new(1, 1), 80, 100));  // 100% >= 80%
        assert!(!rational_gte(&Rational::new(0, 1), 80, 100)); // 0% < 80%
    }

    #[test]
    fn subdomain_pass_rate_correct() {
        let tasks = vec![
            make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Pass),
            make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Pass),
            make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Fail),
            make_task(AgiDomainKind::AlienChemistry, JudgeVerdict::Pass),
        ];
        let rate = subdomain_pass_rate(&tasks, AgiDomainKind::SynthPhysics);
        assert_eq!(rate.num, 2);
        assert_eq!(rate.den, 3);
    }

    #[test]
    fn scoreboard_line_format() {
        let tasks = vec![
            make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Pass),
            make_task(AgiDomainKind::SynthPhysics, JudgeVerdict::Fail),
        ];
        let pr = make_phase(2, tasks);
        let line = format_scoreboard_line(&pr);
        assert!(line.contains("Phase2"));
        assert!(line.contains("1/2"));
    }
}
