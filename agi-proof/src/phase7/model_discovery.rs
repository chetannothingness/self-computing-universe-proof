// Phase 7A: Model Discovery
// Discover hidden polynomial equations from noisy training data.
// All arithmetic is integer-only (i64/u64), zero floats.

use kernel_types::hash;
use kernel_bench::judge::JudgeVerdict;
use serde::{Serialize, Deserialize};

/// A world containing a hidden polynomial equation, training data with noise,
/// and holdout data for evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryWorld {
    pub seed: [u8; 32],
    pub hidden_equation: SymbolicEquation,
    pub training_data: Vec<(i64, i64)>,
    pub holdout_data: Vec<(i64, i64)>,
    pub noise_amplitude_milli: i64,
}

/// A symbolic polynomial equation: sum of terms.
/// Each term is coefficient_milli * x^variable_power.
/// The final evaluated value is divided by 1000 to account for milli-units.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolicEquation {
    pub terms: Vec<EquationTerm>,
}

/// A single term: coefficient_milli * x^variable_power.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EquationTerm {
    pub coefficient_milli: i64,
    pub variable_power: u32,
}

/// A proposed model submitted by the agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedModel {
    pub equation: SymbolicEquation,
    pub predictions: Vec<(i64, i64)>,
}

/// Generate a deterministic discovery world from seed and episode.
///
/// The hidden equation has 2-5 terms with integer coefficients (in milli-units).
/// Training data: equation evaluated at integer x values with deterministic noise.
/// Holdout data: equation evaluated at different integer x values (no noise).
pub fn generate_discovery_world(seed: &[u8; 32], episode: u32) -> DiscoveryWorld {
    // Derive episode seed
    let mut ep_buf = Vec::new();
    ep_buf.extend_from_slice(seed);
    ep_buf.extend_from_slice(&episode.to_le_bytes());
    let ep_seed = hash::H(&ep_buf);

    // Determine number of terms: 2-5
    let num_terms = 2 + (ep_seed[0] as usize % 4);

    // Generate equation terms deterministically
    let mut terms = Vec::with_capacity(num_terms);
    for i in 0..num_terms {
        // Derive per-term hash
        let mut term_buf = Vec::new();
        term_buf.extend_from_slice(&ep_seed);
        term_buf.extend_from_slice(b"term");
        term_buf.extend_from_slice(&(i as u32).to_le_bytes());
        let term_hash = hash::H(&term_buf);

        // Coefficient in milli-units: range [-5000, 5000] (i.e., real range [-5, 5])
        let raw_coeff = ((term_hash[0] as i64) | ((term_hash[1] as i64) << 8)) % 5001;
        let sign = if term_hash[2] % 2 == 0 { 1i64 } else { -1i64 };
        let coefficient_milli = sign * (raw_coeff.abs() + 100); // ensure non-zero, min |100|

        // Power: 0 to 4
        let variable_power = (term_hash[3] as u32) % 5;

        terms.push(EquationTerm {
            coefficient_milli,
            variable_power,
        });
    }

    let hidden_equation = SymbolicEquation { terms };

    // Noise amplitude in milli-units
    let noise_amplitude_milli = 50 + (ep_seed[4] as i64 % 200);

    // Generate training data: x values from -20 to 19 (40 points)
    let mut training_data = Vec::with_capacity(40);
    for x_idx in 0..40i64 {
        let x = x_idx - 20;
        let clean_y = evaluate_equation(&hidden_equation, x);

        // Deterministic noise from hash
        let mut noise_buf = Vec::new();
        noise_buf.extend_from_slice(&ep_seed);
        noise_buf.extend_from_slice(b"train_noise");
        noise_buf.extend_from_slice(&(x_idx as u32).to_le_bytes());
        let noise_hash = hash::H(&noise_buf);
        let raw_noise = ((noise_hash[0] as i64) | ((noise_hash[1] as i64) << 8)) % (noise_amplitude_milli + 1);
        let noise_sign = if noise_hash[2] % 2 == 0 { 1i64 } else { -1i64 };
        let noise = noise_sign * raw_noise;

        training_data.push((x, clean_y + noise));
    }

    // Generate holdout data: x values from 25 to 34 (10 points, outside training range)
    let mut holdout_data = Vec::with_capacity(10);
    for x_idx in 0..10i64 {
        let x = x_idx + 25;
        let clean_y = evaluate_equation(&hidden_equation, x);
        holdout_data.push((x, clean_y));
    }

    DiscoveryWorld {
        seed: *seed,
        hidden_equation,
        training_data,
        holdout_data,
        noise_amplitude_milli,
    }
}

/// Evaluate a polynomial equation at integer x.
/// Result = sum(coefficient_milli * x^power) / 1000
/// All arithmetic is integer.
pub fn evaluate_equation(eq: &SymbolicEquation, x: i64) -> i64 {
    let mut sum = 0i64;
    for term in &eq.terms {
        let x_pow = int_pow(x, term.variable_power);
        sum += term.coefficient_milli.saturating_mul(x_pow);
    }
    sum / 1000
}

/// Integer exponentiation: base^exp.
fn int_pow(base: i64, exp: u32) -> i64 {
    if exp == 0 {
        return 1;
    }
    let mut result = 1i64;
    let mut b = base;
    let mut e = exp;
    while e > 0 {
        if e & 1 == 1 {
            result = result.saturating_mul(b);
        }
        e >>= 1;
        if e > 0 {
            b = b.saturating_mul(b);
        }
    }
    result
}

