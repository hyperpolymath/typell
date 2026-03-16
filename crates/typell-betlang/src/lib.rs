// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-BetLang Bridge — maps BetLang's probabilistic types into TypeLL.
//!
//! BetLang's unique type features:
//! - **Ternary**: Three-valued logic (true/false/unknown)
//! - **Dist<T>**: Distribution over type T (probabilistic values)
//! - **Bet form**: `(bet A B C)` — random selection, modelled as an effect
//!
//! Probabilistic types map to TypeLL's effect system (non-determinism effect)
//! and the Ternary type maps to a refined enum type.

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
