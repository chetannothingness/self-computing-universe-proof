//! Verification Trace Experiment: following Goldbach's kernel flow exactly.
//!
//! Key insight from analysis:
//!   Goldbach = verification trace (every n returns true via Implies guard)
//!   Subset Sum = computation trace (some targets unreachable → false)
//!
//! This module tests: what happens when we feed ONLY reachable targets
//! (verification-only traces) to the kernel pipeline?
//!
//! Pipeline paths tested:
//!   A. Flat traces (eval_bool_with_trace + anti_unify) — Goldbach's "generate_trace_corpus" path
//!   B. Tree certs (eval_structured + anti_unify_structured) — Goldbach's "try_schema_certified" path
//!   C. Existence certs (extract_existence_certs + anti_unify_exist_certs) — obligation-level analysis
//!   D. Manual corpus construction → emit_certificates — full pipeline attempt

use kernel_frc::invsyn::structural_cert::{
    eval_structured, anti_unify_structured,
    eval_bool_with_trace, anti_unify, validate_schema,
    generate_trace_corpus, emit_certificates,
    extract_existence_certs, anti_unify_exist_certs,
    StructCert, AntiUnifyResult, EvalTrace, TraceCorpus,
};
use kernel_frc::invsyn::eval::mk_env;

use crate::encoding;

/// Run the full verification trace experiment.
/// This is the main entry point called by --structure flag.
pub fn run_kernel_pipeline(instances: &[Vec<i64>]) {
    println!("\n=== VERIFICATION TRACE EXPERIMENT ===");
    println!("Following Goldbach's kernel flow on Subset Sum\n");

    // Use the first instance for the detailed pipeline test
    let weights = &instances[0];
    run_verification_pipeline(weights);

    // Then do multi-instance comparison
    if instances.len() >= 2 {
        run_multi_instance_comparison(instances);
    }
}

