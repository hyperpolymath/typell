// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Ephapax-specific typing rules for TypeLL.
//!
//! Ephapax enforces strict linearity by default: every binding must be
//! consumed exactly once. The `mut` keyword relaxes this to affine (at
//! most once). This module encodes those rules.

use typell_core::types::{TypeDiscipline, UsageQuantifier};

use crate::bridge::EphapaxAffinity;

/// Check that a linear binding has been consumed exactly once.
///
/// Returns an error if the binding's usage count does not match its affinity.
pub fn check_consumption(
    name: &str,
    affinity: &EphapaxAffinity,
    use_count: u64,
) -> Result<(), String> {
    match affinity {
        EphapaxAffinity::Linear => {
            if use_count != 1 {
                Err(format!(
                    "linear binding `{}` must be used exactly once, used {} times",
                    name, use_count
                ))
            } else {
                Ok(())
            }
        }
        EphapaxAffinity::Affine => {
            if use_count > 1 {
                Err(format!(
                    "affine binding `{}` may be used at most once, used {} times",
                    name, use_count
                ))
            } else {
                Ok(())
            }
        }
    }
}

/// Check that a contract clause (pre/post/invariant) is well-formed.
///
/// Contract clauses in Ephapax map to refinement predicates in TypeLL.
pub fn check_contract_clause(kind: &str) -> Result<(), String> {
    match kind {
        "pre" | "post" | "invariant" => Ok(()),
        other => Err(format!("unknown contract kind: {}", other)),
    }
}

/// Determine if a function marked `#[safe]` should be checked more strictly.
///
/// Safe functions in Ephapax enforce that all parameters are consumed and
/// no arena-allocated values escape.
pub fn safe_function_discipline() -> (TypeDiscipline, UsageQuantifier) {
    (TypeDiscipline::Linear, UsageQuantifier::One)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_consumed_once_ok() {
        assert!(check_consumption("x", &EphapaxAffinity::Linear, 1).is_ok());
    }

    #[test]
    fn test_linear_consumed_zero_err() {
        assert!(check_consumption("x", &EphapaxAffinity::Linear, 0).is_err());
    }

    #[test]
    fn test_affine_consumed_zero_ok() {
        assert!(check_consumption("x", &EphapaxAffinity::Affine, 0).is_ok());
    }

    #[test]
    fn test_affine_consumed_twice_err() {
        assert!(check_consumption("x", &EphapaxAffinity::Affine, 2).is_err());
    }

    #[test]
    fn test_contract_clause_valid() {
        assert!(check_contract_clause("pre").is_ok());
        assert!(check_contract_clause("post").is_ok());
        assert!(check_contract_clause("invariant").is_ok());
    }

    #[test]
    fn test_contract_clause_invalid() {
        assert!(check_contract_clause("unknown").is_err());
    }
}
