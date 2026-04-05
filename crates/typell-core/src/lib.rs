// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL Core — the unified type system verification kernel.
//!
//! TypeLL is to PanLL what LLVM is to Clang: the formal verification
//! substrate that multiple language frontends compile *types* into.
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────┐  ┌──────────────┐  ┌──────────────┐
//! │   Eclexia    │  │   My-Lang    │  │     VCL      │
//! │  (frontend)  │  │  (frontend)  │  │  (frontend)  │
//! └──────┬───────┘  └──────┬───────┘  └──────┬───────┘
//!        │                 │                 │
//!        ▼                 ▼                 ▼
//! ┌─────────────────────────────────────────────────┐
//! │              typell-core (this crate)            │
//! │                                                  │
//! │  types.rs      — Unified type representation     │
//! │  unify.rs      — Robinson unification + occurs   │
//! │  infer.rs      — Bidirectional type inference    │
//! │  check.rs      — Type checking coordinator       │
//! │  linear.rs     — Linear/affine usage tracking    │
//! │  effects.rs    — Effect system                   │
//! │  qtt.rs        — QTT semiring operations         │
//! │  dimensional.rs — Dimensional analysis           │
//! │  session.rs    — Session type protocols          │
//! │  proof.rs      — Proof obligation generation     │
//! │  error.rs      — Diagnostic types                │
//! └─────────────────────────────────────────────────┘
//! ```
//!
//! # Unified Type System
//!
//! Every type in every supported language is lowered to `types::Type`.
//! The kernel then applies:
//!
//! 1. **Unification** — Robinson's algorithm with occurs check
//! 2. **Inference** — bidirectional (synthesis + checking modes)
//! 3. **Linearity** — QTT-based usage tracking (0, 1, omega, n)
//! 4. **Effects** — algebraic effect tracking and validation
//! 5. **Dimensions** — SI-based dimensional analysis (from Eclexia)
//! 6. **Sessions** — protocol duality checking
//! 7. **Proofs** — obligation generation for dependent types

#![forbid(unsafe_code)]
pub mod check;
pub mod dimensional;
pub mod effects;
pub mod error;
pub mod infer;
pub mod linear;
pub mod proof;
pub mod qtt;
pub mod session;
pub mod types;
pub mod unify;

// Re-export key types for convenience.
pub use check::{CheckResult, TypeChecker};
pub use error::{Span, TypeError, TypeResult};
pub use proof::{eval_predicate, PredicateResult};
pub use types::{
    Dimension, Effect, Predicate, PrimitiveType, SessionType, Term, TermOp, Type,
    TypeDiscipline, TypeScheme, TypeVar, UnifiedType, UsageQuantifier,
};
pub use unify::{eval_term_to_i64, terms_unify, Substitution, Unifier};