/// The core experiment: run ALL kernel pipeline paths on verification-only traces.
fn run_verification_pipeline(weights: &[i64]) {
    let k = weights.len();
    let expr = encoding::build_subset_sum_expr(weights);
    let max_sum = encoding::max_sum(weights);

    println!("════════════════════════════════════════════");
    println!("PIPELINE TEST: weights={:?}, k={}, max_sum={}", weights, k, max_sum);
    println!("════════════════════════════════════════════\n");

    // Step 0: Identify reachable targets (using the kernel itself, not external DP)
    println!("--- Step 0: Identify reachable targets via kernel eval ---");
    let mut reachable: Vec<i64> = Vec::new();
    let mut unreachable: Vec<i64> = Vec::new();
    for t in 0..=max_sum {
        let env = mk_env(t);
        let (val, _) = eval_structured(&env, &expr);
        if val != 0 {
            reachable.push(t);
        } else {
            unreachable.push(t);
        }
    }
    println!("  Total targets: {}", max_sum + 1);
    println!("  Reachable: {} (these become our verification traces)", reachable.len());
    println!("  Unreachable: {}", unreachable.len());

    // ═══════════════════════════════════════════════════════════════════
    // PATH A: Flat traces on verification-only targets
    // (This is what generate_trace_corpus + anti_unify does)
    // ═══════════════════════════════════════════════════════════════════
    println!("\n--- Path A: Flat traces (eval_bool_with_trace) ---");
    println!("  This is Goldbach's generate_trace_corpus path.\n");

    let flat_traces: Vec<EvalTrace> = reachable.iter()
        .map(|&t| eval_bool_with_trace(&expr, t))
        .collect();

    // Show trace lengths to demonstrate the problem
    let mut trace_lengths: std::collections::HashMap<usize, Vec<i64>> =
        std::collections::HashMap::new();
    for (i, trace) in flat_traces.iter().enumerate() {
        trace_lengths.entry(trace.steps.len())
            .or_default()
            .push(reachable[i]);
    }

    println!("  All traces result=true: {}",
        flat_traces.iter().all(|t| t.result));
    println!("  Distinct trace lengths: {}", trace_lengths.len());
    for (len, targets) in &trace_lengths {
        let show: Vec<&i64> = targets.iter().take(3).collect();
        println!("    length={}: {} targets (e.g. {:?}{})",
            len, targets.len(), show,
            if targets.len() > 3 { "..." } else { "" });
    }

    // Try flat anti_unify
    let flat_result = anti_unify(&flat_traces);
    println!("\n  anti_unify (flat): {}",
        if flat_result.is_some() { "SUCCEEDED" } else { "FAILED" });
    if flat_result.is_none() {
        println!("    Reason: ExistsBounded unrolls loop iterations inline,");
        println!("    producing different-length traces for different targets.");
        println!("    Different targets find witnesses at different iteration points.");
    }
    if let Some(ref schema) = flat_result {
        println!("    Schema steps: {}", schema.steps.len());
        println!("    Params: {}", schema.num_params);
        let valid = validate_schema(schema, &flat_traces);
        println!("    Validates: {}", valid);
    }

    // Try building a manual corpus and emit_certificates
    println!("\n  Manual corpus → emit_certificates:");
    let corpus = TraceCorpus {
        problem_id: "subset_sum_verification".to_string(),
        expr: expr.clone(),
        traces: flat_traces,
        all_true: true, // we only included reachable targets
        n_start: 0,
        n_end: max_sum,
    };
    println!("    corpus.all_true = {} (gate passes!)", corpus.all_true);
    let certs = emit_certificates(&corpus);
    println!("    emit_certificates: {}",
        if certs.is_some() { "SUCCEEDED" } else { "FAILED" });
    if certs.is_none() {
        println!("    Reason: anti_unify inside emit_certificates also fails");
        println!("    because flat traces have different lengths.");
    }

    // ═══════════════════════════════════════════════════════════════════
    // Now try the FULL range (including unreachable) to show the all_true gate
    // ═══════════════════════════════════════════════════════════════════
    println!("\n  Compare: full range corpus (reachable + unreachable):");
    let full_corpus = generate_trace_corpus("subset_sum_full", &expr, 0, max_sum);
    println!("    corpus.all_true = {} (gate blocks!)", full_corpus.all_true);
    let full_certs = emit_certificates(&full_corpus);
    println!("    emit_certificates: {}",
        if full_certs.is_some() { "SUCCEEDED" } else { "FAILED" });

    // ═══════════════════════════════════════════════════════════════════
    // PATH B: Tree certs on verification-only targets
    // (This is Goldbach's try_schema_certified path)
    // ═══════════════════════════════════════════════════════════════════
    println!("\n--- Path B: Tree certs (eval_structured + anti_unify_structured) ---");
    println!("  This is Goldbach's try_schema_certified path.\n");

    let tree_certs: Vec<(i64, StructCert)> = reachable.iter()
        .map(|&t| {
            let env = mk_env(t);
            let (_, cert) = eval_structured(&env, &expr);
            (t, cert)
        })
        .collect();

    // Shape analysis
    let shapes: std::collections::HashSet<String> = tree_certs.iter()
        .map(|(_, c)| c.shape())
        .collect();
    println!("  All certs same shape: {} (distinct shapes: {})",
        shapes.len() == 1, shapes.len());
    if let Some(shape) = shapes.iter().next() {
        println!("  Shape: {}", shape);
    }

    // Anti-unify
    let tree_result = anti_unify_structured(&tree_certs);
    println!("\n  anti_unify_structured (tree): {}",
        if tree_result.is_some() { "SUCCEEDED" } else { "FAILED" });

    if let Some(ref au) = tree_result {
        println!("    Schema: {}", au.schema.display());
        println!("    Params: {} (values that vary across targets)", au.num_params);
        println!("    Instances: {} (one per reachable target)", au.instances.len());

        // Show witness table
        let show = std::cmp::min(8, au.instances.len());
        println!("    Witness table (first {}):", show);
        for (n, params) in au.instances.iter().take(show) {
            // Decode the params as item selections
            let selected: Vec<i64> = params.iter()
                .zip(weights.iter())
                .filter_map(|(&b, &w)| if b == 1 { Some(w) } else { None })
                .collect();
            println!("      T={:>3}: bits={:?} → selected={:?} sum={}",
                n, params, selected, selected.iter().sum::<i64>());
        }
        if au.instances.len() > show {
            println!("      ... ({} more)", au.instances.len() - show);
        }
    }

    // ═══════════════════════════════════════════════════════════════════
    // PATH C: Existence certs — obligation-level analysis
    // ═══════════════════════════════════════════════════════════════════
    println!("\n--- Path C: Existence certs (obligation structure) ---");
    println!("  Extracts ExistsWitness nodes and their verification obligations.\n");

    let exist_certs: Vec<_> = tree_certs.iter()
        .flat_map(|(t, cert)| {
            let ec = extract_existence_certs(*t, cert);
            ec
        })
        .collect();

    println!("  Total existence certs extracted: {}", exist_certs.len());
    println!("  Per reachable target: ~{}", exist_certs.len() / reachable.len().max(1));

    // Show a few
    let show = std::cmp::min(5, exist_certs.len());
    for ec in exist_certs.iter().take(show) {
        println!("    n={}, witness={}, lo={}, hi={}, obligations={}",
            ec.n, ec.witness, ec.lo, ec.hi, ec.obligations.len());
        for obl in &ec.obligations {
            println!("      {:?} → {}", ec.witness, obl.result);
        }
    }

    // Try anti-unifying existence certs
    let exist_schema = anti_unify_exist_certs(&exist_certs);
    println!("\n  anti_unify_exist_certs: {}",
        if exist_schema.is_some() { "SUCCEEDED" } else { "FAILED" });
    if let Some(ref schema) = exist_schema {
        println!("    Obligations: {}", schema.num_obligations);
        println!("    Params: {}", schema.num_params);
        println!("    Instances: {}", schema.instances.len());
    }
    if exist_schema.is_none() {
        println!("    Reason: nested ExistsBounded → different obligation counts");
        println!("    per target (k nested levels, but anti_unify_exist_certs");
        println!("    expects uniform obligation structure).");
    }

    // ═══════════════════════════════════════════════════════════════════
    // COMPARISON: Goldbach vs Subset Sum
    // ═══════════════════════════════════════════════════════════════════
    println!("\n════════════════════════════════════════════");
    println!("GOLDBACH vs SUBSET SUM: Pipeline Comparison");
    println!("════════════════════════════════════════════\n");

    println!("                        Goldbach            Subset Sum");
    println!("  ─────────────────────────────────────────────────────────");
    println!("  Trace type:           Verification        Verification*");
    println!("                        (all n true via     (only reachable");
    println!("                         Implies guard)      targets fed)");
    println!("  corpus.all_true:      YES (always)        YES (filtered)");
    println!("  Flat anti_unify:      FAILS               FAILS");
    println!("                        (ExistsBounded      (ExistsBounded");
    println!("                         variable iters)     variable iters)");
    println!("  Tree anti_unify:      SUCCEEDS            SUCCEEDS");
    println!("  expr_to_fn_tag:       fn_tag=1            None");
    println!("                        (recognizes          (pattern not");
    println!("                         And(IsPrime,         recognized)");
    println!("                         IsPrime))                        ");
    println!("  Counting function:    goldbachRepCount    N/A");
    println!("  Density analysis:     YES → unbounded     N/A");
    println!("  Lean proof:           Bounded + ∀n        Bounded only");
    println!();
    println!("  * Key difference: Goldbach naturally covers ALL n.");
    println!("    Subset Sum's reachable set is instance-specific.");
    println!("    No guard can make unreachable targets vacuously true");
    println!("    because the PROBLEM is 'which targets are reachable?'");
    println!();
    println!("  The tree cert schema captures WHAT the brute-force search found:");
    println!("  for each reachable T, which bits (items) were selected.");
    println!("  This is a witness table — structurally, it records brute-force");
    println!("  results rather than discovering a more efficient algorithm.");
    println!();
    println!("  WHY Goldbach goes further:");
    println!("  1. expr_to_fn_tag recognizes ExistsBounded(2,n,And(IsPrime,IsPrime))");
    println!("     and derives a COUNTING function (goldbachRepCount).");
    println!("  2. The counting function reveals DENSITY (how many witnesses exist).");
    println!("  3. Growing density → certified bound → unbounded proof.");
    println!("  4. Subset Sum's ExistsBounded(0,1,...Eq(sum,T)) is not recognized");
    println!("     by expr_to_fn_tag, so no counting function → no density path.");
}

