//! Dynamic programming solver for Subset Sum.
//!
//! dp[j][t] = can we make sum t using the first j items?
//! dp[0][0] = true, dp[0][t>0] = false
//! dp[j][t] = dp[j-1][t] || (t >= w_j && dp[j-1][t - w_j])
//!
//! Total work: O(k * max_sum) vs brute force O(2^k * max_sum).

use std::time::Instant;

/// Check if any subset of `weights` sums to `target`.
pub fn can_sum_dp(weights: &[i64], target: i64) -> bool {
    if target < 0 {
        return false;
    }
    let t = target as usize;
    let k = weights.len();

    // dp[t] = can we make sum t using items considered so far?
    let mut dp = vec![false; t + 1];
    dp[0] = true;

    for j in 0..k {
        let w = weights[j] as usize;
        // Iterate backwards to avoid using the same item twice.
        for s in (w..=t).rev() {
            if dp[s - w] {
                dp[s] = true;
            }
        }
    }
    dp[t]
}

/// Compute reachability for ALL targets in [0, max_sum].
/// Returns a Vec<bool> where result[t] = can_sum_dp(weights, t).
pub fn solve_all_dp(weights: &[i64]) -> Vec<bool> {
    let max_sum: i64 = weights.iter().sum();
    let max = max_sum as usize;

    let mut dp = vec![false; max + 1];
    dp[0] = true;

    for j in 0..weights.len() {
        let w = weights[j] as usize;
        for s in (w..=max).rev() {
            if dp[s - w] {
                dp[s] = true;
            }
        }
    }
    dp
}

/// Brute-force check: enumerate all 2^k subsets.
pub fn can_sum_brute(weights: &[i64], target: i64) -> bool {
    let k = weights.len();
    for mask in 0..(1u64 << k) {
        let mut s: i64 = 0;
        for j in 0..k {
            if mask & (1 << j) != 0 {
                s += weights[j];
            }
        }
        if s == target {
            return true;
        }
    }
    false
}

/// Solve all targets via brute force.
pub fn solve_all_brute(weights: &[i64]) -> Vec<bool> {
    let max_sum: i64 = weights.iter().sum();
    let mut results = vec![false; max_sum as usize + 1];
    let k = weights.len();
    for mask in 0..(1u64 << k) {
        let mut s: i64 = 0;
        for j in 0..k {
            if mask & (1 << j) != 0 {
                s += weights[j];
            }
        }
        if s >= 0 && (s as usize) < results.len() {
            results[s as usize] = true;
        }
    }
    results
}

/// Benchmark both approaches and print results.
pub fn benchmark(weights: &[i64]) {
    let k = weights.len();
    let max_sum: i64 = weights.iter().sum();

    println!("\n=== BENCHMARK: Brute Force vs DP ===");
    println!("Items: {}, Max sum: {}", k, max_sum);
    println!("Brute force work: 2^{} = {} subset evaluations", k, 1u64 << k);
    println!("DP work: {} * {} = {} table entries", k, max_sum, k as i64 * max_sum);

    // Brute force
    let start = Instant::now();
    let brute_results = solve_all_brute(weights);
    let brute_time = start.elapsed();

    // DP
    let start = Instant::now();
    let dp_results = solve_all_dp(weights);
    let dp_time = start.elapsed();

    // Verify agreement
    let mut agree = true;
    for t in 0..=max_sum as usize {
        if brute_results[t] != dp_results[t] {
            println!("DISAGREEMENT at t={}: brute={}, dp={}", t, brute_results[t], dp_results[t]);
            agree = false;
        }
    }

    let reachable = dp_results.iter().filter(|&&x| x).count();
    println!("\nReachable targets: {} / {}", reachable, max_sum + 1);
    println!("Brute force time: {:?}", brute_time);
    println!("DP time:          {:?}", dp_time);
    if brute_time.as_nanos() > 0 && dp_time.as_nanos() > 0 {
        let ratio = brute_time.as_nanos() as f64 / dp_time.as_nanos() as f64;
        println!("Speedup:          {:.1}x", ratio);
    }
    if agree {
        println!("Agreement:        ALL MATCH");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dp_matches_brute() {
        let weights = vec![2, 3, 5, 7, 11];
        let brute = solve_all_brute(&weights);
        let dp = solve_all_dp(&weights);
        assert_eq!(brute.len(), dp.len());
        for t in 0..brute.len() {
            assert_eq!(brute[t], dp[t], "Mismatch at t={}", t);
        }
    }

    #[test]
    fn dp_single_queries() {
        let weights = vec![12, 15, 23, 31, 8];
        assert!(can_sum_dp(&weights, 0));
        assert!(can_sum_dp(&weights, 12));
        assert!(can_sum_dp(&weights, 27)); // 12+15
        assert!(can_sum_dp(&weights, 89)); // all
        assert!(!can_sum_dp(&weights, 1));
        assert!(!can_sum_dp(&weights, 90));
    }
}
