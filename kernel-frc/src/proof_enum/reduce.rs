//! Core reduction — β-δ-ι evaluation on CoreTerms.
//!
//! This is where the kernel COMPUTES. Every reduction step is deterministic.
//! The normalization trace IS the proof.
//!
//! Reduction rules:
//!   β-reduction: (λ x : A. body) arg → body[x := arg]
//!   δ-unfolding: unfold a defined constant to its value
//!   ι-reduction: recursor applied to constructor → compute
//!   ζ-reduction: let x : A := v in body → body[x := v]
//!   Nat reduction: Nat.succ(n) → NatLit(n+1), Nat.add(a,b) → NatLit(a+b), etc.
//!
//! These are NOT heuristic. They are the only computation rules.
//! Confluence + termination = every term has a unique normal form.
//! The normal form IS the answer. The reduction trace IS the proof.

use super::core_term::{CoreTerm, CoreEnv};
use kernel_types::{Hash32, hash};

/// A single reduction step — part of the proof trace.
#[derive(Debug, Clone)]
pub struct ReductionStep {
    /// What kind of reduction was applied.
    pub kind: ReductionKind,
    /// Hash of the term before reduction.
    pub before_hash: Hash32,
    /// Hash of the term after reduction.
    pub after_hash: Hash32,
}

/// The kind of reduction applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReductionKind {
    /// β: (λ x. body) arg → body[x := arg]
    Beta,
    /// δ: unfold constant definition
    Delta(String),
    /// ι: recursor on constructor → compute
    Iota,
    /// ζ: let x := v in body → body[x := v]
    Zeta,
    /// Nat computation: arithmetic on NatLit values
    NatCompute,
}

/// Result of reduction: the normal form + the trace of steps taken.
#[derive(Debug, Clone)]
pub struct ReductionResult {
    /// The term in normal form (or as far as we got).
    pub normal_form: CoreTerm,
    /// The trace of reduction steps (each step carries a proof).
    pub trace: Vec<ReductionStep>,
    /// Total steps taken.
    pub steps: u64,
    /// Whether we reached a true normal form (no more reductions possible).
    pub is_normal: bool,
}

/// Perform a single reduction step on a CoreTerm.
/// Returns None if the term is already in normal form (head normal form).
pub fn reduce_step(term: &CoreTerm, env: &CoreEnv) -> Option<(CoreTerm, ReductionKind)> {
    match term {
        // β-reduction: (λ x : A. body) arg → body[x := arg]
        CoreTerm::App { func, arg } => {
            if let CoreTerm::Lam { body, .. } = func.as_ref() {
                let reduced = body.substitute(0, arg);
                return Some((reduced, ReductionKind::Beta));
            }

            // Try reducing the function position first
            if let Some((reduced_func, kind)) = reduce_step(func, env) {
                return Some((CoreTerm::App {
                    func: Box::new(reduced_func),
                    arg: arg.clone(),
                }, kind));
            }

            // Try reducing the argument
            if let Some((reduced_arg, kind)) = reduce_step(arg, env) {
                return Some((CoreTerm::App {
                    func: func.clone(),
                    arg: Box::new(reduced_arg),
                }, kind));
            }

            // Check for Nat computation: Nat.add(NatLit(a), NatLit(b)) → NatLit(a+b)
            if let Some(result) = try_nat_compute(term) {
                return Some((result, ReductionKind::NatCompute));
            }

            None
        }

        // ζ-reduction: let x : A := v in body → body[x := v]
        CoreTerm::Let { value, body, .. } => {
            let reduced = body.substitute(0, value);
            Some((reduced, ReductionKind::Zeta))
        }

        // δ-unfolding: replace constant with its definition
        CoreTerm::Const { name, .. } => {
            if let Some(def) = env.lookup(name) {
                if let Some(ref value) = def.value {
                    return Some((value.clone(), ReductionKind::Delta(name.clone())));
                }
            }
            None
        }

        // ι-reduction: recursor applied to constructor
        CoreTerm::Recursor { type_name, args } => {
            try_iota_reduce(type_name, args, env)
                .map(|result| (result, ReductionKind::Iota))
        }

        // Reduce under binders (for full normalization)
        CoreTerm::Lam { param_type, body } => {
            // Try reducing param_type
            if let Some((reduced_pt, kind)) = reduce_step(param_type, env) {
                return Some((CoreTerm::Lam {
                    param_type: Box::new(reduced_pt),
                    body: body.clone(),
                }, kind));
            }
            // Try reducing body
            if let Some((reduced_body, kind)) = reduce_step(body, env) {
                return Some((CoreTerm::Lam {
                    param_type: param_type.clone(),
                    body: Box::new(reduced_body),
                }, kind));
            }
            None
        }

        CoreTerm::Pi { param_type, body } => {
            if let Some((reduced_pt, kind)) = reduce_step(param_type, env) {
                return Some((CoreTerm::Pi {
                    param_type: Box::new(reduced_pt),
                    body: body.clone(),
                }, kind));
            }
            if let Some((reduced_body, kind)) = reduce_step(body, env) {
                return Some((CoreTerm::Pi {
                    param_type: param_type.clone(),
                    body: Box::new(reduced_body),
                }, kind));
            }
            None
        }

        CoreTerm::Constructor { type_name, ctor_name, args } => {
            // Try reducing constructor arguments
            for (i, arg) in args.iter().enumerate() {
                if let Some((reduced_arg, kind)) = reduce_step(arg, env) {
                    let mut new_args = args.clone();
                    new_args[i] = reduced_arg;
                    return Some((CoreTerm::Constructor {
                        type_name: type_name.clone(),
                        ctor_name: ctor_name.clone(),
                        args: new_args,
                    }, kind));
                }
            }
            None
        }

        // Already in normal form
        CoreTerm::Type(_) | CoreTerm::Prop | CoreTerm::Var(_) | CoreTerm::NatLit(_) => None,
    }
}

