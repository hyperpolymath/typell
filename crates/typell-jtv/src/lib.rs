// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-JtV Bridge — maps Julia-the-Viper's Harvard architecture types into TypeLL.
//!
//! JtV separates Control Language (Turing-complete) from Data Language
//! (Total/provably halting). This maps to TypeLL's effect system:
//! - **Data expressions**: Pure (no effects, guaranteed to halt)
//! - **Control statements**: May have IO, State, Diverge effects
//!
//! JtV also has 7 number systems with a coercion lattice, mapped to
//! named numeric types in TypeLL.

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
