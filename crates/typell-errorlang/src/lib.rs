// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-ErrorLang Bridge — maps Error-Lang's superposition types into TypeLL.
//!
//! Error-Lang has a unique stability system where program behaviour changes
//! across runs. Key type features:
//! - **Stability score**: 0-100 rating affecting runtime behaviour
//! - **Superposition types**: Values may be in multiple states simultaneously
//! - **Gutter blocks**: Error recovery regions with token capture
//! - **Stability factors**: Mutable state, type instability, null propagation
//!
//! These map to TypeLL's effect system (non-determinism, instability effects)
//! and refinement types (stability score as a predicate).

pub mod bridge;
pub mod rules;
