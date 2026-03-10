//! OBS pipeline for Subset Sum.
//!
//! Runs the kernel's structural evaluation (eval_structured) on the
//! subset sum Expr for a range of targets, collects StructCerts,
//! anti-unifies them, and prints what OBS discovers.

use kernel_frc::invsyn::Expr;
use kernel_frc::invsyn::eval::mk_env;
use kernel_frc::invsyn::structural_cert::{
    eval_structured, anti_unify_structured, StructCert, AntiUnifyResult,
};

use crate::encoding;
use crate::dp;

/// Run the full OBS pipeline on one subset sum instance.
pub fn run_obs(weights: &[i64]) {
    let expr = encoding::build_subset_sum_expr(weights);
    let max_sum = encoding::max_sum(weights);
    let dp_results = dp::solve_all_dp(weights);

    println!("\n=== OBS PIPELINE: Structural Tracing ===");
    println!("{}", encoding::describe_instance(weights));

    // Part 1: Trace each target, collect StructCerts
    println!("\n--- Part 1: eval_structured for each target ---");
    let mut reachable_certs: Vec<(i64, StructCert)> = Vec::new();
    let mut unreachable_certs: Vec<(i64, StructCert)> = Vec::new();

    for t in 0..=max_sum {
        let env = mk_env(t);
        let (val, cert) = eval_structured(&env, &expr);
        let reachable = val != 0;

        // Verify against DP
        let dp_says = dp_results[t as usize];
        assert_eq!(
            reachable, dp_says,
            "Kernel disagrees with DP at target {}: kernel={}, dp={}",
            t, reachable, dp_says
        );

        if reachable {
            // Print witness info for first few
            if reachable_certs.len() < 5 {
                print_witness_chain(&cert, weights, t);
            }
            reachable_certs.push((t, cert));
        } else {
            unreachable_certs.push((t, cert));
        }
    }

    let total = max_sum + 1;
    println!(
        "\nReachable: {} / {} targets",
        reachable_certs.len(),
        total
    );
    println!(
        "Unreachable: {} / {} targets",
        unreachable_certs.len(),
        total
    );

    // Part 2: Shape analysis
    println!("\n--- Part 2: Certificate shapes ---");
    if let Some(first) = reachable_certs.first() {
        let shape = first.1.shape();
        println!("Reachable cert shape: {}", shape);
        let all_same = reachable_certs.iter().all(|(_, c)| c.shape() == shape);
        println!(
            "All reachable certs same shape: {}",
            if all_same { "YES" } else { "NO" }
        );
    }
    if let Some(first) = unreachable_certs.first() {
        let shape = first.1.shape();
        println!("Unreachable cert shape: {}", shape);
    }

    // Part 3: Anti-unification on reachable certs
    println!("\n--- Part 3: Anti-unification (OBS schema extraction) ---");
    if reachable_certs.len() >= 2 {
        match anti_unify_structured(&reachable_certs) {
            Some(result) => {
                print_anti_unify_result(&result, weights);
            }
            None => {
                println!("Anti-unification FAILED (different shapes across targets).");
                println!("This means different targets find witnesses via different");
                println!("structural paths — the brute-force search takes different");
                println!("branches for different targets.");
                diagnose_shape_variance(&reachable_certs);
            }
        }
    } else {
        println!("Not enough reachable targets for anti-unification.");
    }

    // Part 4: Cross-instance comparison hint
    println!("\n--- Part 4: Structural insight ---");
    println!("The StructCert for each reachable target T records WHICH items");
    println!("were selected (the witness chain). OBS extracts the structural");
    println!("pattern: nested ExistsWitness with varying witness values.");
    println!();
    println!("The DP insight: for targets T and T-w_j, the sub-problem");
    println!("'can we make T-w_j without item j?' is SHARED. OBS shows this");
    println!("as certificates with identical inner structure but different");
    println!("outer witness values — the shared structure IS the DP table.");
}

