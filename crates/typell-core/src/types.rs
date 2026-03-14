// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Unified type representation for the TypeLL verification kernel.
//!
//! This module defines the core type language that unifies:
//! - Hindley-Milner polymorphism (inference)
//! - Dependent types (value-indexed types)
//! - Linear/affine types (usage tracking via QTT)
//! - Effect types (declared side effects)
//! - Session types (protocol compliance)
//! - Dimensional types (physical unit tracking from Eclexia)
//! - Refinement types (predicate-narrowed base types)
//!
//! The representation mirrors `TypeLLModel.res` in PanLL, ensuring
//! wire-compatible serialization over the JSON-RPC bridge.

use serde::{Deserialize, Serialize};
use std::fmt;

// ============================================================================
// Type Variables
// ============================================================================

/// A type variable, identified by a unique integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TypeVar(pub u32);

impl fmt::Display for TypeVar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{}", self.0)
    }
}

// ============================================================================
// Usage Quantifiers (QTT)
// ============================================================================

/// Quantitative Type Theory usage quantifier.
///
/// Tracks how many times a value may be consumed:
/// - `Zero`: erased at runtime, proof-only witness
/// - `One`: exactly once (strict linear)
/// - `Omega`: unrestricted (standard functional programming)
/// - `Bounded(n)`: at most n times (generalised affine)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum UsageQuantifier {
    /// Zero uses — erased at runtime (compile-time witness only).
    Zero,
    /// Exactly one use — strict linear consumption.
    One,
    /// Unrestricted — unlimited uses.
    Omega,
    /// Bounded — at most n uses (generalised affine).
    Bounded(u64),
}

impl UsageQuantifier {
    /// Check whether `self` is compatible with `other` (i.e., `self <= other`
    /// in the QTT semiring ordering).
    pub fn compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Zero, _) => true,
            (Self::One, Self::One | Self::Omega) => true,
            (Self::Bounded(n), Self::Bounded(m)) => n <= m,
            (Self::Bounded(n), Self::Omega) => *n > 0,
            (Self::Omega, Self::Omega) => true,
            _ => false,
        }
    }

    /// Combine two usage quantifiers (addition in the QTT semiring).
    pub fn add(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Zero, q) | (q, Self::Zero) => *q,
            (Self::One, Self::One) => Self::Bounded(2),
            (Self::One, Self::Bounded(n)) | (Self::Bounded(n), Self::One) => {
                Self::Bounded(n + 1)
            }
            (Self::Bounded(a), Self::Bounded(b)) => Self::Bounded(a + b),
            _ => Self::Omega,
        }
    }
}

impl fmt::Display for UsageQuantifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Zero => write!(f, "0"),
            Self::One => write!(f, "1"),
            Self::Omega => write!(f, "w"),
            Self::Bounded(n) => write!(f, "{n}"),
        }
    }
}

// ============================================================================
// Type Discipline Modes
// ============================================================================

/// Module-level type discipline declaration.
///
/// Determines the default type system behaviour within a scope.
/// Mirrors `typeDiscipline` in `TypeLLModel.res`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TypeDiscipline {
    /// Affine by default — values used at most once, may be discarded (Rust-like).
    Affine,
    /// Linear — values used exactly once. Strict resource tracking.
    Linear,
    /// Dependent — full dependent types with value-level computation in types.
    Dependent,
    /// Refined — base types narrowed by predicate constraints (Liquid types).
    Refined,
    /// Unrestricted — no usage tracking, standard types. Escape hatch.
    Unrestricted,
}

impl Default for TypeDiscipline {
    fn default() -> Self {
        Self::Affine
    }
}

impl fmt::Display for TypeDiscipline {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Affine => write!(f, "affine"),
            Self::Linear => write!(f, "linear"),
            Self::Dependent => write!(f, "dependent"),
            Self::Refined => write!(f, "refined"),
            Self::Unrestricted => write!(f, "unrestricted"),
        }
    }
}

// ============================================================================
// Dimensional Analysis (ported from Eclexia)
// ============================================================================

