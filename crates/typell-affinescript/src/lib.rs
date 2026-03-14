// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-AffineScript Bridge — maps AffineScript's rich type system into TypeLL.
//!
//! AffineScript has the richest type system of the nextgen languages:
//! - **QTT (Quantitative Type Theory)**: 0/1/omega usage quantifiers
//! - **Refinement types**: Predicate-narrowed base types
//! - **Row polymorphism**: Extensible records and variants
//! - **Algebraic effects**: User-defined effect handlers
//! - **Dependent arrows**: Value-dependent function types
//! - **Ownership modifiers**: own/ref/mut
//!
//! # Architecture
//!
//! ```text
//! AffineScript AST (affinescript/lib/types.ml, ast.ml)
//!     |
//!     v
//! bridge.rs — Convert AffineScript types to TypeLL types
//!     |
//!     v
//! typell-core — Unified type checking
//!     |
//!     v
//! rules.rs — AffineScript-specific QTT and refinement rules
//! ```

pub mod bridge;
pub mod rules;
