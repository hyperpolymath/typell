// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-Oblibeny Bridge — maps Oblibeny's constrained/reversible types into TypeLL.
//!
//! Oblibeny has a dual-form architecture:
//! - **Factory Form**: Turing-complete metaprogramming (unrestricted)
//! - **Constrained Form**: Turing-incomplete, reversible, accountable
//!
//! The constrained form's reversibility (swap, incr/decr, xor_assign) maps
//! to TypeLL's linear type discipline. The accountability traces (TTrace)
//! map to an audit effect.

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
