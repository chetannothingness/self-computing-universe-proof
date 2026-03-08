//! CoreTerm — the internal typed language of the Π-normalizer.
//!
//! Every byte string that passes ELAB becomes a CoreTerm.
//! CoreTerms are the objects that the normalizer rewrites.
//! The proof of any statement IS the normalization trace of its CoreTerm.
//!
//! This is a minimal dependent type theory operating on bytes:
//!   - Type universes (Type_i)
//!   - Prop (the type of propositions)
//!   - Variables (de Bruijn indices)
//!   - Lambda, Pi (dependent function types), Application
//!   - Let bindings
//!   - Constants (references to definitions)
//!   - Natural number literals
//!   - Constructors and eliminators (for inductive types)
//!
//! Every CoreTerm has a canonical byte serialization via SerPi.
//! H(ser_pi(t)) names the term uniquely.

use kernel_types::{Hash32, hash};

/// A core term in the Π-normalizer's internal language.
///
/// This mirrors Lean4's kernel term structure, operating at the byte level.
/// De Bruijn indices for variables (matching existing invsyn infrastructure).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreTerm {
    /// Type universe at level i. Type(0) = Type, Type(1) = Type₁, etc.
    Type(u32),

    /// Prop — the type of propositions. Proof-irrelevant universe.
    Prop,

    /// Variable reference (de Bruijn index).
    Var(usize),

    /// Lambda abstraction: λ (param_type) (body).
    /// Body uses de Bruijn index 0 for the bound variable.
    Lam {
        param_type: Box<CoreTerm>,
        body: Box<CoreTerm>,
    },

    /// Pi type (dependent function type): Π (param_type) (body).
    /// `∀ x : A, B(x)` is `Pi { param_type: A, body: B }`.
    Pi {
        param_type: Box<CoreTerm>,
        body: Box<CoreTerm>,
    },

    /// Application: (function) (argument).
    App {
        func: Box<CoreTerm>,
        arg: Box<CoreTerm>,
    },

    /// Let binding: let x : type = value in body.
    Let {
        bound_type: Box<CoreTerm>,
        value: Box<CoreTerm>,
        body: Box<CoreTerm>,
    },

    /// Reference to a named constant in the environment.
    /// Universe levels for polymorphic definitions.
    Const {
        name: String,
        levels: Vec<u32>,
    },

    /// Natural number literal.
    NatLit(u64),

    /// Inductive type constructor application.
    /// e.g., `Nat.succ`, `List.cons`, `And.intro`.
    Constructor {
        type_name: String,
        ctor_name: String,
        args: Vec<CoreTerm>,
    },

    /// Recursor (eliminator / pattern matching).
    /// Applies an elimination principle to a term of an inductive type.
    Recursor {
        type_name: String,
        args: Vec<CoreTerm>,
    },
}

/// A typing context entry — a variable with its type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtxEntry {
    /// Optional name (for readability; de Bruijn index is authoritative).
    pub name: Option<String>,
    /// The type of this variable.
    pub ty: CoreTerm,
}

/// Typing context — a stack of variable types (de Bruijn).
/// Index 0 = most recently bound variable.
#[derive(Debug, Clone)]
pub struct CoreCtx {
    entries: Vec<CtxEntry>,
}

impl CoreCtx {
    /// Empty context.
    pub fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Push a new variable onto the context.
    pub fn push(&mut self, name: Option<String>, ty: CoreTerm) {
        self.entries.push(CtxEntry { name, ty });
    }

    /// Pop the most recent variable.
    pub fn pop(&mut self) -> Option<CtxEntry> {
        self.entries.pop()
    }

    /// Look up a variable by de Bruijn index.
    /// Index 0 = last pushed (most recent).
    pub fn lookup(&self, idx: usize) -> Option<&CtxEntry> {
        if idx < self.entries.len() {
            Some(&self.entries[self.entries.len() - 1 - idx])
        } else {
            None
        }
    }

    /// Number of variables in context.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Is the context empty?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

/// A definition in the environment (named constant).
#[derive(Debug, Clone)]
pub struct CoreDef {
    /// Name of this definition.
    pub name: String,
    /// The type of this definition.
    pub ty: CoreTerm,
    /// The value (if defined; axioms have None).
    pub value: Option<CoreTerm>,
    /// Universe parameters.
    pub universe_params: Vec<String>,
}

/// The global environment — all defined constants.
#[derive(Debug, Clone)]
pub struct CoreEnv {
    defs: Vec<CoreDef>,
}

impl CoreEnv {
    /// Empty environment.
    pub fn new() -> Self {
        Self { defs: Vec::new() }
    }