/// Reduce a term to normal form, recording the trace.
/// Stops after max_steps to prevent infinite loops during development.
pub fn reduce(term: &CoreTerm, env: &CoreEnv, max_steps: u64) -> ReductionResult {
    let mut current = term.clone();
    let mut trace = Vec::new();
    let mut steps = 0u64;

    while steps < max_steps {
        match reduce_step(&current, env) {
            Some((reduced, kind)) => {
                let step = ReductionStep {
                    kind,
                    before_hash: current.term_hash(),
                    after_hash: reduced.term_hash(),
                };
                trace.push(step);
                current = reduced;
                steps += 1;
            }
            None => {
                // No more reductions — we've reached normal form
                return ReductionResult {
                    normal_form: current,
                    trace,
                    steps,
                    is_normal: true,
                };
            }
        }
    }

    // Budget exhausted
    ReductionResult {
        normal_form: current,
        trace,
        steps,
        is_normal: false,
    }
}

/// Try ι-reduction: Nat.rec applied to Nat.zero or Nat.succ.
fn try_iota_reduce(type_name: &str, args: &[CoreTerm], _env: &CoreEnv) -> Option<CoreTerm> {
    if type_name != "Nat" || args.len() < 3 {
        return None;
    }

    // Nat.rec motive zero_case succ_case n
    // If n = NatLit(0) → zero_case
    // If n = NatLit(k+1) → succ_case k (Nat.rec motive zero_case succ_case k)
    let motive = &args[0];
    let zero_case = &args[1];
    let succ_case = &args[2];

    if args.len() < 4 {
        return None;
    }
    let target = &args[3];

    match target {
        CoreTerm::NatLit(0) => Some(zero_case.clone()),
        CoreTerm::NatLit(n) if *n > 0 => {
            // succ_case (n-1) (rec motive zero succ (n-1))
            let pred = CoreTerm::NatLit(n - 1);
            let recursive_call = CoreTerm::Recursor {
                type_name: "Nat".into(),
                args: vec![motive.clone(), zero_case.clone(), succ_case.clone(), pred.clone()],
            };
            Some(CoreTerm::App {
                func: Box::new(CoreTerm::App {
                    func: Box::new(succ_case.clone()),
                    arg: Box::new(pred),
                }),
                arg: Box::new(recursive_call),
            })
        }
        CoreTerm::Constructor { type_name: tn, ctor_name, args: ctor_args }
            if tn == "Nat" && ctor_name == "zero" && ctor_args.is_empty() =>
        {
            Some(zero_case.clone())
        }
        CoreTerm::Constructor { type_name: tn, ctor_name, args: ctor_args }
            if tn == "Nat" && ctor_name == "succ" && ctor_args.len() == 1 =>
        {
            let pred = &ctor_args[0];
            let recursive_call = CoreTerm::Recursor {
                type_name: "Nat".into(),
                args: vec![motive.clone(), zero_case.clone(), succ_case.clone(), pred.clone()],
            };
            Some(CoreTerm::App {
                func: Box::new(CoreTerm::App {
                    func: Box::new(succ_case.clone()),
                    arg: Box::new(pred.clone()),
                }),
                arg: Box::new(recursive_call),
            })
        }
        _ => None,
    }
}

