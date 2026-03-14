// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-MyLang Bridge — maps My-Lang's AI effect types into TypeLL.
//!
//! My-Lang has a distinctive AI effect type system:
//! - **AI<T>**: Values produced by AI inference (non-deterministic, uncertain)
//! - **Effect<T>**: General side-effect wrapper
//! - Standard types (Int, Float, String, Bool, records, tuples)
//!
//! The AI<T> type maps to a TypeLL effect annotation, tracking which
//! values in the program are AI-generated and may require validation.

pub mod bridge;
pub mod rules;