/// Compare schemas across multiple instances with same k.
fn run_multi_instance_comparison(instances: &[Vec<i64>]) {
    println!("\n════════════════════════════════════════════");
    println!("MULTI-INSTANCE COMPARISON");
    println!("════════════════════════════════════════════\n");

    let mut all_schemas: Vec<(usize, String, usize, usize)> = Vec::new();

    for (idx, weights) in instances.iter().enumerate() {
        let expr = encoding::build_subset_sum_expr(weights);
        let max_sum = encoding::max_sum(weights);

        let reachable_certs: Vec<(i64, StructCert)> = (0..=max_sum)
            .filter_map(|t| {
                let env = mk_env(t);
                let (val, cert) = eval_structured(&env, &expr);
                if val != 0 { Some((t, cert)) } else { None }
            })
            .collect();

        if reachable_certs.len() >= 2 {
            if let Some(au) = anti_unify_structured(&reachable_certs) {
                let shape = au.schema.display();
                println!("  Instance {}: k={}, weights={:?}, reachable={}, schema_params={}",
                    idx + 1, weights.len(), weights, reachable_certs.len(), au.num_params);
                println!("    Shape: {}", shape);
                all_schemas.push((idx, shape, au.num_params, reachable_certs.len()));
            }
        }
    }

    let k_values: Vec<usize> = instances.iter().map(|w| w.len()).collect();
    let same_k = k_values.iter().all(|&k| k == k_values[0]);
    if same_k && all_schemas.len() >= 2 {
        let ref_shape = &all_schemas[0].1;
        let all_same = all_schemas.iter().all(|(_, s, _, _)| s == ref_shape);
        let ref_params = all_schemas[0].2;
        let all_same_params = all_schemas.iter().all(|(_, _, p, _)| *p == ref_params);

        println!();
        println!("  All k={}: same shape={}, same params={}", k_values[0], all_same, all_same_params);
        if all_same {
            println!("  STRUCTURAL INVARIANT: schema shape depends only on k, not weights.");
            println!("  This is the kernel's honest observation: the brute-force search tree");
            println!("  has fixed topology determined by k. Only witness values change.");
        }
    }
}