/// A physical dimension represented as a vector of SI base unit exponents
/// plus extensions for economic and environmental quantities.
///
/// Ported from `eclexia-ast::dimension::Dimension`. Dimensions form an
/// abelian group under multiplication (exponent addition).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Dimension {
    /// Mass (kilogram) exponent.
    pub mass: i8,
    /// Length (metre) exponent.
    pub length: i8,
    /// Time (second) exponent.
    pub time: i8,
    /// Electric current (ampere) exponent.
    pub current: i8,
    /// Temperature (kelvin) exponent.
    pub temperature: i8,
    /// Amount of substance (mole) exponent.
    pub amount: i8,
    /// Luminous intensity (candela) exponent.
    pub luminosity: i8,
    /// Currency (abstract monetary unit) exponent.
    pub money: i8,
    /// Carbon dioxide equivalent (CO2e) exponent.
    pub carbon: i8,
    /// Information (bit) exponent.
    pub information: i8,
}

impl Dimension {
    /// Dimensionless (all exponents zero).
    pub const fn dimensionless() -> Self {
        Self {
            mass: 0, length: 0, time: 0, current: 0,
            temperature: 0, amount: 0, luminosity: 0,
            money: 0, carbon: 0, information: 0,
        }
    }

    /// Check whether all exponents are zero.
    pub const fn is_dimensionless(&self) -> bool {
        self.mass == 0 && self.length == 0 && self.time == 0
            && self.current == 0 && self.temperature == 0
            && self.amount == 0 && self.luminosity == 0
            && self.money == 0 && self.carbon == 0
            && self.information == 0
    }

    // --- Common base dimensions ---

    pub const fn mass() -> Self { Self { mass: 1, ..Self::dimensionless() } }
    pub const fn length() -> Self { Self { length: 1, ..Self::dimensionless() } }
    pub const fn time() -> Self { Self { time: 1, ..Self::dimensionless() } }
    pub const fn current() -> Self { Self { current: 1, ..Self::dimensionless() } }
    pub const fn temperature() -> Self { Self { temperature: 1, ..Self::dimensionless() } }
    pub const fn information() -> Self { Self { information: 1, ..Self::dimensionless() } }
    pub const fn money() -> Self { Self { money: 1, ..Self::dimensionless() } }
    pub const fn carbon() -> Self { Self { carbon: 1, ..Self::dimensionless() } }

    // --- Common derived dimensions ---

    /// Energy: kg * m^2 / s^2  (joule)
    pub const fn energy() -> Self {
        Self { mass: 1, length: 2, time: -2, ..Self::dimensionless() }
    }

    /// Power: kg * m^2 / s^3  (watt)
    pub const fn power() -> Self {
        Self { mass: 1, length: 2, time: -3, ..Self::dimensionless() }
    }

    /// Force: kg * m / s^2  (newton)
    pub const fn force() -> Self {
        Self { mass: 1, length: 1, time: -2, ..Self::dimensionless() }
    }

    /// Velocity: m / s
    pub const fn velocity() -> Self {
        Self { length: 1, time: -1, ..Self::dimensionless() }
    }

    /// Memory (alias for information — bytes = 8 bits).
    pub const fn memory() -> Self { Self::information() }

    // --- Dimension algebra ---

    /// Multiply two dimensions (add exponents).
    pub const fn multiply(&self, other: &Self) -> Self {
        Self {
            mass: self.mass + other.mass,
            length: self.length + other.length,
            time: self.time + other.time,
            current: self.current + other.current,
            temperature: self.temperature + other.temperature,
            amount: self.amount + other.amount,
            luminosity: self.luminosity + other.luminosity,
            money: self.money + other.money,
            carbon: self.carbon + other.carbon,
            information: self.information + other.information,
        }
    }

    /// Divide two dimensions (subtract exponents).
    pub const fn divide(&self, other: &Self) -> Self {
        Self {
            mass: self.mass - other.mass,
            length: self.length - other.length,
            time: self.time - other.time,
            current: self.current - other.current,
            temperature: self.temperature - other.temperature,
            amount: self.amount - other.amount,
            luminosity: self.luminosity - other.luminosity,
            money: self.money - other.money,
            carbon: self.carbon - other.carbon,
            information: self.information - other.information,
        }
    }

    /// Raise a dimension to an integer power (multiply all exponents by n).
    pub const fn pow(&self, n: i8) -> Self {
        Self {
            mass: self.mass * n,
            length: self.length * n,
            time: self.time * n,
            current: self.current * n,
            temperature: self.temperature * n,
            amount: self.amount * n,
            luminosity: self.luminosity * n,
            money: self.money * n,
            carbon: self.carbon * n,
            information: self.information * n,
        }
    }

