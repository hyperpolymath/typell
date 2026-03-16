// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-Phronesis Bridge — maps Phronesis's policy types into TypeLL.
//!
//! Phronesis is a policy DSL. Its "types" are policy-level constructs:
//! - **Policy declarations**: condition -> action with metadata
//! - **Literal types**: integer, float, string, boolean, ip_address, datetime
//! - **Actions**: execute, report, reject, accept
//!
//! These map to TypeLL's refined types (conditions as predicates) and
//! effect system (actions as effects).

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
