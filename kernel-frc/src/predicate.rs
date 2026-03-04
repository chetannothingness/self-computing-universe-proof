// Predicate AST and Compiler — compiles boolean predicates over integer
// expressions into VM bytecode.
//
// This is the bridge between contract semantics (BoolCnf, ArithFind, Table)
// and the FRC VM. Every predicate compiled here genuinely evaluates the
// contract's logic — no proxy predicates, no fake schemas.
//
// Architecture:
//   Expr  — integer arithmetic expression tree
//   Pred  — boolean predicate over Expr nodes
//   VarEnv — variable-to-memory-slot mapping (BTreeMap for determinism)
//   PredicateCompiler — Expr/Pred → Vec<Instruction>
//   cnf_to_pred — DIMACS-style CNF clauses → Pred AST
//   poly_eq_pred — polynomial coefficients + target → Pred AST

use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
use crate::vm::Instruction;

/// Integer arithmetic expression.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Expr {
    /// Constant literal.
    Lit(i64),
    /// Variable (resolved to memory slot via VarEnv).
    Var(String),
    /// Addition: a + b.
    Add(Box<Expr>, Box<Expr>),
    /// Subtraction: a - b.
    Sub(Box<Expr>, Box<Expr>),
    /// Multiplication: a * b.
    Mul(Box<Expr>, Box<Expr>),
    /// Floor division: a / b (div-by-zero → VM fault).
    Div(Box<Expr>, Box<Expr>),
    /// Remainder: a % b (mod-by-zero → VM fault).
    Mod(Box<Expr>, Box<Expr>),
    /// Negation: -a.
    Neg(Box<Expr>),
}

/// Boolean predicate over integer expressions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Pred {
    /// Always true (pushes 1).
    True,
    /// Always false (pushes 0).
    False,
    /// Equality: a == b.
    Eq(Expr, Expr),
    /// Inequality: a != b.
    Ne(Expr, Expr),
    /// Less than: a < b.
    Lt(Expr, Expr),
    /// Less than or equal: a <= b.
    Le(Expr, Expr),
    /// Greater than: a > b.
    Gt(Expr, Expr),
    /// Greater than or equal: a >= b.
    Ge(Expr, Expr),
    /// Conjunction: p AND q (bitwise AND on 0/1 values).
    And(Box<Pred>, Box<Pred>),
    /// Disjunction: p OR q (bitwise OR on 0/1 values).
    Or(Box<Pred>, Box<Pred>),
    /// Negation: NOT p (0→1, nonzero→0).
    Not(Box<Pred>),
}

/// Variable-to-memory-slot mapping. BTreeMap ensures deterministic ordering.
#[derive(Debug, Clone)]
pub struct VarEnv {
    bindings: BTreeMap<String, usize>,
    next_slot: usize,
}

impl VarEnv {
    /// Create a new VarEnv with the first available slot.
    pub fn new(first_slot: usize) -> Self {
        Self {
            bindings: BTreeMap::new(),
            next_slot: first_slot,
        }
    }

    /// Bind a variable name to the next available slot.
    /// Returns the assigned slot. If already bound, returns existing slot.
    pub fn bind(&mut self, name: &str) -> usize {
        if let Some(&slot) = self.bindings.get(name) {
            return slot;
        }
        let slot = self.next_slot;
        self.bindings.insert(name.to_string(), slot);
        self.next_slot += 1;
        slot
    }

    /// Look up a variable's slot. Returns None if not bound.
    pub fn lookup(&self, name: &str) -> Option<usize> {
        self.bindings.get(name).copied()
    }

    /// Number of bound variables.
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }

    /// Next slot that would be assigned.
    pub fn next_slot(&self) -> usize {
        self.next_slot
    }
}

/// Compilation error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileError {
    UnboundVariable(String),
}

impl std::fmt::Display for CompileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompileError::UnboundVariable(name) => write!(f, "unbound variable: {}", name),
        }
    }
}

/// Predicate compiler: Expr/Pred AST → Vec<Instruction>.
///
/// All compilation is against the VM semantics in vm.rs:
///   - Binary ops pop b (top), then a (second).
///   - Lt pushes (a < b), Eq pushes (a == b).
///   - And/Or are bitwise on 0/1 values.
///   - Not: pop a, push (a == 0 ? 1 : 0).
pub struct PredicateCompiler {
    env: VarEnv,
}