/// Try to compute with natural number operations.
/// Nat.add(a, b) → a + b, Nat.mul(a, b) → a * b, etc.
fn try_nat_compute(term: &CoreTerm) -> Option<CoreTerm> {
    // Pattern: App(App(Const("Nat.add"), NatLit(a)), NatLit(b)) → NatLit(a+b)
    if let CoreTerm::App { func, arg } = term {
        if let CoreTerm::NatLit(b) = arg.as_ref() {
            if let CoreTerm::App { func: inner_func, arg: inner_arg } = func.as_ref() {
                if let CoreTerm::NatLit(a) = inner_arg.as_ref() {
                    if let CoreTerm::Const { name, .. } = inner_func.as_ref() {
                        match name.as_str() {
                            "Nat.add" => return Some(CoreTerm::NatLit(a + b)),
                            "Nat.mul" => return Some(CoreTerm::NatLit(a * b)),
                            "Nat.sub" => return Some(CoreTerm::NatLit(a.saturating_sub(*b))),
                            "Nat.mod" => {
                                if *b > 0 {
                                    return Some(CoreTerm::NatLit(a % b));
                                }
                            }
                            "Nat.div" => {
                                if *b > 0 {
                                    return Some(CoreTerm::NatLit(a / b));
                                }
                            }
                            "Nat.ble" => {
                                return Some(CoreTerm::Constructor {
                                    type_name: "Bool".into(),
                                    ctor_name: if a <= b { "true" } else { "false" }.into(),
                                    args: vec![],
                                });
                            }
                            "Nat.beq" => {
                                return Some(CoreTerm::Constructor {
                                    type_name: "Bool".into(),
                                    ctor_name: if a == b { "true" } else { "false" }.into(),
                                    args: vec![],
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::core_term::CoreDef;

    fn nat_const() -> CoreTerm {
        CoreTerm::Const { name: "Nat".into(), levels: vec![] }
    }

    #[test]
    fn beta_reduction() {
        // (λ Nat. Var(0)) 42 → 42
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::Lam {
                param_type: Box::new(nat_const()),
                body: Box::new(CoreTerm::Var(0)),
            }),
            arg: Box::new(CoreTerm::NatLit(42)),
        };

        let env = CoreEnv::new();
        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::Beta);
        assert_eq!(reduced, CoreTerm::NatLit(42));
    }

    #[test]
    fn zeta_reduction() {
        // let x : Nat := 5 in Var(0) → 5
        let term = CoreTerm::Let {
            bound_type: Box::new(nat_const()),
            value: Box::new(CoreTerm::NatLit(5)),
            body: Box::new(CoreTerm::Var(0)),
        };

        let env = CoreEnv::new();
        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::Zeta);
        assert_eq!(reduced, CoreTerm::NatLit(5));
    }

    #[test]
    fn delta_reduction() {
        // Const("myval") with env[myval := 99] → 99
        let term = CoreTerm::Const { name: "myval".into(), levels: vec![] };

        let mut env = CoreEnv::new();
        env.add_def(CoreDef {
            name: "myval".into(),
            ty: nat_const(),
            value: Some(CoreTerm::NatLit(99)),
            universe_params: vec![],
        });

        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::Delta("myval".into()));
        assert_eq!(reduced, CoreTerm::NatLit(99));
    }

    #[test]
    fn nat_add() {
        // Nat.add 3 4 → 7
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "Nat.add".into(), levels: vec![] }),
                arg: Box::new(CoreTerm::NatLit(3)),
            }),
            arg: Box::new(CoreTerm::NatLit(4)),
        };

        let env = CoreEnv::new();
        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::NatCompute);
        assert_eq!(reduced, CoreTerm::NatLit(7));
    }

    #[test]
    fn nat_mul() {
        // Nat.mul 6 7 → 42
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "Nat.mul".into(), levels: vec![] }),
                arg: Box::new(CoreTerm::NatLit(6)),
            }),
            arg: Box::new(CoreTerm::NatLit(7)),
        };

        let env = CoreEnv::new();
        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::NatCompute);
        assert_eq!(reduced, CoreTerm::NatLit(42));
    }

    #[test]
    fn nat_beq() {
        // Nat.beq 5 5 → Bool.true
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Const { name: "Nat.beq".into(), levels: vec![] }),
                arg: Box::new(CoreTerm::NatLit(5)),
            }),
            arg: Box::new(CoreTerm::NatLit(5)),
        };

        let env = CoreEnv::new();
        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::NatCompute);
        assert_eq!(reduced, CoreTerm::Constructor {
            type_name: "Bool".into(),
            ctor_name: "true".into(),
            args: vec![],
        });
    }

    #[test]
    fn iota_nat_zero() {
        // Nat.rec motive zero_case succ_case 0 → zero_case
        let term = CoreTerm::Recursor {
            type_name: "Nat".into(),
            args: vec![
                CoreTerm::Prop,           // motive
                CoreTerm::NatLit(100),    // zero_case
                CoreTerm::Var(0),         // succ_case
                CoreTerm::NatLit(0),      // target = 0
            ],
        };

        let env = CoreEnv::new();
        let (reduced, kind) = reduce_step(&term, &env).unwrap();
        assert_eq!(kind, ReductionKind::Iota);
        assert_eq!(reduced, CoreTerm::NatLit(100));
    }

    #[test]
    fn full_reduction_to_normal_form() {
        // (λ Nat. Nat.add Var(0) 10) 32 → 42
        let term = CoreTerm::App {
            func: Box::new(CoreTerm::Lam {
                param_type: Box::new(nat_const()),
                body: Box::new(CoreTerm::App {
                    func: Box::new(CoreTerm::App {
                        func: Box::new(CoreTerm::Const { name: "Nat.add".into(), levels: vec![] }),
                        arg: Box::new(CoreTerm::Var(0)),
                    }),
                    arg: Box::new(CoreTerm::NatLit(10)),
                }),
            }),
            arg: Box::new(CoreTerm::NatLit(32)),
        };

        let env = CoreEnv::new();
        let result = reduce(&term, &env, 100);
        assert!(result.is_normal);
        assert_eq!(result.normal_form, CoreTerm::NatLit(42));
        assert_eq!(result.steps, 2); // β then NatCompute
        assert_eq!(result.trace.len(), 2);
        assert_eq!(result.trace[0].kind, ReductionKind::Beta);
        assert_eq!(result.trace[1].kind, ReductionKind::NatCompute);
    }

    #[test]
    fn reduction_trace_has_hashes() {
        let term = CoreTerm::Let {
            bound_type: Box::new(nat_const()),
            value: Box::new(CoreTerm::NatLit(5)),
            body: Box::new(CoreTerm::Var(0)),
        };

        let env = CoreEnv::new();
        let result = reduce(&term, &env, 10);
        assert_eq!(result.steps, 1);
        assert_eq!(result.trace[0].kind, ReductionKind::Zeta);
        // Before and after hashes should differ
        assert_ne!(result.trace[0].before_hash, result.trace[0].after_hash);
        // After hash should match the normal form's hash
        assert_eq!(result.trace[0].after_hash, result.normal_form.term_hash());
    }

    #[test]
    fn already_normal() {
        let term = CoreTerm::NatLit(42);
        let env = CoreEnv::new();
        let result = reduce(&term, &env, 100);
        assert!(result.is_normal);
        assert_eq!(result.steps, 0);
        assert!(result.trace.is_empty());
        assert_eq!(result.normal_form, CoreTerm::NatLit(42));
    }

    #[test]
    fn budget_exhaustion() {
        // Create a term that reduces many times
        // let x := 0 in let y := x in let z := y in z
        let term = CoreTerm::Let {
            bound_type: Box::new(nat_const()),
            value: Box::new(CoreTerm::NatLit(0)),
            body: Box::new(CoreTerm::Let {
                bound_type: Box::new(nat_const()),
                value: Box::new(CoreTerm::Var(0)),
                body: Box::new(CoreTerm::Let {
                    bound_type: Box::new(nat_const()),
                    value: Box::new(CoreTerm::Var(0)),
                    body: Box::new(CoreTerm::Var(0)),
                }),
            }),
        };

        // With budget=1, should stop after 1 step
        let env = CoreEnv::new();
        let result = reduce(&term, &env, 1);
        assert!(!result.is_normal);
        assert_eq!(result.steps, 1);

        // With budget=100, should reach normal form
        let result_full = reduce(&term, &env, 100);
        assert!(result_full.is_normal);
        assert_eq!(result_full.normal_form, CoreTerm::NatLit(0));
    }
}