    /// Add a definition.
    pub fn add_def(&mut self, def: CoreDef) {
        self.defs.push(def);
    }

    /// Look up a definition by name.
    pub fn lookup(&self, name: &str) -> Option<&CoreDef> {
        self.defs.iter().find(|d| d.name == name)
    }

    /// Number of definitions.
    pub fn len(&self) -> usize {
        self.defs.len()
    }

    /// Is the environment empty?
    pub fn is_empty(&self) -> bool {
        self.defs.is_empty()
    }
}

// ── Substitution and shifting (de Bruijn operations) ──────────────────

impl CoreTerm {
    /// Substitute variable at `idx` with `replacement` in this term.
    /// Handles de Bruijn shifting correctly.
    pub fn substitute(&self, idx: usize, replacement: &CoreTerm) -> CoreTerm {
        match self {
            CoreTerm::Type(u) => CoreTerm::Type(*u),
            CoreTerm::Prop => CoreTerm::Prop,
            CoreTerm::NatLit(n) => CoreTerm::NatLit(*n),

            CoreTerm::Var(i) => {
                if *i == idx {
                    replacement.clone()
                } else if *i > idx {
                    CoreTerm::Var(*i - 1) // shift down (variable above substitution point)
                } else {
                    CoreTerm::Var(*i)
                }
            }

            CoreTerm::Lam { param_type, body } => CoreTerm::Lam {
                param_type: Box::new(param_type.substitute(idx, replacement)),
                body: Box::new(body.substitute(idx + 1, &replacement.shift(0))),
            },

            CoreTerm::Pi { param_type, body } => CoreTerm::Pi {
                param_type: Box::new(param_type.substitute(idx, replacement)),
                body: Box::new(body.substitute(idx + 1, &replacement.shift(0))),
            },

            CoreTerm::App { func, arg } => CoreTerm::App {
                func: Box::new(func.substitute(idx, replacement)),
                arg: Box::new(arg.substitute(idx, replacement)),
            },

            CoreTerm::Let { bound_type, value, body } => CoreTerm::Let {
                bound_type: Box::new(bound_type.substitute(idx, replacement)),
                value: Box::new(value.substitute(idx, replacement)),
                body: Box::new(body.substitute(idx + 1, &replacement.shift(0))),
            },

            CoreTerm::Const { name, levels } => CoreTerm::Const {
                name: name.clone(),
                levels: levels.clone(),
            },

            CoreTerm::Constructor { type_name, ctor_name, args } => CoreTerm::Constructor {
                type_name: type_name.clone(),
                ctor_name: ctor_name.clone(),
                args: args.iter().map(|a| a.substitute(idx, replacement)).collect(),
            },

            CoreTerm::Recursor { type_name, args } => CoreTerm::Recursor {
                type_name: type_name.clone(),
                args: args.iter().map(|a| a.substitute(idx, replacement)).collect(),
            },
        }
    }

    /// Shift all free variables with index ≥ cutoff by +1.
    /// Used when moving a term under a binder.
    pub fn shift(&self, cutoff: usize) -> CoreTerm {
        self.shift_by(cutoff, 1)
    }

    /// Shift all free variables with index ≥ cutoff by `amount`.
    fn shift_by(&self, cutoff: usize, amount: usize) -> CoreTerm {
        match self {
            CoreTerm::Type(u) => CoreTerm::Type(*u),
            CoreTerm::Prop => CoreTerm::Prop,
            CoreTerm::NatLit(n) => CoreTerm::NatLit(*n),

            CoreTerm::Var(i) => {
                if *i >= cutoff {
                    CoreTerm::Var(*i + amount)
                } else {
                    CoreTerm::Var(*i)
                }
            }

            CoreTerm::Lam { param_type, body } => CoreTerm::Lam {
                param_type: Box::new(param_type.shift_by(cutoff, amount)),
                body: Box::new(body.shift_by(cutoff + 1, amount)),
            },

            CoreTerm::Pi { param_type, body } => CoreTerm::Pi {
                param_type: Box::new(param_type.shift_by(cutoff, amount)),
                body: Box::new(body.shift_by(cutoff + 1, amount)),
            },

            CoreTerm::App { func, arg } => CoreTerm::App {
                func: Box::new(func.shift_by(cutoff, amount)),
                arg: Box::new(arg.shift_by(cutoff, amount)),
            },

            CoreTerm::Let { bound_type, value, body } => CoreTerm::Let {
                bound_type: Box::new(bound_type.shift_by(cutoff, amount)),
                value: Box::new(value.shift_by(cutoff, amount)),
                body: Box::new(body.shift_by(cutoff + 1, amount)),
            },

            CoreTerm::Const { name, levels } => CoreTerm::Const {
                name: name.clone(),
                levels: levels.clone(),
            },

            CoreTerm::Constructor { type_name, ctor_name, args } => CoreTerm::Constructor {
                type_name: type_name.clone(),
                ctor_name: ctor_name.clone(),
                args: args.iter().map(|a| a.shift_by(cutoff, amount)).collect(),
            },

            CoreTerm::Recursor { type_name, args } => CoreTerm::Recursor {
                type_name: type_name.clone(),
                args: args.iter().map(|a| a.shift_by(cutoff, amount)).collect(),
            },
        }
    }