impl PredicateCompiler {
    pub fn new(env: VarEnv) -> Self {
        Self { env }
    }

    /// Compile an expression to instructions that push its value onto the stack.
    pub fn compile_expr(&self, expr: &Expr) -> Result<Vec<Instruction>, CompileError> {
        match expr {
            Expr::Lit(v) => Ok(vec![Instruction::Push(*v)]),
            Expr::Var(name) => {
                let slot = self.env.lookup(name)
                    .ok_or_else(|| CompileError::UnboundVariable(name.clone()))?;
                Ok(vec![Instruction::Load(slot)])
            }
            Expr::Add(a, b) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Add);
                Ok(instrs)
            }
            Expr::Sub(a, b) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Sub);
                Ok(instrs)
            }
            Expr::Mul(a, b) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Mul);
                Ok(instrs)
            }
            Expr::Div(a, b) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Div);
                Ok(instrs)
            }
            Expr::Mod(a, b) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Mod);
                Ok(instrs)
            }
            Expr::Neg(a) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.push(Instruction::Neg);
                Ok(instrs)
            }
        }
    }

    /// Compile a predicate to instructions that push 0 or 1 onto the stack.
    ///
    /// VM stack semantics (verified against vm.rs):
    ///   Binary ops pop b first, then a. So stack [..., a, b] → [..., result].
    ///   - Eq: a == b ? 1 : 0
    ///   - Lt: a < b ? 1 : 0
    ///   - Le(a,b): NOT(b < a) → compile [b, a, Lt, Not]
    ///   - Gt(a,b): b < a → compile [b, a, Lt]
    ///   - Ge(a,b): NOT(a < b) → compile [a, b, Lt, Not]
    ///   - Ne(a,b): NOT(a == b) → compile [a, b, Eq, Not]
    pub fn compile_pred(&self, pred: &Pred) -> Result<Vec<Instruction>, CompileError> {
        match pred {
            Pred::True => Ok(vec![Instruction::Push(1)]),
            Pred::False => Ok(vec![Instruction::Push(0)]),
            Pred::Eq(a, b) => {
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Eq);
                Ok(instrs)
            }
            Pred::Ne(a, b) => {
                // a != b → NOT(a == b) → [a, b, Eq, Not]
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Eq);
                instrs.push(Instruction::Not);
                Ok(instrs)
            }
            Pred::Lt(a, b) => {
                // a < b → [a, b, Lt]
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Lt);
                Ok(instrs)
            }
            Pred::Le(a, b) => {
                // a <= b → NOT(b < a) → [b, a, Lt, Not]
                let mut instrs = self.compile_expr(b)?;
                instrs.extend(self.compile_expr(a)?);
                instrs.push(Instruction::Lt);
                instrs.push(Instruction::Not);
                Ok(instrs)
            }
            Pred::Gt(a, b) => {
                // a > b → b < a → [b, a, Lt]
                let mut instrs = self.compile_expr(b)?;
                instrs.extend(self.compile_expr(a)?);
                instrs.push(Instruction::Lt);
                Ok(instrs)
            }
            Pred::Ge(a, b) => {
                // a >= b → NOT(a < b) → [a, b, Lt, Not]
                let mut instrs = self.compile_expr(a)?;
                instrs.extend(self.compile_expr(b)?);
                instrs.push(Instruction::Lt);
                instrs.push(Instruction::Not);
                Ok(instrs)
            }
            Pred::And(p, q) => {
                let mut instrs = self.compile_pred(p)?;
                instrs.extend(self.compile_pred(q)?);
                instrs.push(Instruction::And);
                Ok(instrs)
            }
            Pred::Or(p, q) => {
                let mut instrs = self.compile_pred(p)?;
                instrs.extend(self.compile_pred(q)?);
                instrs.push(Instruction::Or);
                Ok(instrs)
            }
            Pred::Not(p) => {
                let mut instrs = self.compile_pred(p)?;
                instrs.push(Instruction::Not);
                Ok(instrs)
            }
        }
    }

    /// Count the number of instructions an expression compiles to (for B* derivation).
    pub fn expr_instruction_count(expr: &Expr) -> usize {
        match expr {
            Expr::Lit(_) => 1,
            Expr::Var(_) => 1,
            Expr::Add(a, b) | Expr::Sub(a, b) | Expr::Mul(a, b)
            | Expr::Div(a, b) | Expr::Mod(a, b) => {
                Self::expr_instruction_count(a) + Self::expr_instruction_count(b) + 1
            }
            Expr::Neg(a) => Self::expr_instruction_count(a) + 1,
        }
    }

    /// Count the number of instructions a predicate compiles to (for B* derivation).
    pub fn pred_instruction_count(pred: &Pred) -> usize {
        match pred {
            Pred::True | Pred::False => 1,
            Pred::Eq(a, b) | Pred::Lt(a, b) => {
                Self::expr_instruction_count(a) + Self::expr_instruction_count(b) + 1
            }
            Pred::Ne(a, b) | Pred::Le(a, b) | Pred::Ge(a, b) => {
                // These compile to 2 extra ops (swap operands for Le/Ge, or Eq+Not for Ne)
                Self::expr_instruction_count(a) + Self::expr_instruction_count(b) + 2
            }
            Pred::Gt(a, b) => {
                // a > b → [b, a, Lt] — same count as Lt but operands swapped
                Self::expr_instruction_count(a) + Self::expr_instruction_count(b) + 1
            }
            Pred::And(p, q) | Pred::Or(p, q) => {
                Self::pred_instruction_count(p) + Self::pred_instruction_count(q) + 1
            }
            Pred::Not(p) => Self::pred_instruction_count(p) + 1,
        }
    }
}

