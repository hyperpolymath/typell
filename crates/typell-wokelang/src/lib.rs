// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-WokeLang Bridge — maps WokeLang's consent-gated types and
//! dimensional analysis (units of measure) into TypeLL.
//!
//! WokeLang's unique type features:
//! - **Consent gates**: `only if okay "permission" { ... }` — types are
//!   gated by runtime consent checks, modelled as effects
//! - **Units of measure**: `remember x = 5 measured in meters` — tracked
//!   via TypeLL's dimensional analysis (from Eclexia)
//! - **Emotional annotations**: `@emote` tags — metadata, not type-level

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
