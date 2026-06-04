// SPDX-License-Identifier: MPL-2.0
// Copyright (c) Jonathan D.A. Jewell <j.d.a.jewell@open.ac.uk>
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

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
