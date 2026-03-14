// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! BetLang-specific typing rules for TypeLL.
//!
//! Implements ternary logic compatibility and distribution type rules.

use typell_core::types::Effect;

/// Check that a bet form produces a valid result type.
///
/// In `(bet A B C)`, all three branches must have compatible types.
/// The result type is the join of the three branch types.
pub fn check_bet_branches(
    a_type: &str,
    b_type: &str,
    c_type: &str,
) -> Result<String, String> {
    if a_type == b_type && b_type == c_type {
        Ok(a_type.to_string())
    } else {
        Err(format!(
            "bet branches must have compatible types: got {}, {}, {}",
            a_type, b_type, c_type
        ))
    }
}

/// Check that a Ternary value is used correctly.
///
/// Ternary values (true/false/unknown) require explicit handling of the
/// unknown case. Using a Ternary where a Bool is expected is an error
/// unless the unknown case is explicitly handled.
pub fn check_ternary_exhaustive(handles_unknown: bool) -> Result<(), String> {
    if handles_unknown {
        Ok(())
    } else {
        Err("ternary value requires explicit handling of 'unknown' case".to_string())
    }
}

/// The effect produced by a `bet` form (non-deterministic choice).
pub fn bet_effect() -> Effect {
    Effect::Named("NonDet".to_string())
}

/// Check if a function using Dist<T> properly propagates non-determinism.
pub fn check_dist_propagation(
    fn_effects: &[Effect],
    uses_dist: bool,
) -> Result<(), String> {
    if uses_dist {
        let has_nondet = fn_effects.iter().any(|e| match e {
            Effect::Named(n) => n == "NonDet",
            _ => false,
        });
        if !has_nondet {
            return Err(
                "function uses Dist<T> but does not declare NonDet effect".to_string()
            );
        }
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bet_same_types_ok() {
        assert!(check_bet_branches("Int", "Int", "Int").is_ok());
    }

    #[test]
    fn test_bet_different_types_err() {
        assert!(check_bet_branches("Int", "String", "Int").is_err());
    }

    #[test]
    fn test_ternary_exhaustive_ok() {
        assert!(check_ternary_exhaustive(true).is_ok());
    }

    #[test]
    fn test_ternary_not_exhaustive_err() {
        assert!(check_ternary_exhaustive(false).is_err());
    }

    #[test]
    fn test_dist_propagation_ok() {
        let effects = vec![Effect::Named("NonDet".to_string())];
        assert!(check_dist_propagation(&effects, true).is_ok());
    }

    #[test]
    fn test_dist_propagation_missing_err() {
        let effects = vec![Effect::IO];
        assert!(check_dist_propagation(&effects, true).is_err());
    }
}