    /// Inverse dimension (negate all exponents).
    pub const fn inverse(&self) -> Self { self.pow(-1) }
}

impl std::ops::Mul for Dimension {
    type Output = Self;
    fn mul(self, rhs: Self) -> Self { self.multiply(&rhs) }
}

impl std::ops::Div for Dimension {
    type Output = Self;
    fn div(self, rhs: Self) -> Self { self.divide(&rhs) }
}

impl fmt::Display for Dimension {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_dimensionless() {
            return write!(f, "dimensionless");
        }

        let mut pos = Vec::new();
        let mut neg = Vec::new();

        macro_rules! dim {
            ($field:ident, $sym:literal) => {
                match self.$field {
                    0 => {}
                    1 => pos.push($sym.to_string()),
                    -1 => neg.push($sym.to_string()),
                    n if n > 0 => pos.push(format!("{}^{}", $sym, n)),
                    n => neg.push(format!("{}^{}", $sym, -n)),
                }
            };
        }

        dim!(mass, "kg"); dim!(length, "m"); dim!(time, "s");
        dim!(current, "A"); dim!(temperature, "K"); dim!(amount, "mol");
        dim!(luminosity, "cd"); dim!(money, "$"); dim!(carbon, "CO2e");
        dim!(information, "bit");

        if neg.is_empty() {
            write!(f, "{}", pos.join("\u{00b7}"))
        } else if pos.is_empty() {
            write!(f, "1/{}", neg.join("\u{00b7}"))
        } else {
            write!(f, "{}/{}", pos.join("\u{00b7}"), neg.join("\u{00b7}"))
        }
    }
}

// ============================================================================
// Effects
// ============================================================================

/// An effect annotation on a type. Effects are tracked as part of the
/// type signature: `fn foo {IO, State s} (x: a) -> a @ {IO, State s}`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Effect {
    /// Pure — no side effects.
    Pure,
    /// IO — reads/writes to the outside world.
    IO,
    /// State — thread-local mutable state parameterised by type.
    State(String),
    /// Exception — may throw an exception of the given type.
    Except(String),
    /// Allocation — performs heap allocation.
    Alloc,
    /// Divergence — may not terminate.
    Diverge,
    /// Network — performs network I/O.
    Network,
    /// File system — reads/writes files.
    FileSystem,
    /// Custom named effect (for user-defined effect handlers).
    Named(String),
}

impl fmt::Display for Effect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pure => write!(f, "Pure"),
            Self::IO => write!(f, "IO"),
            Self::State(s) => write!(f, "State {s}"),
            Self::Except(e) => write!(f, "Except {e}"),
            Self::Alloc => write!(f, "Alloc"),
            Self::Diverge => write!(f, "Diverge"),
            Self::Network => write!(f, "Network"),
            Self::FileSystem => write!(f, "FileSystem"),
            Self::Named(n) => write!(f, "{n}"),
        }
    }
}

// ============================================================================
// Terms (for dependent type indices)
// ============================================================================

/// A term that can appear inside a dependent type as an index.
///
/// For example, in `Vec n a`, `n` is a `Term::Var`. In `Vec (n + m) a`,
/// the index is `Term::BinOp(Add, Var("n"), Var("m"))`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Term {
    /// Integer literal.
    Lit(i64),
    /// Variable reference.
    Var(String),
    /// Binary operation on terms.
    BinOp {
        op: TermOp,
        lhs: Box<Term>,
        rhs: Box<Term>,
    },
    /// Application of a function to arguments.
    App {
        func: String,
        args: Vec<Term>,
    },
}

/// Binary operations on terms (for dependent type index arithmetic).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TermOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Lit(n) => write!(f, "{n}"),
            Self::Var(v) => write!(f, "{v}"),
            Self::BinOp { op, lhs, rhs } => {
                let sym = match op {
                    TermOp::Add => "+",
                    TermOp::Sub => "-",
                    TermOp::Mul => "*",
                    TermOp::Div => "/",
                    TermOp::Mod => "%",
                };
                write!(f, "({lhs} {sym} {rhs})")
            }
            Self::App { func, args } => {
                write!(f, "{func}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{a}")?;
                }
                write!(f, ")")
            }
        }
    }
}

// ============================================================================
// Predicates (for refinement types)
// ============================================================================

