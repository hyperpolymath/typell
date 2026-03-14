// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-Eclexia Bridge — maps Eclexia's type system into TypeLL.
//!
//! Eclexia is the first dogfooding target for TypeLL. This crate bridges
//! Eclexia's existing type checker (HM inference + dimensional analysis)
//! into TypeLL's unified representation.
//!
//! # What Eclexia brings to TypeLL
//!
//! - **HM inference**: Already working in Eclexia — TypeLL generalises it
//! - **Dimensional analysis**: SI-based resource type checking, ported directly
//! - **Resource types**: Shadow prices, adaptive decisions, carbon tracking
//! - **Effect tracking**: IO, State, Alloc effects from Eclexia's effect system
//!
//! # What TypeLL adds to Eclexia
//!
//! - **Linear/affine types**: Resource consumption tracking (new for Eclexia)
//! - **Session types**: Protocol safety for Eclexia's channel-based concurrency
//! - **QTT**: Formal usage quantifiers replacing Eclexia's informal tracking
//! - **Proof obligations**: Dependent type constraints for ECHIDNA integration
//!
//! # Architecture
//!
//! ```text
//! Eclexia AST (eclexia-ast)
//!     │
//!     ▼
//! bridge.rs — Convert Eclexia types to TypeLL types
//!     │
//!     ▼
//! typell-core — Unified type checking
//!     │
//!     ▼
//! resource.rs — Eclexia-specific resource type rules
//! ```

pub mod bridge;
pub mod resource;