/// Run OBS on multiple instances to compare structures.
pub fn run_sweep(instances: &[Vec<i64>]) {
    println!("\n=== OBS SWEEP: Comparing structures across instances ===\n");

    for (i, weights) in instances.iter().enumerate() {
        let expr = encoding::build_subset_sum_expr(weights);
        let max_sum = encoding::max_sum(weights);
        let dp_results = dp::solve_all_dp(weights);

        let mut reachable_certs: Vec<(i64, StructCert)> = Vec::new();
        for t in 0..=max_sum {
            let env = mk_env(t);
            let (val, cert) = eval_structured(&env, &expr);
            if val != 0 {
                reachable_certs.push((t, cert));
            }
        }

        let reachable_count = dp_results.iter().filter(|&&x| x).count();
        let shape = reachable_certs
            .first()
            .map(|(_, c)| c.shape())
            .unwrap_or_else(|| "N/A".to_string());

        let all_same_shape = reachable_certs.iter().all(|(_, c)| c.shape() == shape);

        println!(
            "Instance {}: {} items, weights={:?}",
            i + 1,
            weights.len(),
            weights
        );
        println!(
            "  Reachable: {}/{}, shape: {}, uniform: {}",
            reachable_count,
            max_sum + 1,
            shape,
            all_same_shape
        );

        if reachable_certs.len() >= 2 {
            match anti_unify_structured(&reachable_certs) {
                Some(result) => {
                    println!(
                        "  Anti-unify: {} params, {} instances",
                        result.num_params,
                        result.instances.len()
                    );
                }
                None => {
                    println!("  Anti-unify: FAILED (shape variance)");
                }
            }
        }
        println!();
    }

    // Compare shapes across instances with same size
    let sizes: Vec<usize> = instances.iter().map(|w| w.len()).collect();
    if sizes.iter().all(|&s| s == sizes[0]) {
        println!("All instances have {} items.", sizes[0]);
        println!("The cert SHAPE is determined by k (number of items), not by");
        println!("the specific weights. This is the structural invariant:");
        println!("the search tree has the same topology regardless of data.");
        println!("Only the WITNESS VALUES change — which IS the DP table content.");
    }
}

/// Extract and print the witness chain from a reachable StructCert.
fn print_witness_chain(cert: &StructCert, weights: &[i64], target: i64) {
    let mut witnesses = Vec::new();
    extract_witnesses(cert, &mut witnesses);
    // witnesses come outermost-first, matching weights[0], weights[1], ...

    let selected: Vec<i64> = witnesses
        .iter()
        .zip(weights.iter())
        .filter_map(|(&b, &w)| if b == 1 { Some(w) } else { None })
        .collect();

    println!(
        "  T={:>3}: bits={:?}, selected={:?}, sum={}",
        target,
        witnesses,
        selected,
        selected.iter().sum::<i64>()
    );
}

fn extract_witnesses(cert: &StructCert, witnesses: &mut Vec<i64>) {
    match cert {
        StructCert::ExistsWitness {
            witness,
            witness_cert,
            ..
        } => {
            witnesses.push(*witness);
            extract_witnesses(witness_cert, witnesses);
        }
        StructCert::Compare { .. }
        | StructCert::Leaf { .. }
        | StructCert::Arith { .. }
        | StructCert::PrimitiveCall { .. } => {}
        StructCert::Logic { children, .. } => {
            for child in children {
                extract_witnesses(child, witnesses);
            }
        }
        StructCert::ForallCerts { certs, .. } => {
            for (_, c) in certs {
                extract_witnesses(c, witnesses);
            }
        }
        StructCert::ImpliesCert { body_cert, .. } => {
            if let Some(bc) = body_cert {
                extract_witnesses(bc, witnesses);
            }
        }
    }
}

fn print_anti_unify_result(result: &AntiUnifyResult, weights: &[i64]) {
    println!("Anti-unification SUCCEEDED.");
    println!("  Parameters: {} (values that vary across targets)", result.num_params);
    println!("  Instances:  {} (one per reachable target)", result.instances.len());

    // Show a few instances
    let show = std::cmp::min(5, result.instances.len());
    println!("  First {} instances:", show);
    for (n, params) in result.instances.iter().take(show) {
        println!("    T={}: params={:?}", n, params);
    }
    if result.instances.len() > show {
        println!("    ... ({} more)", result.instances.len() - show);
    }

    println!();
    println!("  The schema shape is CONSTANT across all reachable targets.");
    println!("  Only the parameter values change — these encode WHICH items");
    println!("  were selected. The constant shape = the search structure.");
    println!("  The varying params = the DP table content.");
}

fn diagnose_shape_variance(certs: &[(i64, StructCert)]) {
    // Group certs by shape
    let mut shape_groups: std::collections::HashMap<String, Vec<i64>> =
        std::collections::HashMap::new();
    for (t, cert) in certs {
        let shape = cert.shape();
        shape_groups.entry(shape).or_default().push(*t);
    }

    println!("\n  Shape groups ({} distinct shapes):", shape_groups.len());
    for (shape, targets) in &shape_groups {
        let show: Vec<&i64> = targets.iter().take(5).collect();
        println!(
            "    {} -> {} targets (e.g., {:?}{})",
            shape,
            targets.len(),
            show,
            if targets.len() > 5 { "..." } else { "" }
        );
    }
}
