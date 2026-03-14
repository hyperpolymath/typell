// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Error-Lang-specific typing rules for TypeLL.
//!
//! Implements stability score calculation and superposition type rules.

use crate::bridge::StabilityFactor;

/// Calculate the stability score from a set of stability factors.
///
/// Starts at 100 and subtracts penalties for each factor.
pub fn calculate_stability(factors: &[StabilityFactor]) -> u32 {
    let base: i64 = 100;
    let penalty: i64 = factors.iter().map(factor_penalty).sum();
    (base + penalty).clamp(0, 100) as u32
}

/// Calculate the penalty for a single stability factor.
fn factor_penalty(factor: &StabilityFactor) -> i64 {
    match factor {
        StabilityFactor::MutableState { mutations, readers } => {
            -(10 * *mutations as i64 + 5 * *readers as i64)
        }
        StabilityFactor::TypeInstability { reassignments } => {
            -(15 * *reassignments as i64)
        }
        StabilityFactor::NullPropagation { depth } => {
            -(20 * *depth as i64)
        }
        StabilityFactor::GlobalState { mutations, dependencies } => {
            -(30 * *mutations as i64 + 5 * *dependencies as i64)
        }
        StabilityFactor::UnhandledError { paths } => {
            -(25 * *paths as i64)
        }
    }
}

/// Check that a gutter block properly captures error tokens.
///
/// Gutter blocks are Error-Lang's error recovery mechanism. They must
/// be marked as either recovered (tokens were meaningfully processed)
/// or unrecovered (tokens were discarded).
pub fn check_gutter_block(recovered: bool, token_count: usize) -> Result<(), String> {
    if !recovered && token_count > 0 {
        Err(format!(
            "gutter block discarded {} tokens without recovery",
            token_count
        ))
    } else {
        Ok(())
    }
}

/// Check if a ternary expression (a ? b : c) has compatible branch types.
pub fn check_ternary_branches(
    then_type: &str,
    else_type: &str,
) -> Result<(), String> {
    if then_type == else_type {
        Ok(())
    } else {
        Err(format!(
            "ternary branches must have same type: '{}' vs '{}'",
            then_type, else_type
        ))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stability_no_factors() {
        assert_eq!(calculate_stability(&[]), 100);
    }

    #[test]
    fn test_stability_with_mutations() {
        let factors = vec![StabilityFactor::MutableState {
            mutations: 3,
            readers: 2,
        }];
        // 100 - (10*3 + 5*2) = 100 - 40 = 60
        assert_eq!(calculate_stability(&factors), 60);
    }

    #[test]
    fn test_stability_clamped_at_zero() {
        let factors = vec![StabilityFactor::GlobalState {
            mutations: 10,
            dependencies: 10,
        }];
        // 100 - (30*10 + 5*10) = 100 - 350 = 0 (clamped)
        assert_eq!(calculate_stability(&factors), 0);
    }

    #[test]
    fn test_gutter_recovered_ok() {
        assert!(check_gutter_block(true, 5).is_ok());
    }

    #[test]
    fn test_gutter_unrecovered_err() {
        assert!(check_gutter_block(false, 5).is_err());
    }

    #[test]
    fn test_gutter_unrecovered_empty_ok() {
        assert!(check_gutter_block(false, 0).is_ok());
    }

    #[test]
    fn test_ternary_same_types_ok() {
        assert!(check_ternary_branches("Int", "Int").is_ok());
    }

    #[test]
    fn test_ternary_different_types_err() {
        assert!(check_ternary_branches("Int", "String").is_err());
    }
}