/// Compute mean absolute error between actual and predicted data points.
/// Both slices are (x, y) pairs; comparison is done on y values at matching indices.
/// Returns MAE as an i64.
pub fn compute_prediction_error(actual: &[(i64, i64)], predicted: &[(i64, i64)]) -> i64 {
    let count = actual.len().min(predicted.len());
    if count == 0 {
        return i64::MAX;
    }

    let mut total_error = 0i64;
    for i in 0..count {
        let diff = actual[i].1 - predicted[i].1;
        total_error = total_error.saturating_add(diff.abs());
    }
    total_error / count as i64
}

/// Judge a proposed model against a null model.
/// PASS iff proposed_error < 90% of null_model_error.
/// Comparison uses integer arithmetic: proposed_error * 1000 < null_model_error * 900.
pub fn judge_discovery(
    world: &DiscoveryWorld,
    proposed: &ProposedModel,
    null_model_error: i64,
) -> JudgeVerdict {
    let proposed_error = compute_prediction_error(&world.holdout_data, &proposed.predictions);

    // PASS iff proposed_error < 90% of null_model_error
    // Equivalent: proposed_error * 1000 < null_model_error * 900
    if proposed_error * 1000 < null_model_error * 900 {
        JudgeVerdict::Pass
    } else {
        JudgeVerdict::Fail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_world_deterministic() {
        let seed = [42u8; 32];
        let w1 = generate_discovery_world(&seed, 0);
        let w2 = generate_discovery_world(&seed, 0);

        assert_eq!(w1.hidden_equation, w2.hidden_equation);
        assert_eq!(w1.training_data, w2.training_data);
        assert_eq!(w1.holdout_data, w2.holdout_data);
        assert_eq!(w1.noise_amplitude_milli, w2.noise_amplitude_milli);

        // Different episode produces different world
        let w3 = generate_discovery_world(&seed, 1);
        assert_ne!(w1.hidden_equation, w3.hidden_equation);
    }

    #[test]
    fn symbolic_equation_evaluates_correctly() {
        // Equation: 2000 * x^2 + 500 * x^1 + 3000 * x^0
        // In real terms: 2 * x^2 + 0.5 * x + 3
        let eq = SymbolicEquation {
            terms: vec![
                EquationTerm { coefficient_milli: 2000, variable_power: 2 },
                EquationTerm { coefficient_milli: 500, variable_power: 1 },
                EquationTerm { coefficient_milli: 3000, variable_power: 0 },
            ],
        };

        // x=0: (2000*0 + 500*0 + 3000*1) / 1000 = 3000 / 1000 = 3
        assert_eq!(evaluate_equation(&eq, 0), 3);

        // x=1: (2000*1 + 500*1 + 3000*1) / 1000 = 5500 / 1000 = 5
        assert_eq!(evaluate_equation(&eq, 1), 5);

        // x=3: (2000*9 + 500*3 + 3000*1) / 1000 = (18000 + 1500 + 3000) / 1000 = 22
        assert_eq!(evaluate_equation(&eq, 3), 22);

        // x=-1: (2000*1 + 500*(-1) + 3000*1) / 1000 = (2000 - 500 + 3000) / 1000 = 4
        assert_eq!(evaluate_equation(&eq, -1), 4);

        // x=10: (2000*100 + 500*10 + 3000*1) / 1000 = (200000 + 5000 + 3000) / 1000 = 208
        assert_eq!(evaluate_equation(&eq, 10), 208);
    }

    #[test]
    fn judge_discovery_pass_on_improvement() {
        let seed = [7u8; 32];
        let world = generate_discovery_world(&seed, 0);

        // Compute the correct predictions using the hidden equation
        let predictions: Vec<(i64, i64)> = world.holdout_data.iter()
            .map(|&(x, _)| (x, evaluate_equation(&world.hidden_equation, x)))
            .collect();

        let proposed = ProposedModel {
            equation: world.hidden_equation.clone(),
            predictions,
        };

        // Null model: predict 0 for everything
        let null_predictions: Vec<(i64, i64)> = world.holdout_data.iter()
            .map(|&(x, _)| (x, 0))
            .collect();
        let null_error = compute_prediction_error(&world.holdout_data, &null_predictions);

        let verdict = judge_discovery(&world, &proposed, null_error);
        assert_eq!(verdict, JudgeVerdict::Pass);
    }

    #[test]
    fn judge_discovery_fail_on_no_improvement() {
        let seed = [11u8; 32];
        let world = generate_discovery_world(&seed, 0);

        // Null model: predict 0 for everything
        let null_predictions: Vec<(i64, i64)> = world.holdout_data.iter()
            .map(|&(x, _)| (x, 0))
            .collect();
        let null_error = compute_prediction_error(&world.holdout_data, &null_predictions);

        // Proposed model is the same null model (no improvement)
        let proposed = ProposedModel {
            equation: SymbolicEquation { terms: vec![] },
            predictions: null_predictions,
        };

        let verdict = judge_discovery(&world, &proposed, null_error);
        assert_eq!(verdict, JudgeVerdict::Fail);
    }
}