/// Convert DIMACS-style CNF clauses to a Pred AST.
///
/// Each clause is a Vec<i32> of literals:
///   +i means variable (i-1) is true (bit i-1 of assignment integer is 1)
///   -i means variable (i-1) is false (bit i-1 is 0)
///
/// The assignment is read from a single integer variable "a" via bit extraction:
///   bit k = (a / 2^k) % 2
///
/// A positive literal i is satisfied when bit (i-1) != 0.
/// A negative literal -i is satisfied when bit (i-1) == 0.
///
/// Each clause is an OR of its literals.
/// The CNF is an AND of all clauses.
///
/// Empty clauses list → True (vacuously satisfied).
/// Empty clause → False (unsatisfiable).
pub fn cnf_to_pred(clauses: &[Vec<i32>]) -> Pred {
    if clauses.is_empty() {
        return Pred::True;
    }

    let clause_preds: Vec<Pred> = clauses.iter().map(|clause| {
        if clause.is_empty() {
            return Pred::False;
        }

        let lit_preds: Vec<Pred> = clause.iter().map(|&lit| {
            let var_idx = lit.unsigned_abs() as u32 - 1; // 1-indexed → 0-indexed
            let power = 1i64 << var_idx;
            // bit extraction: (a / 2^k) % 2
            let bit_expr = Expr::Mod(
                Box::new(Expr::Div(
                    Box::new(Expr::Var("a".to_string())),
                    Box::new(Expr::Lit(power)),
                )),
                Box::new(Expr::Lit(2)),
            );
            if lit > 0 {
                // positive literal: bit != 0
                Pred::Ne(bit_expr, Expr::Lit(0))
            } else {
                // negative literal: bit == 0
                Pred::Eq(bit_expr, Expr::Lit(0))
            }
        }).collect();

        // OR chain of literals
        let mut result = lit_preds.into_iter();
        let first = result.next().unwrap();
        result.fold(first, |acc, p| Pred::Or(Box::new(acc), Box::new(p)))
    }).collect();

    // AND chain of clauses
    let mut result = clause_preds.into_iter();
    let first = result.next().unwrap();
    result.fold(first, |acc, p| Pred::And(Box::new(acc), Box::new(p)))
}

/// Convert polynomial f(x) = c[0] + c[1]*x + c[2]*x^2 + ... and target
/// to a Pred: Eq(polynomial_expr, Lit(target)).
///
/// Generates: Eq(c0 + c1*x + c2*x*x + ..., target) using nested Add/Mul.
///
/// Empty coefficients → Eq(Lit(0), Lit(target)).
pub fn poly_eq_pred(coefficients: &[i64], target: i64) -> Pred {
    let poly = build_polynomial_expr(coefficients, "x");
    Pred::Eq(poly, Expr::Lit(target))
}

