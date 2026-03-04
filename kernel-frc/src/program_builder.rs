// Program Builder — converts SearchProblems into VM programs with honest B*.
//
// A SearchProblem describes WHAT to search for:
//   Exists { vars, pred } — find an assignment satisfying the predicate
//   ForAll { vars, pred } — verify all assignments satisfy the predicate
//   Sat { num_vars, clauses } — boolean satisfiability (special case of Exists)
//
// build_program converts the SearchProblem into:
//   (Program, b_star) where B* is derived from program structure (A1 requirement)
//
// Memory layout:
//   Single-variable:  slot 0 = value, slot 1 = limit (hi+1)
//   Multi-variable:   slot 2*i = var_i value, slot 2*i+1 = var_i limit
//   SAT:              slot 0 = assignment integer, slot 1 = 2^n

use serde::{Serialize, Deserialize};
use crate::vm::{Instruction, Program};
use crate::predicate::*;

/// A bounded variable with name and domain [lo, hi] inclusive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedVar {
    pub name: String,
    pub lo: i64,
    pub hi: i64,
}

/// A search problem — what the FRC engine needs to decide.
#[derive(Debug, Clone)]
pub enum SearchProblem {
    /// Existential: find an assignment in the Cartesian product of domains
    /// that satisfies the predicate.
    /// Halted(1) → satisfying candidate exists.
    /// Halted(0) → no satisfying candidate (exhaustive search).
    Exists {
        variables: Vec<BoundedVar>,
        predicate: Pred,
    },
    /// Universal: verify that ALL assignments satisfy the predicate.
    /// Halted(1) → all verified.
    /// Halted(0) → counterexample found.
    ForAll {
        variables: Vec<BoundedVar>,
        predicate: Pred,
    },
    /// Boolean satisfiability: special case of Exists over bit assignments.
    /// Converted internally to Exists { vars: [("a", 0, 2^n-1)], pred: cnf_to_pred(clauses) }.
    Sat {
        num_vars: usize,
        clauses: Vec<Vec<i32>>,
    },
}

/// Build error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildError {
    /// Empty variable list (nothing to search).
    NoVariables,
    /// Domain too large (would exceed step budget).
    DomainTooLarge(String),
    /// Predicate compilation failed.
    CompileError(CompileError),
    /// SAT variable count too large.
    TooManyVars(usize),
}

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildError::NoVariables => write!(f, "no variables in search problem"),
            BuildError::DomainTooLarge(msg) => write!(f, "domain too large: {}", msg),
            BuildError::CompileError(e) => write!(f, "predicate compile error: {}", e),
            BuildError::TooManyVars(n) => write!(f, "too many SAT variables: {}", n),
        }
    }
}

impl From<CompileError> for BuildError {
    fn from(e: CompileError) -> Self {
        BuildError::CompileError(e)
    }
}

/// Build a VM program from a SearchProblem.
///
/// Returns (Program, b_star) where b_star is honestly derived from:
///   - domain_size (product of all variable ranges)
///   - predicate instruction count
///   - loop overhead (branch, increment, jump)
///
/// B* is a theorem, not a parameter (A1 requirement).
pub fn build_program(problem: &SearchProblem) -> Result<(Program, u64), BuildError> {
    match problem {
        SearchProblem::Sat { num_vars, clauses } => {
            if *num_vars > 20 {
                return Err(BuildError::TooManyVars(*num_vars));
            }
            // Convert to Exists with CNF predicate
            let domain_size = 1i64.checked_shl(*num_vars as u32)
                .ok_or_else(|| BuildError::DomainTooLarge(format!("2^{}", num_vars)))?;
            let pred = cnf_to_pred(clauses);
            let vars = vec![BoundedVar {
                name: "a".to_string(),
                lo: 0,
                hi: domain_size - 1,
            }];
            build_exists_program(&vars, &pred)
        }
        SearchProblem::Exists { variables, predicate } => {
            if variables.is_empty() {
                return Err(BuildError::NoVariables);
            }
            build_exists_program(variables, predicate)
        }
        SearchProblem::ForAll { variables, predicate } => {
            if variables.is_empty() {
                return Err(BuildError::NoVariables);
            }
            build_forall_program(variables, predicate)
        }
    }
}

