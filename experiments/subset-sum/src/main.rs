mod encoding;
mod obs_demo;
mod dp;
mod structure_speedup;

use clap::Parser;
use rand::Rng;

#[derive(Parser)]
#[command(name = "subset-sum")]
#[command(about = "Subset Sum efficiency experiment: brute force vs DP, with kernel OBS tracing")]
struct Cli {
    /// Comma-separated weights (e.g., "12,15,23,31,8")
    #[arg(long)]
    weights: Option<String>,

    /// Specific target to check (if omitted, sweeps all targets 0..max_sum)
    #[arg(long)]
    target: Option<i64>,

    /// Generate random weights: number of items
    #[arg(long, default_value_t = 8)]
    size: usize,

    /// Max weight value for random generation
    #[arg(long, default_value_t = 30)]
    max_weight: i64,

    /// Run sweep: multiple instances, compare OBS structures
    #[arg(long)]
    sweep: bool,

    /// Run benchmark at increasing k values
    #[arg(long)]
    benchmark: bool,

    /// Run structure-guided speedup demo
    #[arg(long)]
    structure: bool,

    /// Number of instances for sweep mode
    #[arg(long, default_value_t = 4)]
    sweep_count: usize,

    /// Seed for random generation (for reproducibility)
    #[arg(long)]
    seed: Option<u64>,
}

fn main() {
    let cli = Cli::parse();

    let weights = if let Some(ref w) = cli.weights {
        parse_weights(w)
    } else {
        random_weights(cli.size, cli.max_weight, cli.seed)
    };

    println!("=== SUBSET SUM EFFICIENCY EXPERIMENT ===");
    println!("{}", encoding::describe_instance(&weights));

    if cli.sweep {
        run_sweep_mode(&cli, &weights);
    } else if cli.benchmark {
        run_benchmark_mode(&cli);
    } else if cli.structure {
        run_structure_mode(&cli, &weights);
    } else {
        run_single_mode(&weights, cli.target);
    }
}

fn run_single_mode(weights: &[i64], target: Option<i64>) {
    if let Some(t) = target {
        // Single target query
        let expr = encoding::build_subset_sum_expr(weights);
        let env = kernel_frc::invsyn::eval::mk_env(t);
        let (val, cert) = kernel_frc::invsyn::structural_cert::eval_structured(&env, &expr);

        let dp_result = dp::can_sum_dp(weights, t);
        let brute_result = dp::can_sum_brute(weights, t);

        println!("\nTarget: {}", t);
        println!("Kernel (eval_structured): {}", val != 0);
        println!("DP:                       {}", dp_result);
        println!("Brute force:              {}", brute_result);
        println!("Certificate shape:        {}", cert.shape());
        println!("All agree:                {}", (val != 0) == dp_result && dp_result == brute_result);
    } else {
        // Full analysis: OBS + benchmark
        obs_demo::run_obs(weights);
        dp::benchmark(weights);
    }
}

fn run_sweep_mode(cli: &Cli, first_weights: &[i64]) {
    let mut instances = vec![first_weights.to_vec()];
    for i in 1..cli.sweep_count {
        let seed = cli.seed.map(|s| s + i as u64);
        instances.push(random_weights(first_weights.len(), cli.max_weight, seed));
    }
    obs_demo::run_sweep(&instances);
}

fn run_structure_mode(cli: &Cli, weights: &[i64]) {
    // Generate multiple instances with same k, different weights
    let mut instances = vec![weights.to_vec()];
    for i in 1..cli.sweep_count {
        let seed = cli.seed.map(|s| s + 100 + i as u64).or(Some(100 + i as u64));
        instances.push(random_weights(weights.len(), cli.max_weight, seed));
    }
    structure_speedup::run_kernel_pipeline(&instances);
}

fn run_benchmark_mode(cli: &Cli) {
    println!("\n=== SCALING BENCHMARK ===");
    println!("Comparing brute force (2^k) vs DP (k * max_sum) at increasing k\n");

    for k in [5, 8, 10, 12, 15, 18, 20] {
        if k > 25 {
            println!("k={}: skipping (2^{} = {} too large for brute force)", k, k, 1u64 << k);
            continue;
        }
        let weights = random_weights(k, cli.max_weight, cli.seed);
        println!("--- k={} ---", k);
        dp::benchmark(&weights);
        println!();
    }
}

fn parse_weights(s: &str) -> Vec<i64> {
    s.split(',')
        .map(|x| x.trim().parse::<i64>().expect("Invalid weight"))
        .collect()
}

fn random_weights(size: usize, max_weight: i64, seed: Option<u64>) -> Vec<i64> {
    let mut rng = if let Some(s) = seed {
        use rand::SeedableRng;
        rand::rngs::StdRng::seed_from_u64(s)
    } else {
        use rand::SeedableRng;
        rand::rngs::StdRng::from_entropy()
    };

    (0..size)
        .map(|_| rng.gen_range(1..=max_weight))
        .collect()
}