/// Build a polynomial expression: c[0] + c[1]*x + c[2]*x^2 + ...
fn build_polynomial_expr(coefficients: &[i64], var_name: &str) -> Expr {
    if coefficients.is_empty() {
        return Expr::Lit(0);
    }

    // Build terms: c[i] * x^i for each coefficient
    let mut terms: Vec<Expr> = Vec::new();

    for (i, &coeff) in coefficients.iter().enumerate() {
        if coeff == 0 {
            continue;
        }
        let x_power = if i == 0 {
            None // constant term, no x factor
        } else {
            // x^i = x * x * ... * x (i times)
            let x = Expr::Var(var_name.to_string());
            let mut power = x.clone();
            for _ in 1..i {
                power = Expr::Mul(Box::new(power), Box::new(x.clone()));
            }
            Some(power)
        };

        let term = match x_power {
            None => Expr::Lit(coeff),
            Some(xp) => {
                if coeff == 1 {
                    xp
                } else if coeff == -1 {
                    Expr::Neg(Box::new(xp))
                } else {
                    Expr::Mul(Box::new(Expr::Lit(coeff)), Box::new(xp))
                }
            }
        };
        terms.push(term);
    }

    if terms.is_empty() {
        return Expr::Lit(0);
    }

    // Sum all terms
    let mut result = terms.into_iter();
    let first = result.next().unwrap();
    result.fold(first, |acc, t| Expr::Add(Box::new(acc), Box::new(t)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vm::{Vm, Program, VmOutcome};

    /// Helper: compile a predicate and run it in the VM with given variable bindings.
    fn eval_pred(pred: &Pred, bindings: &[(&str, i64)]) -> i64 {
        let mut env = VarEnv::new(0);
        for &(name, _) in bindings {
            env.bind(name);
        }
        let compiler = PredicateCompiler::new(env.clone());
        let mut instrs = compiler.compile_pred(pred).unwrap();
        instrs.push(Instruction::Halt(0)); // halt after pushing result

        let program = Program::new(instrs);
        let mut state = crate::vm::VmState::initial();
        // Set variable values in memory
        for &(name, val) in bindings {
            let slot = env.lookup(name).unwrap();
            state.memory.insert(slot, val);
        }

        // Run manually
        while Vm::step(&program, &mut state) {}

        // The predicate result is on top of the stack
        *state.stack.last().unwrap_or(&-1)
    }

    /// Helper: compile an expression and run it in the VM.
    fn eval_expr(expr: &Expr, bindings: &[(&str, i64)]) -> i64 {
        let mut env = VarEnv::new(0);
        for &(name, _) in bindings {
            env.bind(name);
        }
        let compiler = PredicateCompiler::new(env.clone());
        let mut instrs = compiler.compile_expr(expr).unwrap();
        instrs.push(Instruction::Halt(0));

        let program = Program::new(instrs);
        let mut state = crate::vm::VmState::initial();
        for &(name, val) in bindings {
            let slot = env.lookup(name).unwrap();
            state.memory.insert(slot, val);
        }

        while Vm::step(&program, &mut state) {}
        *state.stack.last().unwrap_or(&-1)
    }

    // --- Expr compilation tests ---

    #[test]
    fn expr_lit() {
        assert_eq!(eval_expr(&Expr::Lit(42), &[]), 42);
        assert_eq!(eval_expr(&Expr::Lit(-7), &[]), -7);
        assert_eq!(eval_expr(&Expr::Lit(0), &[]), 0);
    }

    #[test]
    fn expr_var() {
        assert_eq!(eval_expr(&Expr::Var("x".into()), &[("x", 10)]), 10);
        assert_eq!(eval_expr(&Expr::Var("y".into()), &[("x", 3), ("y", 7)]), 7);
    }

    #[test]
    fn expr_add() {
        let e = Expr::Add(Box::new(Expr::Lit(3)), Box::new(Expr::Lit(4)));
        assert_eq!(eval_expr(&e, &[]), 7);
    }

    #[test]
    fn expr_sub() {
        let e = Expr::Sub(Box::new(Expr::Lit(10)), Box::new(Expr::Lit(3)));
        assert_eq!(eval_expr(&e, &[]), 7);
    }

    #[test]
    fn expr_mul() {
        let e = Expr::Mul(Box::new(Expr::Lit(6)), Box::new(Expr::Lit(7)));
        assert_eq!(eval_expr(&e, &[]), 42);
    }

    #[test]
    fn expr_div() {
        let e = Expr::Div(Box::new(Expr::Lit(10)), Box::new(Expr::Lit(3)));
        assert_eq!(eval_expr(&e, &[]), 3); // floor division
    }

    #[test]
    fn expr_mod() {
        let e = Expr::Mod(Box::new(Expr::Lit(10)), Box::new(Expr::Lit(3)));
        assert_eq!(eval_expr(&e, &[]), 1);
    }

    #[test]
    fn expr_neg() {
        let e = Expr::Neg(Box::new(Expr::Lit(5)));
        assert_eq!(eval_expr(&e, &[]), -5);
    }

    #[test]
    fn expr_nested_arithmetic() {
        // (x + 3) * (y - 1) where x=5, y=4 → 8*3 = 24
        let e = Expr::Mul(
            Box::new(Expr::Add(
                Box::new(Expr::Var("x".into())),
                Box::new(Expr::Lit(3)),
            )),
            Box::new(Expr::Sub(
                Box::new(Expr::Var("y".into())),
                Box::new(Expr::Lit(1)),
            )),
        );
        assert_eq!(eval_expr(&e, &[("x", 5), ("y", 4)]), 24);
    }

    // --- Pred compilation tests ---

    #[test]
    fn pred_true_false() {
        assert_eq!(eval_pred(&Pred::True, &[]), 1);
        assert_eq!(eval_pred(&Pred::False, &[]), 0);
    }

    #[test]
    fn pred_eq() {
        let p = Pred::Eq(Expr::Lit(5), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Eq(Expr::Lit(5), Expr::Lit(6));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_ne() {
        let p = Pred::Ne(Expr::Lit(5), Expr::Lit(6));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Ne(Expr::Lit(5), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_lt() {
        let p = Pred::Lt(Expr::Lit(3), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Lt(Expr::Lit(5), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
        let p = Pred::Lt(Expr::Lit(7), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_le() {
        let p = Pred::Le(Expr::Lit(3), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Le(Expr::Lit(5), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1); // equal values
        let p = Pred::Le(Expr::Lit(7), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_gt() {
        let p = Pred::Gt(Expr::Lit(7), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Gt(Expr::Lit(5), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
        let p = Pred::Gt(Expr::Lit(3), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_ge() {
        let p = Pred::Ge(Expr::Lit(7), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Ge(Expr::Lit(5), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 1); // equal values
        let p = Pred::Ge(Expr::Lit(3), Expr::Lit(5));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_and() {
        let p = Pred::And(Box::new(Pred::True), Box::new(Pred::True));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::And(Box::new(Pred::True), Box::new(Pred::False));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_or() {
        let p = Pred::Or(Box::new(Pred::False), Box::new(Pred::True));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Or(Box::new(Pred::False), Box::new(Pred::False));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    #[test]
    fn pred_not() {
        let p = Pred::Not(Box::new(Pred::True));
        assert_eq!(eval_pred(&p, &[]), 0);
        let p = Pred::Not(Box::new(Pred::False));
        assert_eq!(eval_pred(&p, &[]), 1);
    }

    #[test]
    fn pred_le_ge_boundary() {
        // Le and Ge at boundary: a=0, b=0
        let p = Pred::Le(Expr::Lit(0), Expr::Lit(0));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Ge(Expr::Lit(0), Expr::Lit(0));
        assert_eq!(eval_pred(&p, &[]), 1);
        // Negative values
        let p = Pred::Le(Expr::Lit(-1), Expr::Lit(0));
        assert_eq!(eval_pred(&p, &[]), 1);
        let p = Pred::Ge(Expr::Lit(-1), Expr::Lit(0));
        assert_eq!(eval_pred(&p, &[]), 0);
    }

    // --- VarEnv tests ---

    #[test]
    fn var_env_bind_and_lookup() {
        let mut env = VarEnv::new(0);
        assert_eq!(env.bind("x"), 0);
        assert_eq!(env.bind("y"), 1);
        assert_eq!(env.bind("x"), 0); // re-bind returns same slot
        assert_eq!(env.lookup("x"), Some(0));
        assert_eq!(env.lookup("y"), Some(1));
        assert_eq!(env.lookup("z"), None);
        assert_eq!(env.len(), 2);
    }

    #[test]
    fn var_env_deterministic() {
        // BTreeMap ensures deterministic ordering regardless of insertion order.
        let mut env1 = VarEnv::new(0);
        env1.bind("b");
        env1.bind("a");

        let mut env2 = VarEnv::new(0);
        env2.bind("b");
        env2.bind("a");

        // Same insertion order → same slots
        assert_eq!(env1.lookup("a"), env2.lookup("a"));
        assert_eq!(env1.lookup("b"), env2.lookup("b"));
    }

    #[test]
    fn var_env_custom_start_slot() {
        let mut env = VarEnv::new(4);
        assert_eq!(env.bind("x"), 4);
        assert_eq!(env.bind("y"), 5);
        assert_eq!(env.next_slot(), 6);
    }

    // --- cnf_to_pred tests ---

    #[test]
    fn cnf_empty_clauses() {
        // Empty CNF is vacuously true.
        let pred = cnf_to_pred(&[]);
        assert_eq!(pred, Pred::True);
    }

    #[test]
    fn cnf_single_clause() {
        // (x1) — clause with one positive literal
        let pred = cnf_to_pred(&[vec![1]]);
        // For a=1 (bit 0 set), should be satisfied
        assert_eq!(eval_pred(&pred, &[("a", 1)]), 1);
        // For a=0 (bit 0 not set), should not be satisfied
        assert_eq!(eval_pred(&pred, &[("a", 0)]), 0);
    }

    #[test]
    fn cnf_multi_clause() {
        // (x1 OR x2) AND (NOT x1 OR x2) AND (NOT x2)
        // This is UNSAT: clause 3 forces x2=0, then clause 2 forces x1=0,
        // but clause 1 needs x1=1 or x2=1.
        let clauses = vec![vec![1, 2], vec![-1, 2], vec![-2]];
        let pred = cnf_to_pred(&clauses);
        // Check all 4 assignments: none should satisfy
        for a in 0..4i64 {
            assert_eq!(eval_pred(&pred, &[("a", a)]), 0,
                "assignment {} should not satisfy", a);
        }
    }

    #[test]
    fn cnf_satisfiable() {
        // (x1 OR x2) — satisfied by a=1,2,3
        let pred = cnf_to_pred(&[vec![1, 2]]);
        assert_eq!(eval_pred(&pred, &[("a", 0)]), 0); // both false
        assert_eq!(eval_pred(&pred, &[("a", 1)]), 1); // x1=true
        assert_eq!(eval_pred(&pred, &[("a", 2)]), 1); // x2=true
        assert_eq!(eval_pred(&pred, &[("a", 3)]), 1); // both true
    }

    #[test]
    fn cnf_negative_literal() {
        // (NOT x1) — satisfied only when x1=false (a=0 or a=2)
        let pred = cnf_to_pred(&[vec![-1]]);
        assert_eq!(eval_pred(&pred, &[("a", 0)]), 1); // bit 0 = 0 → NOT x1 true
        assert_eq!(eval_pred(&pred, &[("a", 1)]), 0); // bit 0 = 1 → NOT x1 false
    }

    // --- poly_eq_pred tests ---

    #[test]
    fn poly_constant() {
        // f(x) = 5, target = 5 → always true
        let pred = poly_eq_pred(&[5], 5);
        assert_eq!(eval_pred(&pred, &[("x", 0)]), 1);
        assert_eq!(eval_pred(&pred, &[("x", 100)]), 1);
    }

    #[test]
    fn poly_linear() {
        // f(x) = 3 + 2*x, target = 7 → x=2
        let pred = poly_eq_pred(&[3, 2], 7);
        assert_eq!(eval_pred(&pred, &[("x", 2)]), 1);
        assert_eq!(eval_pred(&pred, &[("x", 1)]), 0);
        assert_eq!(eval_pred(&pred, &[("x", 3)]), 0);
    }

    #[test]
    fn poly_quadratic() {
        // f(x) = x^2 = 0 + 0*x + 1*x^2, target = 49 → x=7 or x=-7
        let pred = poly_eq_pred(&[0, 0, 1], 49);
        assert_eq!(eval_pred(&pred, &[("x", 7)]), 1);
        assert_eq!(eval_pred(&pred, &[("x", -7)]), 1);
        assert_eq!(eval_pred(&pred, &[("x", 6)]), 0);
    }

    // --- Instruction count tests ---

    #[test]
    fn instruction_count_expr() {
        assert_eq!(PredicateCompiler::expr_instruction_count(&Expr::Lit(1)), 1);
        assert_eq!(PredicateCompiler::expr_instruction_count(&Expr::Var("x".into())), 1);
        let add = Expr::Add(Box::new(Expr::Lit(1)), Box::new(Expr::Lit(2)));
        assert_eq!(PredicateCompiler::expr_instruction_count(&add), 3); // Push, Push, Add
    }

    #[test]
    fn instruction_count_pred() {
        assert_eq!(PredicateCompiler::pred_instruction_count(&Pred::True), 1);
        let eq = Pred::Eq(Expr::Lit(1), Expr::Lit(2));
        assert_eq!(PredicateCompiler::pred_instruction_count(&eq), 3); // Push, Push, Eq
        let ne = Pred::Ne(Expr::Lit(1), Expr::Lit(2));
        assert_eq!(PredicateCompiler::pred_instruction_count(&ne), 4); // Push, Push, Eq, Not
    }

    #[test]
    fn instruction_count_matches_actual() {
        // Verify that instruction_count matches actual compiled length
        let pred = Pred::And(
            Box::new(Pred::Lt(Expr::Var("x".into()), Expr::Lit(10))),
            Box::new(Pred::Ge(Expr::Var("x".into()), Expr::Lit(0))),
        );
        let expected_count = PredicateCompiler::pred_instruction_count(&pred);

        let mut env = VarEnv::new(0);
        env.bind("x");
        let compiler = PredicateCompiler::new(env);
        let instrs = compiler.compile_pred(&pred).unwrap();

        assert_eq!(instrs.len(), expected_count);
    }

    // --- Integration: compile pred → run in VM → check result ---

    #[test]
    fn integration_compile_and_run_cnf() {
        // (x1 OR x2) AND (NOT x1 OR x2) → x2 must be true
        // Assignment a=2 (x1=false, x2=true) should satisfy
        // Assignment a=3 (x1=true, x2=true) should satisfy
        let pred = cnf_to_pred(&[vec![1, 2], vec![-1, 2]]);

        let mut env = VarEnv::new(0);
        env.bind("a");
        let compiler = PredicateCompiler::new(env);
        let mut instrs = compiler.compile_pred(&pred).unwrap();
        instrs.push(Instruction::Halt(0));

        let program = Program::new(instrs);

        // Test a=2 (x2=true, x1=false)
        let (outcome, _state) = Vm::run(&program, 1000);
        assert_eq!(outcome, VmOutcome::Halted(0));
        // Memory slot 0 wasn't initialized, defaults to 0 → a=0
        // With a=0: clause 1 is (false OR false) = false → not satisfied
        // Let's manually set memory
        let mut state = crate::vm::VmState::initial();
        state.memory.insert(0, 2); // a=2
        while Vm::step(&program, &mut state) {}
        assert_eq!(*state.stack.last().unwrap(), 1); // satisfied

        let mut state = crate::vm::VmState::initial();
        state.memory.insert(0, 0); // a=0
        while Vm::step(&program, &mut state) {}
        assert_eq!(*state.stack.last().unwrap(), 0); // not satisfied
    }

    #[test]
    fn integration_compile_and_run_poly() {
        // f(x) = 2x + 3, target = 7 → x=2
        let pred = poly_eq_pred(&[3, 2], 7);

        let mut env = VarEnv::new(0);
        env.bind("x");
        let compiler = PredicateCompiler::new(env);
        let mut instrs = compiler.compile_pred(&pred).unwrap();
        instrs.push(Instruction::Halt(0));

        let program = Program::new(instrs);

        let mut state = crate::vm::VmState::initial();
        state.memory.insert(0, 2); // x=2
        while Vm::step(&program, &mut state) {}
        assert_eq!(*state.stack.last().unwrap(), 1); // 2*2+3=7 ✓

        let mut state = crate::vm::VmState::initial();
        state.memory.insert(0, 3); // x=3
        while Vm::step(&program, &mut state) {}
        assert_eq!(*state.stack.last().unwrap(), 0); // 2*3+3=9≠7
    }

    #[test]
    fn unbound_variable_error() {
        let env = VarEnv::new(0);
        let compiler = PredicateCompiler::new(env);
        let result = compiler.compile_expr(&Expr::Var("missing".into()));
        assert_eq!(result, Err(CompileError::UnboundVariable("missing".into())));
    }

    #[test]
    fn poly_empty_coefficients() {
        // f(x) = 0, target = 0 → always true
        let pred = poly_eq_pred(&[], 0);
        assert_eq!(eval_pred(&pred, &[("x", 42)]), 1);
        // f(x) = 0, target = 1 → always false
        let pred = poly_eq_pred(&[], 1);
        assert_eq!(eval_pred(&pred, &[("x", 42)]), 0);
    }
}