/// A refinement predicate constraining a type.
///
/// For `{x : Int | x > 0 && x < 256}`, the predicates would be
/// `[Gt(Var("x"), Lit(0)), Lt(Var("x"), Lit(256))]`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Predicate {
    /// `term > term`
    Gt(Term, Term),
    /// `term >= term`
    Gte(Term, Term),
    /// `term < term`
    Lt(Term, Term),
    /// `term <= term`
    Lte(Term, Term),
    /// `term == term`
    Eq(Term, Term),
    /// `term != term`
    Neq(Term, Term),
    /// Logical AND of predicates.
    And(Box<Predicate>, Box<Predicate>),
    /// Logical OR of predicates.
    Or(Box<Predicate>, Box<Predicate>),
    /// Logical NOT of a predicate.
    Not(Box<Predicate>),
    /// Raw predicate expression (escape hatch for complex constraints).
    Raw(String),
}

impl fmt::Display for Predicate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Gt(a, b) => write!(f, "{a} > {b}"),
            Self::Gte(a, b) => write!(f, "{a} >= {b}"),
            Self::Lt(a, b) => write!(f, "{a} < {b}"),
            Self::Lte(a, b) => write!(f, "{a} <= {b}"),
            Self::Eq(a, b) => write!(f, "{a} == {b}"),
            Self::Neq(a, b) => write!(f, "{a} != {b}"),
            Self::And(a, b) => {
                // Parenthesise nested Or to preserve precedence.
                let left = if matches!(a.as_ref(), Predicate::Or(..)) {
                    format!("({a})")
                } else {
                    format!("{a}")
                };
                let right = if matches!(b.as_ref(), Predicate::Or(..)) {
                    format!("({b})")
                } else {
                    format!("{b}")
                };
                write!(f, "{left} && {right}")
            }
            Self::Or(a, b) => {
                // Parenthesise nested And to preserve precedence.
                let left = if matches!(a.as_ref(), Predicate::And(..)) {
                    format!("({a})")
                } else {
                    format!("{a}")
                };
                let right = if matches!(b.as_ref(), Predicate::And(..)) {
                    format!("({b})")
                } else {
                    format!("{b}")
                };
                write!(f, "{left} || {right}")
            }
            Self::Not(a) => {
                if matches!(
                    a.as_ref(),
                    Predicate::And(..) | Predicate::Or(..)
                ) {
                    write!(f, "!({a})")
                } else {
                    write!(f, "!{a}")
                }
            }
            Self::Raw(s) => write!(f, "{s}"),
        }
    }
}

// ============================================================================
// Base Types
// ============================================================================

/// Primitive base types.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveType {
    Bool,
    Int,
    I8, I16, I32, I64, I128,
    U8, U16, U32, U64, U128,
    Float, F32, F64,
    Char,
    String,
    Unit,
    Never,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool => write!(f, "Bool"),
            Self::Int => write!(f, "Int"),
            Self::I8 => write!(f, "I8"),
            Self::I16 => write!(f, "I16"),
            Self::I32 => write!(f, "I32"),
            Self::I64 => write!(f, "I64"),
            Self::I128 => write!(f, "I128"),
            Self::U8 => write!(f, "U8"),
            Self::U16 => write!(f, "U16"),
            Self::U32 => write!(f, "U32"),
            Self::U64 => write!(f, "U64"),
            Self::U128 => write!(f, "U128"),
            Self::Float => write!(f, "Float"),
            Self::F32 => write!(f, "F32"),
            Self::F64 => write!(f, "F64"),
            Self::Char => write!(f, "Char"),
            Self::String => write!(f, "String"),
            Self::Unit => write!(f, "()"),
            Self::Never => write!(f, "!"),
        }
    }
}

// ============================================================================
// The Unified Type
// ============================================================================