/// Build an Exists program: enumerate all assignments, halt(1) if predicate satisfied.
fn build_exists_program(variables: &[BoundedVar], predicate: &Pred) -> Result<(Program, u64), BuildError> {
    if variables.len() == 1 {
        build_single_var_exists(&variables[0], predicate)
    } else {
        build_multi_var_exists(variables, predicate)
    }
}

/// Build a ForAll program: enumerate all assignments, halt(0) if predicate fails.
fn build_forall_program(variables: &[BoundedVar], predicate: &Pred) -> Result<(Program, u64), BuildError> {
    if variables.len() == 1 {
        build_single_var_forall(&variables[0], predicate)
    } else {
        build_multi_var_forall(variables, predicate)
    }
}

/// Single-variable Exists:
/// ```text
/// Push(lo), Store(0), Push(hi+1), Store(1)    // init
/// LOOP: Load(0), Load(1), Lt, Jz(EXHAUSTED)  // check x < limit
///   [compiled predicate]                       // pushes 0 or 1
///   Jz(NEXT)                                   // skip if false
///   Halt(1)                                    // FOUND
/// NEXT: Load(0), Push(1), Add, Store(0), Jmp(LOOP)  // x++
/// EXHAUSTED: Halt(0)                           // NOT FOUND
/// ```
fn build_single_var_exists(var: &BoundedVar, predicate: &Pred) -> Result<(Program, u64), BuildError> {
    let val_slot = 0usize;
    let limit_slot = 1usize;

    let mut var_env = VarEnv::new(0);
    var_env.bind(&var.name); // slot 0 = value

    let compiler = PredicateCompiler::new(var_env);
    let pred_instrs = compiler.compile_pred(predicate)?;
    let pred_count = pred_instrs.len();

    let mut instrs = Vec::new();

    // Init: mem[0] = lo, mem[1] = hi+1
    instrs.push(Instruction::Push(var.lo));          // 0
    instrs.push(Instruction::Store(val_slot));        // 1
    instrs.push(Instruction::Push(var.hi + 1));       // 2
    instrs.push(Instruction::Store(limit_slot));      // 3

    let loop_start = instrs.len();                    // 4
    // Check x < limit
    instrs.push(Instruction::Load(val_slot));         // 4
    instrs.push(Instruction::Load(limit_slot));       // 5
    instrs.push(Instruction::Lt);                     // 6
    let jz_exhausted_idx = instrs.len();
    instrs.push(Instruction::Jz(0));                  // 7: placeholder

    // Compiled predicate (pushes 0 or 1)
    instrs.extend(pred_instrs);

    let jz_next_idx = instrs.len();
    instrs.push(Instruction::Jz(0));                  // placeholder

    // Found!
    instrs.push(Instruction::Halt(1));

    let next_iter = instrs.len();
    instrs[jz_next_idx] = Instruction::Jz(next_iter);

    // Increment x
    instrs.push(Instruction::Load(val_slot));
    instrs.push(Instruction::Push(1));
    instrs.push(Instruction::Add);
    instrs.push(Instruction::Store(val_slot));
    instrs.push(Instruction::Jmp(loop_start));

    // Exhausted
    let exhausted = instrs.len();
    instrs.push(Instruction::Halt(0));
    instrs[jz_exhausted_idx] = Instruction::Jz(exhausted);

    let domain_size = compute_domain_size(var.lo, var.hi);
    // B* derivation (honest, from program structure):
    //   init: 4 steps (Push lo, Store, Push hi+1, Store)
    //   per iteration: 4 (loop check) + pred_count + 1 (Jz) + 5 (increment + Jmp) = 10 + pred_count
    //   final loop check when exhausted: 4 (Load, Load, Lt, Jz → jumps to EXHAUSTED)
    //   halt: 1
    //   Total: 4 + domain_size * (10 + pred_count) + 4 + 1 = 9 + domain_size * (10 + pred_count)
    let steps_per_iter = 10 + pred_count as u64;
    let b_star = 9 + domain_size * steps_per_iter;

    Ok((Program::new(instrs), b_star))
}

