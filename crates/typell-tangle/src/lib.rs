// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-Tangle Bridge — maps Tangle's braid word types into TypeLL.
//!
//! Tangle's type system revolves around braid groups:
//! - **Word[n]**: A braid word on n strands (generators sigma_i)
//! - **Tangle[A, B]**: A morphism from strand set A to strand set B
//! - **Compose (.)**: Vertical stacking of braids
//! - **Tensor (|)**: Horizontal juxtaposition of braids
//!
//! These map naturally to TypeLL's session types: a braid word describes
//! a communication protocol where strands represent channels and crossings
//! represent message exchanges.

pub mod bridge;
pub mod rules;