/// The core type representation in the TypeLL kernel.
///
/// Every type in every supported language is lowered to this representation.
/// It combines base type information with QTT usage tracking, effect
/// annotations, dependent indices, dimensional analysis, and refinement
/// predicates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Type {
    /// A primitive type (Bool, Int, Float, etc.).
    Primitive(PrimitiveType),

    /// A type variable (for polymorphism / unification).
    Var(TypeVar),

    /// A named type constructor with type arguments.
    /// e.g., `Vec<Int>` is `Named("Vec", [Primitive(Int)])`.
    Named {
        name: String,
        args: Vec<Type>,
    },

    /// A function type.
    Function {
        params: Vec<Type>,
        ret: Box<Type>,
        effects: Vec<Effect>,
    },

    /// A tuple type.
    Tuple(Vec<Type>),

    /// An array type with optional dependent length.
    Array {
        elem: Box<Type>,
        length: Option<Term>,
    },

    /// A universally quantified type (forall).
    ForAll {
        vars: Vec<String>,
        body: Box<Type>,
    },

    /// A resource type with dimensional annotation (from Eclexia).
    /// `Resource<Energy>` carries a `Dimension` for compile-time unit checking.
    Resource {
        base: Box<Type>,
        dimension: Dimension,
    },

    /// A refined type: base type narrowed by predicates.
    /// `{x : Int | x > 0}` is `Refined(Primitive(Int), [Gt(Var("x"), Lit(0))])`.
    Refined {
        base: Box<Type>,
        predicates: Vec<Predicate>,
    },

    /// A dependent function type (Pi type).
    /// `(x : A) -> B(x)` where the return type depends on the argument value.
    Pi {
        param_name: String,
        param_type: Box<Type>,
        body: Box<Type>,
    },

    /// A dependent pair type (Sigma type).
    /// `(x : A, B(x))` where the second component's type depends on the first.
    Sigma {
        fst_name: String,
        fst_type: Box<Type>,
        snd_type: Box<Type>,
    },

    /// A session type (protocol specification).
    Session(SessionType),

    /// The universal supertype — all types are subtypes of Top.
    Top,

    /// The empty type — subtype of all types (exceptions, impossible branches).
    Bottom,

    /// An error sentinel (permits error recovery without cascading).
    Error,
}

impl Type {
    /// Check whether this type is a type variable.
    pub fn is_var(&self) -> bool {
        matches!(self, Self::Var(_))
    }

    /// Check whether this type contains any type variables.
    pub fn has_vars(&self) -> bool {
        match self {
            Self::Var(_) => true,
            Self::Primitive(_) | Self::Error | Self::Top | Self::Bottom => false,
            Self::Named { args, .. } => args.iter().any(|a| a.has_vars()),
            Self::Function { params, ret, .. } => {
                params.iter().any(|p| p.has_vars()) || ret.has_vars()
            }
            Self::Tuple(elems) => elems.iter().any(|e| e.has_vars()),
            Self::Array { elem, .. } => elem.has_vars(),
            Self::ForAll { body, .. } => body.has_vars(),
            Self::Resource { base, .. } => base.has_vars(),
            Self::Refined { base, .. } => base.has_vars(),
            Self::Pi { param_type, body, .. } => param_type.has_vars() || body.has_vars(),
            Self::Sigma { fst_type, snd_type, .. } => {
                fst_type.has_vars() || snd_type.has_vars()
            }
            Self::Session(s) => s.has_vars(),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Primitive(p) => write!(f, "{p}"),
            Self::Var(v) => write!(f, "{v}"),
            Self::Named { name, args } if args.is_empty() => write!(f, "{name}"),
            Self::Named { name, args } => {
                write!(f, "{name}<")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{a}")?;
                }
                write!(f, ">")
            }
            Self::Function { params, ret, effects } => {
                write!(f, "(")?;
                for (i, p) in params.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{p}")?;
                }
                write!(f, ")")?;
                if !effects.is_empty() {
                    write!(f, " {{")?;
                    for (i, e) in effects.iter().enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{e}")?;
                    }
                    write!(f, "}}")?;
                }
                write!(f, " -> {ret}")
            }
            Self::Tuple(elems) => {
                write!(f, "(")?;
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{e}")?;
                }
                write!(f, ")")
            }
            Self::Array { elem, length: None } => write!(f, "[{elem}]"),
            Self::Array { elem, length: Some(n) } => write!(f, "[{elem}; {n}]"),
            Self::ForAll { vars, body } => {
                write!(f, "forall")?;
                for v in vars { write!(f, " {v}")?; }
                write!(f, ". {body}")
            }
            Self::Resource { base, dimension } => {
                write!(f, "Resource<{base}, {dimension}>")
            }
            Self::Refined { base, predicates } => {
                write!(f, "{{x : {base} | ")?;
                for (i, p) in predicates.iter().enumerate() {
                    if i > 0 { write!(f, " && ")?; }
                    write!(f, "{p}")?;
                }
                write!(f, "}}")
            }
            Self::Pi { param_name, param_type, body } => {
                write!(f, "({param_name} : {param_type}) -> {body}")
            }
            Self::Sigma { fst_name, fst_type, snd_type } => {
                write!(f, "({fst_name} : {fst_type}, {snd_type})")
            }
            Self::Session(s) => write!(f, "{s}"),
            Self::Top => write!(f, "\u{22a4}"),
            Self::Bottom => write!(f, "\u{22a5}"),
            Self::Error => write!(f, "<error>"),
        }
    }
}