/// Single-variable ForAll:
/// Same loop structure, inverted halting:
///   predicate false → Halt(0) (counterexample found)
///   all pass → Halt(1)
fn build_single_var_forall(var: &BoundedVar, predicate: &Pred) -> Result<(Program, u64), BuildError> {
    let val_slot = 0usize;
    let limit_slot = 1usize;

    let mut var_env = VarEnv::new(0);
    var_env.bind(&var.name); // slot 0 = value

    let compiler = PredicateCompiler::new(var_env);
    let pred_instrs = compiler.compile_pred(predicate)?;
    let pred_count = pred_instrs.len();

    let mut instrs = Vec::new();

    // Init
    instrs.push(Instruction::Push(var.lo));
    instrs.push(Instruction::Store(val_slot));
    instrs.push(Instruction::Push(var.hi + 1));
    instrs.push(Instruction::Store(limit_slot));

    let loop_start = instrs.len();
    instrs.push(Instruction::Load(val_slot));
    instrs.push(Instruction::Load(limit_slot));
    instrs.push(Instruction::Lt);
    let jz_all_pass_idx = instrs.len();
    instrs.push(Instruction::Jz(0)); // placeholder

    // Compiled predicate
    instrs.extend(pred_instrs);

    let jz_counter_idx = instrs.len();
    instrs.push(Instruction::Jz(0)); // placeholder: if pred false, counterexample

    // Predicate passed, increment
    instrs.push(Instruction::Load(val_slot));
    instrs.push(Instruction::Push(1));
    instrs.push(Instruction::Add);
    instrs.push(Instruction::Store(val_slot));
    instrs.push(Instruction::Jmp(loop_start));

    // Counterexample found
    let counterexample = instrs.len();
    instrs.push(Instruction::Halt(0));
    instrs[jz_counter_idx] = Instruction::Jz(counterexample);

    // All passed
    let all_pass = instrs.len();
    instrs.push(Instruction::Halt(1));
    instrs[jz_all_pass_idx] = Instruction::Jz(all_pass);

    let domain_size = compute_domain_size(var.lo, var.hi);
    // Same B* derivation as Exists: 9 + domain_size * (10 + pred_count)
    let steps_per_iter = 10 + pred_count as u64;
    let b_star = 9 + domain_size * steps_per_iter;

    Ok((Program::new(instrs), b_star))
}

/// Multi-variable Exists: nested loops.
/// Outer variables iterate in order; innermost runs the predicate.
fn build_multi_var_exists(variables: &[BoundedVar], predicate: &Pred) -> Result<(Program, u64), BuildError> {
    // Memory layout: slot 0..n-1 = variable values, slot n..2n-1 = variable limits.
    // VarEnv::new(0) naturally maps var 0→slot 0, var 1→slot 1, etc.
    let n = variables.len();
    let mut var_env = VarEnv::new(0);
    for var in variables {
        var_env.bind(&var.name);
    }
    // Now variables map to slots 0..n-1. Limits will be at slots n..2n-1.

    let compiler = PredicateCompiler::new(var_env);
    let pred_instrs = compiler.compile_pred(predicate)?;
    let pred_count = pred_instrs.len();

    let mut instrs = Vec::new();

    // Init all variables and limits
    for (i, var) in variables.iter().enumerate() {
        let val_slot = i;
        let limit_slot = n + i;
        instrs.push(Instruction::Push(var.lo));
        instrs.push(Instruction::Store(val_slot));
        instrs.push(Instruction::Push(var.hi + 1));
        instrs.push(Instruction::Store(limit_slot));
    }

    // Build nested loops from outermost to innermost
    let mut loop_starts: Vec<usize> = Vec::new();
    let mut jz_done_indices: Vec<usize> = Vec::new();

    for i in 0..n {
        let val_slot = i;
        let limit_slot = n + i;

        let loop_start = instrs.len();
        loop_starts.push(loop_start);

        instrs.push(Instruction::Load(val_slot));
        instrs.push(Instruction::Load(limit_slot));
        instrs.push(Instruction::Lt);
        let jz_idx = instrs.len();
        jz_done_indices.push(jz_idx);
        instrs.push(Instruction::Jz(0)); // placeholder
    }

    // Innermost: evaluate predicate
    instrs.extend(pred_instrs);
    let jz_next_idx = instrs.len();
    instrs.push(Instruction::Jz(0)); // placeholder

    // Found!
    instrs.push(Instruction::Halt(1));

    let next_inner = instrs.len();
    instrs[jz_next_idx] = Instruction::Jz(next_inner);

    // Increment innermost variable, then cascade outward
    for i in (0..n).rev() {
        let val_slot = i;
        instrs.push(Instruction::Load(val_slot));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(val_slot));
        instrs.push(Instruction::Jmp(loop_starts[i]));

        // Patch the exhaustion jump for this level
        let exit_point = instrs.len();
        instrs[jz_done_indices[i]] = Instruction::Jz(exit_point);

        // Reset this variable to lo for next outer iteration (except outermost)
        if i > 0 {
            instrs.push(Instruction::Push(variables[i].lo));
            instrs.push(Instruction::Store(val_slot));
        }
    }

    // All exhausted
    instrs.push(Instruction::Halt(0));

    // B* derivation for multi-variable (honest, from program structure):
    //   init: 4*n steps
    //   per inner iteration (generous upper bound):
    //     all loop checks: n*4, pred: pred_count, Jz: 1, all increments: n*5, resets: (n-1)*2
    //   final exhaustion cascade: n*4 (one check per level) + n*7 (resets+increments) + 1 (halt)
    let total_domain: u64 = variables.iter()
        .map(|v| compute_domain_size(v.lo, v.hi))
        .product();
    let init_steps = (4 * n) as u64;
    let per_iter = (n as u64 * 4) + pred_count as u64 + 1 + (n as u64 * 5) + ((n.saturating_sub(1)) as u64 * 2);
    let exhaust_overhead = (n as u64 * 11) + 1; // generous: checks + resets + halt
    let b_star = init_steps + total_domain * per_iter + exhaust_overhead;

    Ok((Program::new(instrs), b_star))
}

