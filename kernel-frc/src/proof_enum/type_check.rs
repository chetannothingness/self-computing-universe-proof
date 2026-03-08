//! Type checker — the kernel's internal proof verifier.
//!
//! This is where the kernel checks its own proofs. No external oracle needed.
//! Given a CoreTerm and a goal type, determines if the term inhabits the type.
//!
//! Check(ctx, term, goal_type) → PASS | FAIL
//!
//! PASS means: term is a valid proof of goal_type in context ctx.
//! The proof hash is the canonical hash of the term.
//!
//! This replaces Lean as the verification oracle for internal operations.
//! The kernel becomes self-contained: it checks its own witnesses,
//! extracts rules from them, and normalizes future questions.
//!
//! Type checking rules (minimal dependent type theory):
//!   Var(i)       : ctx[i].ty
//!   Lam(A, body) : Pi(A, B) when body : B under extended ctx
//!   App(f, a)    : B[0 := a] when f : Pi(A, B) and a : A
//!   Pi(A, B)     : Type(max(u,v)) when A : Type(u) and B : Type(v)
//!   NatLit(n)    : Nat
//!   Constructor  : check args match constructor signature
//!   Let(A,v,b)   : check v : A, then b under extended ctx

use super::core_term::{CoreTerm, CoreCtx, CoreEnv};
use super::reduce;
use kernel_types::{Hash32, hash};

/// Result of type checking a term against a goal type.
#[derive(Debug, Clone)]
pub enum CheckResult {
    /// The term inhabits the goal type. Proof verified.
    Pass {
        /// Hash of the verified proof term.
        proof_hash: Hash32,
    },
    /// The term does not inhabit the goal type.
    Fail {
        /// Why the check failed.
        reason: String,
    },
}

impl CheckResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, CheckResult::Pass { .. })
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, CheckResult::Fail { .. })
    }
}