// ============================================================================
// Session Types (protocol specifications)
// ============================================================================

/// A session type describes a communication protocol.
///
/// Session types ensure that two communicating parties follow a compatible
/// protocol. They are checked via duality: if one side sends, the other
/// must receive.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionType {
    /// Send a value of the given type, then continue with the rest.
    Send(Box<Type>, Box<SessionType>),
    /// Receive a value of the given type, then continue with the rest.
    Recv(Box<Type>, Box<SessionType>),
    /// Offer a choice between branches (external choice).
    Offer(Vec<(String, SessionType)>),
    /// Select one of the offered branches (internal choice).
    Select(Vec<(String, SessionType)>),
    /// End of protocol.
    End,
    /// Recursive session (mu-binder).
    Rec(String, Box<SessionType>),
    /// Reference to a recursive binder.
    RecVar(String),
}

impl SessionType {
    /// Check whether this session type contains any type variables.
    ///
    /// Session types contain embedded `Type` values in Send/Recv positions,
    /// which may themselves contain type variables. This enables polymorphic
    /// session protocols like `Send<a, Recv<b, End>>`.
    pub fn has_vars(&self) -> bool {
        match self {
            Self::Send(ty, cont) | Self::Recv(ty, cont) => {
                ty.has_vars() || cont.has_vars()
            }
            Self::Offer(branches) | Self::Select(branches) => {
                branches.iter().any(|(_, s)| s.has_vars())
            }
            Self::End => false,
            Self::Rec(_, body) => body.has_vars(),
            Self::RecVar(_) => false,
        }
    }

    /// Collect free type variables from the embedded types.
    pub fn free_type_vars(&self) -> Vec<TypeVar> {
        match self {
            Self::Send(ty, cont) | Self::Recv(ty, cont) => {
                let mut vars = collect_type_vars(ty);
                vars.extend(cont.free_type_vars());
                vars
            }
            Self::Offer(branches) | Self::Select(branches) => {
                branches.iter().flat_map(|(_, s)| s.free_type_vars()).collect()
            }
            Self::End | Self::RecVar(_) => Vec::new(),
            Self::Rec(_, body) => body.free_type_vars(),
        }
    }
}

impl fmt::Display for SessionType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Send(ty, cont) => write!(f, "!{ty}.{cont}"),
            Self::Recv(ty, cont) => write!(f, "?{ty}.{cont}"),
            Self::Offer(branches) => {
                write!(f, "&{{")?;
                for (i, (label, s)) in branches.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{label}: {s}")?;
                }
                write!(f, "}}")
            }
            Self::Select(branches) => {
                write!(f, "+{{")?;
                for (i, (label, s)) in branches.iter().enumerate() {
                    if i > 0 { write!(f, ", ")?; }
                    write!(f, "{label}: {s}")?;
                }
                write!(f, "}}")
            }
            Self::End => write!(f, "end"),
            Self::Rec(var, body) => write!(f, "\u{03bc}{var}.{body}"),
            Self::RecVar(var) => write!(f, "{var}"),
        }
    }
}

/// Collect all type variables from a Type.
fn collect_type_vars(ty: &Type) -> Vec<TypeVar> {
    match ty {
        Type::Var(v) => vec![*v],
        Type::Named { args, .. } => args.iter().flat_map(collect_type_vars).collect(),
        Type::Function { params, ret, .. } => {
            let mut vars: Vec<TypeVar> = params.iter().flat_map(collect_type_vars).collect();
            vars.extend(collect_type_vars(ret));
            vars
        }
        Type::Tuple(elems) => elems.iter().flat_map(collect_type_vars).collect(),
        Type::Array { elem, .. } => collect_type_vars(elem),
        Type::ForAll { body, .. } => collect_type_vars(body),
        Type::Resource { base, .. } | Type::Refined { base, .. } => collect_type_vars(base),
        Type::Pi { param_type, body, .. } => {
            let mut vars = collect_type_vars(param_type);
            vars.extend(collect_type_vars(body));
            vars
        }
        Type::Sigma { fst_type, snd_type, .. } => {
            let mut vars = collect_type_vars(fst_type);
            vars.extend(collect_type_vars(snd_type));
            vars
        }
        Type::Session(s) => s.free_type_vars(),
        _ => Vec::new(),
    }
}

