// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-VQL Bridge — maps VQL-dt++ query types into TypeLL.
//!
//! VQL-dt++ extends VeriSimDB's query language with six type-theoretic
//! extensions that map directly to TypeLL concepts:
//!
//! 1. **Linear types**: CONSUME AFTER N USE → QTT bounded usage
//! 2. **Session types**: WITH SESSION protocol → SessionType
//! 3. **Effect systems**: EFFECTS { Read, Write, .. } → Effect annotations
//! 4. **Modal types**: IN TRANSACTION state → state-dependent types
//! 5. **Proof-carrying**: PROOF ATTACHED theorem → refinement predicates
//! 6. **QTT**: USAGE LIMIT n → UsageQuantifier::Bounded(n)

#![forbid(unsafe_code)]
pub mod bridge;
pub mod rules;