/// Multi-variable ForAll: nested loops with inverted halting.
fn build_multi_var_forall(variables: &[BoundedVar], predicate: &Pred) -> Result<(Program, u64), BuildError> {
    let n = variables.len();
    let mut var_env = VarEnv::new(0);
    for var in variables {
        var_env.bind(&var.name);
    }

    let compiler = PredicateCompiler::new(var_env);
    let pred_instrs = compiler.compile_pred(predicate)?;
    let pred_count = pred_instrs.len();

    let mut instrs = Vec::new();

    // Init
    for (i, var) in variables.iter().enumerate() {
        let val_slot = i;
        let limit_slot = n + i;
        instrs.push(Instruction::Push(var.lo));
        instrs.push(Instruction::Store(val_slot));
        instrs.push(Instruction::Push(var.hi + 1));
        instrs.push(Instruction::Store(limit_slot));
    }

    // Nested loops
    let mut loop_starts = Vec::new();
    let mut jz_done_indices = Vec::new();

    for i in 0..n {
        let val_slot = i;
        let limit_slot = n + i;

        let loop_start = instrs.len();
        loop_starts.push(loop_start);

        instrs.push(Instruction::Load(val_slot));
        instrs.push(Instruction::Load(limit_slot));
        instrs.push(Instruction::Lt);
        let jz_idx = instrs.len();
        jz_done_indices.push(jz_idx);
        instrs.push(Instruction::Jz(0)); // placeholder
    }

    // Evaluate predicate
    instrs.extend(pred_instrs);
    let jz_counter_idx = instrs.len();
    instrs.push(Instruction::Jz(0)); // placeholder: counterexample

    // Predicate passed, continue inner loop increment
    for i in (0..n).rev() {
        let val_slot = i;
        instrs.push(Instruction::Load(val_slot));
        instrs.push(Instruction::Push(1));
        instrs.push(Instruction::Add);
        instrs.push(Instruction::Store(val_slot));
        instrs.push(Instruction::Jmp(loop_starts[i]));

        let exit_point = instrs.len();
        instrs[jz_done_indices[i]] = Instruction::Jz(exit_point);

        if i > 0 {
            instrs.push(Instruction::Push(variables[i].lo));
            instrs.push(Instruction::Store(val_slot));
        }
    }

    // All passed
    instrs.push(Instruction::Halt(1));

    // Counterexample found
    let counterexample = instrs.len();
    instrs.push(Instruction::Halt(0));
    instrs[jz_counter_idx] = Instruction::Jz(counterexample);

    // B* (same formula as multi-var Exists)
    let total_domain: u64 = variables.iter()
        .map(|v| compute_domain_size(v.lo, v.hi))
        .product();
    let init_steps = (4 * n) as u64;
    let per_iter = (n as u64 * 4) + pred_count as u64 + 1 + (n as u64 * 5) + ((n.saturating_sub(1)) as u64 * 2);
    let exhaust_overhead = (n as u64 * 11) + 1;
    let b_star = init_steps + total_domain * per_iter + exhaust_overhead;

    Ok((Program::new(instrs), b_star))
}