// ============================================================================
// Unified Type (the full picture including annotations)
// ============================================================================

/// A fully annotated type in the unified system.
///
/// This combines a base `Type` with all the metadata that the TypeLL kernel
/// tracks: usage quantifier, discipline, effects, dependent indices, and
/// refinement predicates. Mirrors `unifiedTypeExpr` in `TypeLLModel.res`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedType {
    /// The base type expression.
    pub base: Type,
    /// Usage quantifier (QTT: 0, 1, omega, or bounded n).
    pub usage: UsageQuantifier,
    /// Type discipline in effect.
    pub discipline: TypeDiscipline,
    /// Dependent type indices (value-level terms appearing in the type).
    pub dependent_indices: Vec<Term>,
    /// Effect annotations.
    pub effects: Vec<Effect>,
    /// Refinement predicates.
    pub refinements: Vec<Predicate>,
}

impl UnifiedType {
    /// Create an unrestricted unified type with no annotations.
    pub fn simple(base: Type) -> Self {
        Self {
            base,
            usage: UsageQuantifier::Omega,
            discipline: TypeDiscipline::Unrestricted,
            dependent_indices: Vec::new(),
            effects: Vec::new(),
            refinements: Vec::new(),
        }
    }

    /// Create a linear unified type.
    pub fn linear(base: Type) -> Self {
        Self {
            base,
            usage: UsageQuantifier::One,
            discipline: TypeDiscipline::Linear,
            dependent_indices: Vec::new(),
            effects: Vec::new(),
            refinements: Vec::new(),
        }
    }

    /// Create an affine unified type.
    pub fn affine(base: Type) -> Self {
        Self {
            base,
            usage: UsageQuantifier::One,
            discipline: TypeDiscipline::Affine,
            dependent_indices: Vec::new(),
            effects: Vec::new(),
            refinements: Vec::new(),
        }
    }
}

// ============================================================================
// Type Scheme (polymorphic types)
// ============================================================================

/// A polymorphic type scheme: a type with universally quantified variables.
///
/// `forall a b. a -> b -> (a, b)` is represented as
/// `TypeScheme { vars: ["a", "b"], body: Function(...) }`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypeScheme {
    /// Quantified type variable names.
    pub vars: Vec<String>,
    /// The body type.
    pub body: UnifiedType,
}

impl TypeScheme {
    /// Create a monomorphic (non-polymorphic) type scheme.
    pub fn mono(ty: UnifiedType) -> Self {
        Self {
            vars: Vec::new(),
            body: ty,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_energy_division_gives_power() {
        let energy = Dimension::energy();
        let time = Dimension::time();
        let power = energy / time;
        assert_eq!(power, Dimension::power());
    }

    #[test]
    fn test_dimension_display() {
        assert_eq!(Dimension::energy().to_string(), "kg\u{00b7}m^2/s^2");
        assert_eq!(Dimension::power().to_string(), "kg\u{00b7}m^2/s^3");
        assert_eq!(Dimension::velocity().to_string(), "m/s");
        assert_eq!(Dimension::dimensionless().to_string(), "dimensionless");
    }

    #[test]
    fn test_usage_quantifier_compatibility() {
        assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::One));
        assert!(UsageQuantifier::One.compatible_with(&UsageQuantifier::Omega));
        assert!(!UsageQuantifier::Omega.compatible_with(&UsageQuantifier::One));
    }

    #[test]
    fn test_usage_quantifier_addition() {
        assert_eq!(
            UsageQuantifier::One.add(&UsageQuantifier::One),
            UsageQuantifier::Bounded(2)
        );
        assert_eq!(
            UsageQuantifier::Zero.add(&UsageQuantifier::One),
            UsageQuantifier::One
        );
    }

    #[test]
    fn test_unified_type_simple() {
        let ty = UnifiedType::simple(Type::Primitive(PrimitiveType::Int));
        assert_eq!(ty.usage, UsageQuantifier::Omega);
        assert_eq!(ty.discipline, TypeDiscipline::Unrestricted);
    }