    /// Canonical byte serialization for hashing.
    /// Deterministic: same term → same bytes → same hash.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.write_bytes(&mut buf);
        buf
    }

    /// Hash of this term (canonical name).
    pub fn term_hash(&self) -> Hash32 {
        hash::H(&self.to_bytes())
    }

    fn write_bytes(&self, buf: &mut Vec<u8>) {
        match self {
            CoreTerm::Type(u) => {
                buf.push(0x01);
                buf.extend_from_slice(&u.to_le_bytes());
            }
            CoreTerm::Prop => {
                buf.push(0x02);
            }
            CoreTerm::Var(i) => {
                buf.push(0x03);
                buf.extend_from_slice(&(*i as u64).to_le_bytes());
            }
            CoreTerm::Lam { param_type, body } => {
                buf.push(0x04);
                param_type.write_bytes(buf);
                body.write_bytes(buf);
            }
            CoreTerm::Pi { param_type, body } => {
                buf.push(0x05);
                param_type.write_bytes(buf);
                body.write_bytes(buf);
            }
            CoreTerm::App { func, arg } => {
                buf.push(0x06);
                func.write_bytes(buf);
                arg.write_bytes(buf);
            }
            CoreTerm::Let { bound_type, value, body } => {
                buf.push(0x07);
                bound_type.write_bytes(buf);
                value.write_bytes(buf);
                body.write_bytes(buf);
            }
            CoreTerm::Const { name, levels } => {
                buf.push(0x08);
                buf.extend_from_slice(&(name.len() as u32).to_le_bytes());
                buf.extend_from_slice(name.as_bytes());
                buf.extend_from_slice(&(levels.len() as u32).to_le_bytes());
                for l in levels {
                    buf.extend_from_slice(&l.to_le_bytes());
                }
            }
            CoreTerm::NatLit(n) => {
                buf.push(0x09);
                buf.extend_from_slice(&n.to_le_bytes());
            }
            CoreTerm::Constructor { type_name, ctor_name, args } => {
                buf.push(0x0A);
                buf.extend_from_slice(&(type_name.len() as u32).to_le_bytes());
                buf.extend_from_slice(type_name.as_bytes());
                buf.extend_from_slice(&(ctor_name.len() as u32).to_le_bytes());
                buf.extend_from_slice(ctor_name.as_bytes());
                buf.extend_from_slice(&(args.len() as u32).to_le_bytes());
                for a in args {
                    a.write_bytes(buf);
                }
            }
            CoreTerm::Recursor { type_name, args } => {
                buf.push(0x0B);
                buf.extend_from_slice(&(type_name.len() as u32).to_le_bytes());
                buf.extend_from_slice(type_name.as_bytes());
                buf.extend_from_slice(&(args.len() as u32).to_le_bytes());
                for a in args {
                    a.write_bytes(buf);
                }
            }
        }
    }

