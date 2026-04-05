// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL-VCL Bridge — maps VCL-total's 10 type safety levels into TypeLL.
//!
//! VCL-total (Ultimate Type-Safe) supersedes VCL-DT by extending dependent
//! type checking to a full 10-level type safety hierarchy:
//!
//! ## Established Levels (1–6)
//!
//! | Level | Name                        | TypeLL Concept                    |
//! |-------|-----------------------------|-----------------------------------|
//! | 1     | Parse-time safety           | Well-formed AST                   |
//! | 2     | Schema-binding safety       | Named type resolution             |
//! | 3     | Type-compatible operations  | Unification + operator checking   |
//! | 4     | Null-safety                 | Option types, totality checking   |
//! | 5     | Injection-proof safety      | Refinement predicates             |
//! | 6     | Result-type safety          | Return type inference             |
//!
//! ## Research-Identified Levels (7–10)
//!
//! | Level | Name                        | TypeLL Concept                    |
//! |-------|-----------------------------|-----------------------------------|
//! | 7     | Cardinality safety          | Bounded quantifiers               |
//! | 8     | Effect-tracking safety      | Algebraic effects                 |
//! | 9     | Temporal safety             | Session types, state machines     |
//! | 10    | Linearity safety            | QTT bounded usage, linear types   |
//!
//! ## VCL-DT Legacy (6 mechanisms)
//!
//! The original VCL-dt++ extensions map to levels 7–10:
//! 1. **Linear types** (`CONSUME AFTER N USE`) → Level 10
//! 2. **Session types** (`WITH SESSION`) → Level 9
//! 3. **Effect systems** (`EFFECTS { Read, Write }`) → Level 8
//! 4. **Modal types** (`IN TRANSACTION`) → Level 9 (temporal)
//! 5. **Proof-carrying** (`PROOF ATTACHED`) → Level 5 (refinements)
//! 6. **QTT** (`USAGE LIMIT n`) → Level 10

#![forbid(unsafe_code)]
pub mod bridge;
pub mod levels;
pub mod rules;