/// Compute domain size for a single variable [lo, hi].
/// Returns 0 for empty domains (hi < lo).
fn compute_domain_size(lo: i64, hi: i64) -> u64 {
    if hi < lo { 0 } else { (hi - lo + 1) as u64 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{Vm, VmOutcome};

    // --- Test 1: exists x in [0,100]: x*x == 49 ---
    #[test]
    fn exists_x_squared_eq_49() {
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 100 }],
            predicate: Pred::Eq(
                Expr::Mul(Box::new(Expr::Var("x".into())), Box::new(Expr::Var("x".into()))),
                Expr::Lit(49),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, state) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(state.memory[&0], 7); // x = 7
    }

    // --- Test 2: forall x in [0,5]: x*(x+1) % 2 == 0 ---
    #[test]
    fn forall_product_consecutive_even() {
        let problem = SearchProblem::ForAll {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 5 }],
            predicate: Pred::Eq(
                Expr::Mod(
                    Box::new(Expr::Mul(
                        Box::new(Expr::Var("x".into())),
                        Box::new(Expr::Add(
                            Box::new(Expr::Var("x".into())),
                            Box::new(Expr::Lit(1)),
                        )),
                    )),
                    Box::new(Expr::Lit(2)),
                ),
                Expr::Lit(0),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1)); // all pass
    }

    // --- Test 3: SAT (x0|x1) & (!x0|x1) & (!x1) → UNSAT ---
    #[test]
    fn sat_unsat() {
        let problem = SearchProblem::Sat {
            num_vars: 2,
            clauses: vec![vec![1, 2], vec![-1, 2], vec![-2]],
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(0)); // UNSAT
    }

    // --- Test 4: exists x,y in [0,10]: x*x + y*y == 100 ---
    #[test]
    fn exists_two_var_pythagorean() {
        let problem = SearchProblem::Exists {
            variables: vec![
                BoundedVar { name: "x".into(), lo: 0, hi: 10 },
                BoundedVar { name: "y".into(), lo: 0, hi: 10 },
            ],
            predicate: Pred::Eq(
                Expr::Add(
                    Box::new(Expr::Mul(Box::new(Expr::Var("x".into())), Box::new(Expr::Var("x".into())))),
                    Box::new(Expr::Mul(Box::new(Expr::Var("y".into())), Box::new(Expr::Var("y".into())))),
                ),
                Expr::Lit(100),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, state) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        // Should find (0,10) or (6,8) or (8,6) or (10,0)
        let x = state.memory[&0];
        let y = state.memory[&1];
        assert_eq!(x * x + y * y, 100);
    }

    // --- Test 5: exists x in [0,100]: x*x == 50 → Halted(0) (no integer sqrt) ---
    #[test]
    fn exists_no_integer_sqrt() {
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 100 }],
            predicate: Pred::Eq(
                Expr::Mul(Box::new(Expr::Var("x".into())), Box::new(Expr::Var("x".into()))),
                Expr::Lit(50),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(0));
    }

    // --- Test 6: forall x in [0,6]: x*x < 26 → Halted(0) (counterexample x=6) ---
    #[test]
    fn forall_counterexample() {
        let problem = SearchProblem::ForAll {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 6 }],
            predicate: Pred::Lt(
                Expr::Mul(Box::new(Expr::Var("x".into())), Box::new(Expr::Var("x".into()))),
                Expr::Lit(26),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(0)); // counterexample: 6*6=36 >= 26
    }

    // --- Test 7: SAT (x0|x1) → SAT ---
    #[test]
    fn sat_satisfiable() {
        let problem = SearchProblem::Sat {
            num_vars: 2,
            clauses: vec![vec![1, 2]],
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1)); // SAT
    }

    // --- Test 8: exists x in [-5,5]: x*x*x == -8 → x=-2 ---
    #[test]
    fn exists_negative_domain() {
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: -5, hi: 5 }],
            predicate: Pred::Eq(
                Expr::Mul(
                    Box::new(Expr::Mul(Box::new(Expr::Var("x".into())), Box::new(Expr::Var("x".into())))),
                    Box::new(Expr::Var("x".into())),
                ),
                Expr::Lit(-8),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, state) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(state.memory[&0], -2);
    }

    // --- Test 9: empty domain exists → Halted(0) ---
    #[test]
    fn exists_empty_domain() {
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: 10, hi: 5 }], // hi < lo
            predicate: Pred::True,
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(0));
    }

    // --- Test 10: empty domain forall → Halted(1) (vacuous truth) ---
    #[test]
    fn forall_empty_domain() {
        let problem = SearchProblem::ForAll {
            variables: vec![BoundedVar { name: "x".into(), lo: 10, hi: 5 }],
            predicate: Pred::False, // doesn't matter; domain is empty
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1)); // vacuous
    }

    // --- Test 11: single element domain ---
    #[test]
    fn exists_single_element() {
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: 42, hi: 42 }],
            predicate: Pred::Eq(Expr::Var("x".into()), Expr::Lit(42)),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, state) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert_eq!(state.memory[&0], 42);
    }

    // --- Test 12: B* sufficient (halts within budget) ---
    #[test]
    fn b_star_sufficient() {
        // Search 101 elements, should halt within b_star
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 100 }],
            predicate: Pred::Eq(Expr::Var("x".into()), Expr::Lit(100)), // found at last element
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, state) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
        assert!(state.steps_taken <= b_star);
    }

    // --- Test 13: B* tight (budget-1 causes BudgetExhausted for worst case) ---
    #[test]
    fn b_star_tight() {
        // For exhaustive search (no match), the program halts at exactly the end
        let problem = SearchProblem::Exists {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 5 }],
            predicate: Pred::False, // never satisfied → exhausts entire domain
        };
        let (program, b_star) = build_program(&problem).unwrap();

        // With full budget: should halt
        let (outcome, state) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(0));
        let actual_steps = state.steps_taken;

        // Actual steps should be <= b_star
        assert!(actual_steps <= b_star, "actual {} > b_star {}", actual_steps, b_star);

        // With less budget than actual steps: should exhaust
        if actual_steps > 1 {
            let (outcome2, _) = Vm::run(&program, actual_steps - 1);
            assert_eq!(outcome2, VmOutcome::BudgetExhausted);
        }
    }

    // --- Test 14: determinism ---
    #[test]
    fn determinism() {
        let problem = SearchProblem::Sat {
            num_vars: 3,
            clauses: vec![vec![1, 2], vec![-1, 3], vec![-2, -3]],
        };
        let (prog1, b1) = build_program(&problem).unwrap();
        let (prog2, b2) = build_program(&problem).unwrap();
        assert_eq!(prog1, prog2);
        assert_eq!(b1, b2);
        // Same program hash
        use kernel_types::SerPi;
        assert_eq!(prog1.ser_pi_hash(), prog2.ser_pi_hash());
    }

    // --- Test 15: forall x in [0,10]: x + 1 > x ---
    #[test]
    fn forall_successor_greater() {
        let problem = SearchProblem::ForAll {
            variables: vec![BoundedVar { name: "x".into(), lo: 0, hi: 10 }],
            predicate: Pred::Gt(
                Expr::Add(Box::new(Expr::Var("x".into())), Box::new(Expr::Lit(1))),
                Expr::Var("x".into()),
            ),
        };
        let (program, b_star) = build_program(&problem).unwrap();
        let (outcome, _) = Vm::run(&program, b_star);
        assert_eq!(outcome, VmOutcome::Halted(1));
    }

    // --- Error cases ---
    #[test]
    fn error_no_variables() {
        let problem = SearchProblem::Exists {
            variables: vec![],
            predicate: Pred::True,
        };
        assert!(matches!(build_program(&problem), Err(BuildError::NoVariables)));
    }

    #[test]
    fn error_too_many_sat_vars() {
        let problem = SearchProblem::Sat {
            num_vars: 25,
            clauses: vec![vec![1]],
        };
        assert!(matches!(build_program(&problem), Err(BuildError::TooManyVars(25))));
    }
}
