// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-Ephapax Bridge — maps Ephapax's linear/affine dual type system into TypeLL.
//!
//! Ephapax enforces two modes simultaneously:
//! - **Linear** (default): values must be used exactly once
//! - **Affine** (via `mut`): values may be used at most once (can be dropped)
//!
//! The bridge maps Ephapax's `Affinity` enum directly to TypeLL's QTT
//! discipline, with contracts (pre/post/invariant) becoming refinement
//! predicates.

pub mod bridge;
pub mod rules;