    // ========================================================================
    // Session type variable tracking tests
    // ========================================================================

    #[test]
    fn test_session_type_has_no_vars() {
        let s = Type::Session(SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        ));
        assert!(!s.has_vars());
    }

    #[test]
    fn test_session_type_has_vars_in_send() {
        let s = Type::Session(SessionType::Send(
            Box::new(Type::Var(TypeVar(0))),
            Box::new(SessionType::End),
        ));
        assert!(s.has_vars());
    }

    #[test]
    fn test_session_type_has_vars_in_offer() {
        let s = Type::Session(SessionType::Offer(vec![
            ("buy".to_string(), SessionType::Send(
                Box::new(Type::Var(TypeVar(1))),
                Box::new(SessionType::End),
            )),
        ]));
        assert!(s.has_vars());
    }

    // ========================================================================
    // Predicate Display tests
    // ========================================================================

    #[test]
    fn test_predicate_display_gt() {
        let p = Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0));
        assert_eq!(p.to_string(), "x > 0");
    }

    #[test]
    fn test_predicate_display_and() {
        let p = Predicate::And(
            Box::new(Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0))),
            Box::new(Predicate::Lt(Term::Var("x".to_string()), Term::Lit(256))),
        );
        assert_eq!(p.to_string(), "x > 0 && x < 256");
    }

    #[test]
    fn test_predicate_display_not_compound() {
        let p = Predicate::Not(Box::new(Predicate::Or(
            Box::new(Predicate::Eq(Term::Var("a".to_string()), Term::Lit(1))),
            Box::new(Predicate::Eq(Term::Var("b".to_string()), Term::Lit(2))),
        )));
        assert_eq!(p.to_string(), "!(a == 1 || b == 2)");
    }

    #[test]
    fn test_predicate_display_raw() {
        let p = Predicate::Raw("custom constraint".to_string());
        assert_eq!(p.to_string(), "custom constraint");
    }

    // ========================================================================
    // SessionType Display tests
    // ========================================================================

    #[test]
    fn test_session_display_send_end() {
        let s = SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::End),
        );
        assert_eq!(s.to_string(), "!Int.end");
    }

    #[test]
    fn test_session_display_recv_then_send() {
        let s = SessionType::Recv(
            Box::new(Type::Primitive(PrimitiveType::String)),
            Box::new(SessionType::Send(
                Box::new(Type::Primitive(PrimitiveType::Bool)),
                Box::new(SessionType::End),
            )),
        );
        assert_eq!(s.to_string(), "?String.!Bool.end");
    }

    #[test]
    fn test_session_display_offer() {
        let s = SessionType::Offer(vec![
            ("accept".to_string(), SessionType::End),
            ("reject".to_string(), SessionType::End),
        ]);
        assert_eq!(s.to_string(), "&{accept: end, reject: end}");
    }

    #[test]
    fn test_session_display_rec() {
        let s = SessionType::Rec(
            "X".to_string(),
            Box::new(SessionType::Send(
                Box::new(Type::Primitive(PrimitiveType::Int)),
                Box::new(SessionType::RecVar("X".to_string())),
            )),
        );
        assert_eq!(s.to_string(), "\u{03bc}X.!Int.X");
    }

    // ========================================================================
    // Top and Bottom type tests
    // ========================================================================

    #[test]
    fn test_top_has_no_vars() {
        assert!(!Type::Top.has_vars());
    }

    #[test]
    fn test_bottom_has_no_vars() {
        assert!(!Type::Bottom.has_vars());
    }

    #[test]
    fn test_top_display() {
        assert_eq!(Type::Top.to_string(), "\u{22a4}");
    }

    #[test]
    fn test_bottom_display() {
        assert_eq!(Type::Bottom.to_string(), "\u{22a5}");
    }

    #[test]
    fn test_session_free_type_vars() {
        let s = SessionType::Send(
            Box::new(Type::Var(TypeVar(0))),
            Box::new(SessionType::Recv(
                Box::new(Type::Var(TypeVar(1))),
                Box::new(SessionType::End),
            )),
        );
        let vars = s.free_type_vars();
        assert_eq!(vars.len(), 2);
        assert!(vars.contains(&TypeVar(0)));
        assert!(vars.contains(&TypeVar(1)));
    }
}