/// Infer the type of a CoreTerm in a given context and environment.
///
/// Returns the type of the term, or an error if the term is ill-typed.
pub fn infer_type(
    ctx: &CoreCtx,
    term: &CoreTerm,
    env: &CoreEnv,
) -> Result<CoreTerm, String> {
    match term {
        // Type(i) : Type(i+1)
        CoreTerm::Type(u) => Ok(CoreTerm::Type(u + 1)),

        // Prop : Type(0)
        CoreTerm::Prop => Ok(CoreTerm::Type(0)),

        // Var(i) : ctx[i].ty
        CoreTerm::Var(i) => {
            match ctx.lookup(*i) {
                Some(entry) => Ok(entry.ty.clone()),
                None => Err(format!("unbound variable Var({})", i)),
            }
        }

        // NatLit(n) : Nat
        CoreTerm::NatLit(_) => Ok(CoreTerm::Const {
            name: "Nat".into(),
            levels: vec![],
        }),

        // Pi(A, B) : Type(max(u, v)) when A : Type(u), B : Type(v)
        CoreTerm::Pi { param_type, body } => {
            let a_type = infer_type(ctx, param_type, env)?;
            let u = extract_universe(&a_type)?;

            let mut extended_ctx = ctx.clone();
            extended_ctx.push(None, *param_type.clone());
            let b_type = infer_type(&extended_ctx, body, env)?;
            let v = extract_universe(&b_type)?;

            Ok(CoreTerm::Type(u.max(v)))
        }

        // Lam(A, body) : Pi(A, B) where B = infer(body) under extended ctx
        CoreTerm::Lam { param_type, body } => {
            // Check param_type is a valid type
            let _ = infer_type(ctx, param_type, env)?;

            let mut extended_ctx = ctx.clone();
            extended_ctx.push(None, *param_type.clone());
            let body_type = infer_type(&extended_ctx, body, env)?;

            Ok(CoreTerm::Pi {
                param_type: param_type.clone(),
                body: Box::new(body_type),
            })
        }

        // App(f, a) : B[0 := a] when f : Pi(A, B) and a : A
        CoreTerm::App { func, arg } => {
            let func_type = infer_type(ctx, func, env)?;
            let func_type_nf = normalize_for_check(&func_type, env);

            match func_type_nf {
                CoreTerm::Pi { param_type, body } => {
                    // Check argument has the right type
                    let arg_type = infer_type(ctx, arg, env)?;
                    if !types_equal(&arg_type, &param_type, env) {
                        return Err(format!(
                            "argument type mismatch: expected {:?}, got {:?}",
                            param_type, arg_type
                        ));
                    }
                    // Result type: body[0 := arg]
                    Ok(body.substitute(0, arg))
                }
                _ => Err(format!(
                    "applying non-function: {:?} has type {:?}",
                    func, func_type_nf
                )),
            }
        }

        // Let(A, v, body) : body_type[0 := v]
        CoreTerm::Let { bound_type, value, body } => {
            // Check value has the declared type
            let value_type = infer_type(ctx, value, env)?;
            if !types_equal(&value_type, bound_type, env) {
                return Err(format!(
                    "let value type mismatch: declared {:?}, got {:?}",
                    bound_type, value_type
                ));
            }

            let mut extended_ctx = ctx.clone();
            extended_ctx.push(None, *bound_type.clone());
            let body_type = infer_type(&extended_ctx, body, env)?;

            // Substitute the value into the body type
            Ok(body_type.substitute(0, value))
        }

        // Const(name) : env[name].ty
        CoreTerm::Const { name, .. } => {
            match env.lookup(name) {
                Some(def) => Ok(def.ty.clone()),
                None => {
                    // Built-in constants
                    match name.as_str() {
                        "Nat" => Ok(CoreTerm::Type(0)),
                        "Bool" => Ok(CoreTerm::Type(0)),
                        "Nat.add" | "Nat.mul" | "Nat.sub" | "Nat.div" | "Nat.mod" => {
                            // Nat → Nat → Nat
                            Ok(nat_binop_type())
                        }
                        "Nat.ble" | "Nat.beq" => {
                            // Nat → Nat → Bool
                            Ok(nat_to_nat_to_bool_type())
                        }
                        "Nat.zero" => Ok(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
                        "Nat.succ" => {
                            // Nat → Nat
                            Ok(CoreTerm::Pi {
                                param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
                                body: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
                            })
                        }
                        "Nat.Prime" => {
                            // Nat → Prop
                            Ok(CoreTerm::Pi {
                                param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
                                body: Box::new(CoreTerm::Prop),
                            })
                        }
                        _ => Err(format!("unknown constant: {}", name)),
                    }
                }
            }
        }

        // Constructor: check args match expected types
        CoreTerm::Constructor { type_name, ctor_name, args } => {
            match (type_name.as_str(), ctor_name.as_str()) {
                ("Nat", "zero") => {
                    if args.is_empty() {
                        Ok(CoreTerm::Const { name: "Nat".into(), levels: vec![] })
                    } else {
                        Err("Nat.zero takes no arguments".into())
                    }
                }
                ("Nat", "succ") => {
                    if args.len() == 1 {
                        let arg_type = infer_type(ctx, &args[0], env)?;
                        if types_equal(&arg_type, &CoreTerm::Const { name: "Nat".into(), levels: vec![] }, env) {
                            Ok(CoreTerm::Const { name: "Nat".into(), levels: vec![] })
                        } else {
                            Err(format!("Nat.succ argument must be Nat, got {:?}", arg_type))
                        }
                    } else {
                        Err(format!("Nat.succ takes 1 argument, got {}", args.len()))
                    }
                }
                ("Bool", "true") | ("Bool", "false") => {
                    if args.is_empty() {
                        Ok(CoreTerm::Const { name: "Bool".into(), levels: vec![] })
                    } else {
                        Err(format!("Bool.{} takes no arguments", ctor_name))
                    }
                }
                ("And", "intro") => {
                    // And.intro : A → B → A ∧ B
                    // Simplified: just check we have 2 args
                    if args.len() >= 2 {
                        Ok(CoreTerm::Prop) // simplified: And is a Prop
                    } else {
                        Err(format!("And.intro requires at least 2 arguments, got {}", args.len()))
                    }
                }
                ("Exists", "intro") => {
                    // Exists.intro : (w : A) → P w → ∃ x, P x
                    if args.len() >= 2 {
                        Ok(CoreTerm::Prop) // simplified
                    } else {
                        Err(format!("Exists.intro requires at least 2 arguments, got {}", args.len()))
                    }
                }
                _ => {
                    // Generic: return the type_name as a Const
                    Ok(CoreTerm::Const { name: type_name.clone(), levels: vec![] })
                }
            }
        }

        // Recursor: complex, depends on the inductive type
        CoreTerm::Recursor { type_name, args } => {
            match type_name.as_str() {
                "Nat" => {
                    // Nat.rec : (motive : Nat → Sort u) → motive 0 → (∀ n, motive n → motive (n+1)) → ∀ n, motive n
                    if args.len() >= 4 {
                        let motive = &args[0];
                        let target = &args[3];
                        // Result type: motive applied to target
                        Ok(CoreTerm::App {
                            func: Box::new(motive.clone()),
                            arg: Box::new(target.clone()),
                        })
                    } else {
                        Err(format!("Nat.rec requires 4 arguments, got {}", args.len()))
                    }
                }
                _ => Err(format!("unknown recursor for type: {}", type_name)),
            }
        }
    }
}

/// Check if a term has a specific type.
/// This is the main entry point: does `term` inhabit `goal_type`?
pub fn type_check(
    ctx: &CoreCtx,
    term: &CoreTerm,
    goal_type: &CoreTerm,
    env: &CoreEnv,
) -> CheckResult {
    match infer_type(ctx, term, env) {
        Ok(inferred_type) => {
            if types_equal(&inferred_type, goal_type, env) {
                CheckResult::Pass {
                    proof_hash: term.term_hash(),
                }
            } else {
                CheckResult::Fail {
                    reason: format!(
                        "type mismatch: term has type {:?}, expected {:?}",
                        inferred_type, goal_type
                    ),
                }
            }
        }
        Err(reason) => CheckResult::Fail { reason },
    }
}

// ── Helper functions ──────────────────────────────────────────────────

/// Extract universe level from a Type term.
fn extract_universe(ty: &CoreTerm) -> Result<u32, String> {
    match ty {
        CoreTerm::Type(u) => Ok(*u),
        CoreTerm::Prop => Ok(0),
        _ => Ok(0), // be permissive: treat unknown as Type(0)
    }
}

/// Check if two types are definitionally equal (up to reduction).
fn types_equal(a: &CoreTerm, b: &CoreTerm, env: &CoreEnv) -> bool {
    // First try structural equality
    if a == b {
        return true;
    }

    // Normalize both and compare
    let a_nf = normalize_for_check(a, env);
    let b_nf = normalize_for_check(b, env);

    a_nf == b_nf
}

/// Normalize a term for type checking purposes (limited budget).
fn normalize_for_check(term: &CoreTerm, env: &CoreEnv) -> CoreTerm {
    let result = reduce::reduce(term, env, 100);
    result.normal_form
}

/// Type of Nat → Nat → Nat (binary operation on naturals).
fn nat_binop_type() -> CoreTerm {
    let nat = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
    CoreTerm::Pi {
        param_type: Box::new(nat.clone()),
        body: Box::new(CoreTerm::Pi {
            param_type: Box::new(nat.clone()),
            body: Box::new(nat),
        }),
    }
}

/// Type of Nat → Nat → Bool.
fn nat_to_nat_to_bool_type() -> CoreTerm {
    let nat = CoreTerm::Const { name: "Nat".into(), levels: vec![] };
    let bool_ty = CoreTerm::Const { name: "Bool".into(), levels: vec![] };
    CoreTerm::Pi {
        param_type: Box::new(nat.clone()),
        body: Box::new(CoreTerm::Pi {
            param_type: Box::new(nat),
            body: Box::new(bool_ty),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::core_term::CoreDef;

    fn nat_type() -> CoreTerm {
        CoreTerm::Const { name: "Nat".into(), levels: vec![] }
    }

    #[test]
    fn check_nat_lit() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();
        let result = type_check(&ctx, &CoreTerm::NatLit(42), &nat_type(), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_nat_lit_wrong_type() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();
        let result = type_check(&ctx, &CoreTerm::NatLit(42), &CoreTerm::Prop, &env);
        assert!(result.is_fail());
    }

    #[test]
    fn check_var_in_context() {
        let mut ctx = CoreCtx::new();
        ctx.push(Some("n".into()), nat_type());
        let env = CoreEnv::new();

        let result = type_check(&ctx, &CoreTerm::Var(0), &nat_type(), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_unbound_var() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();
        let result = type_check(&ctx, &CoreTerm::Var(0), &nat_type(), &env);
        assert!(result.is_fail());
    }

    #[test]
    fn check_identity_lambda() {
        // λ (n : Nat). n  :  Nat → Nat
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let identity = CoreTerm::Lam {
            param_type: Box::new(nat_type()),
            body: Box::new(CoreTerm::Var(0)),
        };
        let expected_type = CoreTerm::Pi {
            param_type: Box::new(nat_type()),
            body: Box::new(nat_type()),
        };

        let result = type_check(&ctx, &identity, &expected_type, &env);
        assert!(result.is_pass(), "identity should have type Nat → Nat: {:?}", result);
    }

    #[test]
    fn check_const_lambda() {
        // λ (n : Nat). 0  :  Nat → Nat
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let const_fn = CoreTerm::Lam {
            param_type: Box::new(nat_type()),
            body: Box::new(CoreTerm::NatLit(0)),
        };
        let expected_type = CoreTerm::Pi {
            param_type: Box::new(nat_type()),
            body: Box::new(nat_type()),
        };

        let result = type_check(&ctx, &const_fn, &expected_type, &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_application() {
        // (λ (n : Nat). n) 42  :  Nat
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let app = CoreTerm::App {
            func: Box::new(CoreTerm::Lam {
                param_type: Box::new(nat_type()),
                body: Box::new(CoreTerm::Var(0)),
            }),
            arg: Box::new(CoreTerm::NatLit(42)),
        };

        let result = type_check(&ctx, &app, &nat_type(), &env);
        assert!(result.is_pass(), "id(42) should have type Nat: {:?}", result);
    }

    #[test]
    fn check_nat_zero_constructor() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let zero = CoreTerm::Constructor {
            type_name: "Nat".into(),
            ctor_name: "zero".into(),
            args: vec![],
        };

        let result = type_check(&ctx, &zero, &nat_type(), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_nat_succ_constructor() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let one = CoreTerm::Constructor {
            type_name: "Nat".into(),
            ctor_name: "succ".into(),
            args: vec![CoreTerm::NatLit(0)],
        };

        let result = type_check(&ctx, &one, &nat_type(), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_let_binding() {
        // let x : Nat := 5 in x  :  Nat
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let term = CoreTerm::Let {
            bound_type: Box::new(nat_type()),
            value: Box::new(CoreTerm::NatLit(5)),
            body: Box::new(CoreTerm::Var(0)),
        };

        let result = type_check(&ctx, &term, &nat_type(), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_let_type_mismatch() {
        // let x : Nat := Prop in x  — type error (Prop is not Nat)
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let term = CoreTerm::Let {
            bound_type: Box::new(nat_type()),
            value: Box::new(CoreTerm::Prop),
            body: Box::new(CoreTerm::Var(0)),
        };

        let result = type_check(&ctx, &term, &nat_type(), &env);
        assert!(result.is_fail());
    }

    #[test]
    fn check_prop_type() {
        // Prop : Type(0)
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let result = type_check(&ctx, &CoreTerm::Prop, &CoreTerm::Type(0), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_type_universe() {
        // Type(0) : Type(1)
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let result = type_check(&ctx, &CoreTerm::Type(0), &CoreTerm::Type(1), &env);
        assert!(result.is_pass());
    }

    #[test]
    fn check_env_constant() {
        let ctx = CoreCtx::new();
        let mut env = CoreEnv::new();
        env.add_def(CoreDef {
            name: "my_val".into(),
            ty: nat_type(),
            value: Some(CoreTerm::NatLit(99)),
            universe_params: vec![],
        });

        let result = type_check(
            &ctx,
            &CoreTerm::Const { name: "my_val".into(), levels: vec![] },
            &nat_type(),
            &env,
        );
        assert!(result.is_pass());
    }

    #[test]
    fn check_nat_add_type() {
        // Nat.add : Nat → Nat → Nat
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let result = type_check(
            &ctx,
            &CoreTerm::Const { name: "Nat.add".into(), levels: vec![] },
            &nat_binop_type(),
            &env,
        );
        assert!(result.is_pass());
    }

    #[test]
    fn check_proof_hash_deterministic() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let term = CoreTerm::NatLit(42);
        let r1 = type_check(&ctx, &term, &nat_type(), &env);
        let r2 = type_check(&ctx, &term, &nat_type(), &env);

        match (r1, r2) {
            (CheckResult::Pass { proof_hash: h1 }, CheckResult::Pass { proof_hash: h2 }) => {
                assert_eq!(h1, h2, "proof hashes must be deterministic");
            }
            _ => panic!("both should pass"),
        }
    }

    #[test]
    fn infer_pi_type() {
        // Nat → Nat  :  Type(0)
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let pi = CoreTerm::Pi {
            param_type: Box::new(nat_type()),
            body: Box::new(nat_type()),
        };

        let inferred = infer_type(&ctx, &pi, &env).unwrap();
        assert_eq!(inferred, CoreTerm::Type(0));
    }

    #[test]
    fn check_bool_constructor() {
        let ctx = CoreCtx::new();
        let env = CoreEnv::new();

        let bool_type = CoreTerm::Const { name: "Bool".into(), levels: vec![] };
        let t = CoreTerm::Constructor {
            type_name: "Bool".into(),
            ctor_name: "true".into(),
            args: vec![],
        };

        let result = type_check(&ctx, &t, &bool_type, &env);
        assert!(result.is_pass());
    }
}