    /// Parse a CoreTerm from its canonical byte representation.
    pub fn from_bytes(data: &[u8]) -> Option<(CoreTerm, usize)> {
        if data.is_empty() {
            return None;
        }

        let tag = data[0];
        let rest = &data[1..];

        match tag {
            0x01 => {
                // Type(u32)
                if rest.len() < 4 { return None; }
                let u = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]);
                Some((CoreTerm::Type(u), 5))
            }
            0x02 => {
                // Prop
                Some((CoreTerm::Prop, 1))
            }
            0x03 => {
                // Var(usize)
                if rest.len() < 8 { return None; }
                let i = u64::from_le_bytes(rest[..8].try_into().ok()?) as usize;
                Some((CoreTerm::Var(i), 9))
            }
            0x04 => {
                // Lam { param_type, body }
                let (param_type, n1) = CoreTerm::from_bytes(rest)?;
                let (body, n2) = CoreTerm::from_bytes(&rest[n1..])?;
                Some((CoreTerm::Lam {
                    param_type: Box::new(param_type),
                    body: Box::new(body),
                }, 1 + n1 + n2))
            }
            0x05 => {
                // Pi { param_type, body }
                let (param_type, n1) = CoreTerm::from_bytes(rest)?;
                let (body, n2) = CoreTerm::from_bytes(&rest[n1..])?;
                Some((CoreTerm::Pi {
                    param_type: Box::new(param_type),
                    body: Box::new(body),
                }, 1 + n1 + n2))
            }
            0x06 => {
                // App { func, arg }
                let (func, n1) = CoreTerm::from_bytes(rest)?;
                let (arg, n2) = CoreTerm::from_bytes(&rest[n1..])?;
                Some((CoreTerm::App {
                    func: Box::new(func),
                    arg: Box::new(arg),
                }, 1 + n1 + n2))
            }
            0x07 => {
                // Let { bound_type, value, body }
                let (bound_type, n1) = CoreTerm::from_bytes(rest)?;
                let (value, n2) = CoreTerm::from_bytes(&rest[n1..])?;
                let (body, n3) = CoreTerm::from_bytes(&rest[n1 + n2..])?;
                Some((CoreTerm::Let {
                    bound_type: Box::new(bound_type),
                    value: Box::new(value),
                    body: Box::new(body),
                }, 1 + n1 + n2 + n3))
            }
            0x08 => {
                // Const { name, levels }
                if rest.len() < 4 { return None; }
                let name_len = u32::from_le_bytes(rest[..4].try_into().ok()?) as usize;
                if rest.len() < 4 + name_len + 4 { return None; }
                let name = String::from_utf8(rest[4..4 + name_len].to_vec()).ok()?;
                let levels_start = 4 + name_len;
                let levels_count = u32::from_le_bytes(
                    rest[levels_start..levels_start + 4].try_into().ok()?
                ) as usize;
                let mut levels = Vec::with_capacity(levels_count);
                let mut pos = levels_start + 4;
                for _ in 0..levels_count {
                    if rest.len() < pos + 4 { return None; }
                    levels.push(u32::from_le_bytes(rest[pos..pos + 4].try_into().ok()?));
                    pos += 4;
                }
                Some((CoreTerm::Const { name, levels }, 1 + pos))
            }
            0x09 => {
                // NatLit(u64)
                if rest.len() < 8 { return None; }
                let n = u64::from_le_bytes(rest[..8].try_into().ok()?);
                Some((CoreTerm::NatLit(n), 9))
            }
            0x0A => {
                // Constructor { type_name, ctor_name, args }
                if rest.len() < 4 { return None; }
                let tn_len = u32::from_le_bytes(rest[..4].try_into().ok()?) as usize;
                if rest.len() < 4 + tn_len + 4 { return None; }
                let type_name = String::from_utf8(rest[4..4 + tn_len].to_vec()).ok()?;
                let cn_start = 4 + tn_len;
                let cn_len = u32::from_le_bytes(
                    rest[cn_start..cn_start + 4].try_into().ok()?
                ) as usize;
                if rest.len() < cn_start + 4 + cn_len + 4 { return None; }
                let ctor_name = String::from_utf8(
                    rest[cn_start + 4..cn_start + 4 + cn_len].to_vec()
                ).ok()?;
                let args_start = cn_start + 4 + cn_len;
                let args_count = u32::from_le_bytes(
                    rest[args_start..args_start + 4].try_into().ok()?
                ) as usize;
                let mut args = Vec::with_capacity(args_count);
                let mut pos = args_start + 4;
                for _ in 0..args_count {
                    let (arg, n) = CoreTerm::from_bytes(&rest[pos..])?;
                    args.push(arg);
                    pos += n;
                }
                Some((CoreTerm::Constructor { type_name, ctor_name, args }, 1 + pos))
            }
            0x0B => {
                // Recursor { type_name, args }
                if rest.len() < 4 { return None; }
                let tn_len = u32::from_le_bytes(rest[..4].try_into().ok()?) as usize;
                if rest.len() < 4 + tn_len + 4 { return None; }
                let type_name = String::from_utf8(rest[4..4 + tn_len].to_vec()).ok()?;
                let args_start = 4 + tn_len;
                let args_count = u32::from_le_bytes(
                    rest[args_start..args_start + 4].try_into().ok()?
                ) as usize;
                let mut args = Vec::with_capacity(args_count);
                let mut pos = args_start + 4;
                for _ in 0..args_count {
                    let (arg, n) = CoreTerm::from_bytes(&rest[pos..])?;
                    args.push(arg);
                    pos += n;
                }
                Some((CoreTerm::Recursor { type_name, args }, 1 + pos))
            }
            _ => None,
        }
    }

    /// Size of this term (node count, for enumeration ordering).
    pub fn size(&self) -> usize {
        match self {
            CoreTerm::Type(_) | CoreTerm::Prop | CoreTerm::Var(_) | CoreTerm::NatLit(_) => 1,
            CoreTerm::Lam { param_type, body } => 1 + param_type.size() + body.size(),
            CoreTerm::Pi { param_type, body } => 1 + param_type.size() + body.size(),
            CoreTerm::App { func, arg } => 1 + func.size() + arg.size(),
            CoreTerm::Let { bound_type, value, body } => {
                1 + bound_type.size() + value.size() + body.size()
            }
            CoreTerm::Const { .. } => 1,
            CoreTerm::Constructor { args, .. } => 1 + args.iter().map(|a| a.size()).sum::<usize>(),
            CoreTerm::Recursor { args, .. } => 1 + args.iter().map(|a| a.size()).sum::<usize>(),
        }
    }

    /// Is this term a value (not reducible further)?
    pub fn is_value(&self) -> bool {
        match self {
            CoreTerm::Type(_) | CoreTerm::Prop | CoreTerm::NatLit(_) => true,
            CoreTerm::Lam { .. } => true, // lambdas are values
            CoreTerm::Pi { .. } => true,  // Pi types are values
            CoreTerm::Var(_) => true,     // free variables are values (stuck)
            CoreTerm::Const { .. } => true, // unfolded constants are values
            CoreTerm::Constructor { args, .. } => args.iter().all(|a| a.is_value()),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_context() {
        let ctx = CoreCtx::new();
        assert_eq!(ctx.len(), 0);
        assert!(ctx.is_empty());
        assert!(ctx.lookup(0).is_none());
    }

    #[test]
    fn context_push_lookup() {
        let mut ctx = CoreCtx::new();
        ctx.push(Some("n".into()), CoreTerm::Const { name: "Nat".into(), levels: vec![] });
        ctx.push(Some("h".into()), CoreTerm::Prop);

        assert_eq!(ctx.len(), 2);
        // Index 0 = most recent = "h"
        assert_eq!(ctx.lookup(0).unwrap().name.as_deref(), Some("h"));
        // Index 1 = older = "n"
        assert_eq!(ctx.lookup(1).unwrap().name.as_deref(), Some("n"));
    }

    #[test]
    fn term_size() {
        assert_eq!(CoreTerm::Prop.size(), 1);
        assert_eq!(CoreTerm::NatLit(42).size(), 1);
        assert_eq!(CoreTerm::Var(0).size(), 1);

        let app = CoreTerm::App {
            func: Box::new(CoreTerm::Var(0)),
            arg: Box::new(CoreTerm::NatLit(1)),
        };
        assert_eq!(app.size(), 3);

        let lam = CoreTerm::Lam {
            param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
            body: Box::new(app),
        };
        assert_eq!(lam.size(), 5);
    }

    #[test]
    fn byte_roundtrip_simple() {
        let terms = vec![
            CoreTerm::Type(0),
            CoreTerm::Prop,
            CoreTerm::Var(3),
            CoreTerm::NatLit(42),
            CoreTerm::Const { name: "Nat".into(), levels: vec![0] },
        ];

        for term in &terms {
            let bytes = term.to_bytes();
            let (parsed, len) = CoreTerm::from_bytes(&bytes).unwrap();
            assert_eq!(&parsed, term, "roundtrip failed for {:?}", term);
            assert_eq!(len, bytes.len());
        }
    }

    #[test]
    fn byte_roundtrip_compound() {
        let term = CoreTerm::Lam {
            param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
            body: Box::new(CoreTerm::App {
                func: Box::new(CoreTerm::Var(1)),
                arg: Box::new(CoreTerm::Var(0)),
            }),
        };

        let bytes = term.to_bytes();
        let (parsed, len) = CoreTerm::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, term);
        assert_eq!(len, bytes.len());
    }

    #[test]
    fn byte_roundtrip_constructor() {
        let term = CoreTerm::Constructor {
            type_name: "Nat".into(),
            ctor_name: "succ".into(),
            args: vec![CoreTerm::NatLit(0)],
        };

        let bytes = term.to_bytes();
        let (parsed, len) = CoreTerm::from_bytes(&bytes).unwrap();
        assert_eq!(parsed, term);
        assert_eq!(len, bytes.len());
    }

    #[test]
    fn substitute_var() {
        // Var(0)[0 := NatLit(5)] = NatLit(5)
        let result = CoreTerm::Var(0).substitute(0, &CoreTerm::NatLit(5));
        assert_eq!(result, CoreTerm::NatLit(5));

        // Var(1)[0 := NatLit(5)] = Var(0) (shifted down)
        let result = CoreTerm::Var(1).substitute(0, &CoreTerm::NatLit(5));
        assert_eq!(result, CoreTerm::Var(0));

        // Var(0)[1 := NatLit(5)] = Var(0) (not affected)
        let result = CoreTerm::Var(0).substitute(1, &CoreTerm::NatLit(5));
        assert_eq!(result, CoreTerm::Var(0));
    }

    #[test]
    fn substitute_under_binder() {
        // (λ Nat. Var(1))[0 := NatLit(5)]
        // Var(1) inside the lambda refers to the outer variable at index 0
        // After substitution: λ Nat. NatLit(5)
        let lam = CoreTerm::Lam {
            param_type: Box::new(CoreTerm::Const { name: "Nat".into(), levels: vec![] }),
            body: Box::new(CoreTerm::Var(1)), // refers to outer var 0
        };
        let result = lam.substitute(0, &CoreTerm::NatLit(5));
        match result {
            CoreTerm::Lam { body, .. } => {
                assert_eq!(*body, CoreTerm::NatLit(5));
            }
            _ => panic!("expected Lam"),
        }
    }

    #[test]
    fn term_hash_deterministic() {
        let t1 = CoreTerm::App {
            func: Box::new(CoreTerm::Var(0)),
            arg: Box::new(CoreTerm::NatLit(42)),
        };
        let t2 = CoreTerm::App {
            func: Box::new(CoreTerm::Var(0)),
            arg: Box::new(CoreTerm::NatLit(42)),
        };
        assert_eq!(t1.term_hash(), t2.term_hash());

        let t3 = CoreTerm::App {
            func: Box::new(CoreTerm::Var(0)),
            arg: Box::new(CoreTerm::NatLit(43)),
        };
        assert_ne!(t1.term_hash(), t3.term_hash());
    }

    #[test]
    fn is_value() {
        assert!(CoreTerm::Prop.is_value());
        assert!(CoreTerm::NatLit(0).is_value());
        assert!(CoreTerm::Var(0).is_value());
        assert!(CoreTerm::Lam {
            param_type: Box::new(CoreTerm::Prop),
            body: Box::new(CoreTerm::Var(0)),
        }.is_value());

        // App is not a value (potentially reducible)
        assert!(!CoreTerm::App {
            func: Box::new(CoreTerm::Lam {
                param_type: Box::new(CoreTerm::Prop),
                body: Box::new(CoreTerm::Var(0)),
            }),
            arg: Box::new(CoreTerm::NatLit(0)),
        }.is_value());
    }

    #[test]
    fn empty_env() {
        let env = CoreEnv::new();
        assert_eq!(env.len(), 0);
        assert!(env.is_empty());
        assert!(env.lookup("Nat").is_none());
    }

    #[test]
    fn env_add_lookup() {
        let mut env = CoreEnv::new();
        env.add_def(CoreDef {
            name: "Nat.zero".into(),
            ty: CoreTerm::Const { name: "Nat".into(), levels: vec![] },
            value: Some(CoreTerm::NatLit(0)),
            universe_params: vec![],
        });

        assert_eq!(env.len(), 1);
        let def = env.lookup("Nat.zero").unwrap();
        assert_eq!(def.value, Some(CoreTerm::NatLit(0)));
    }
}
