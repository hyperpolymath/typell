// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! WokeLang-specific typing rules for TypeLL.
//!
//! Implements consent gate checking and unit-of-measure validation.

use typell_core::types::{Dimension, Effect};

/// Check that a consent-gated block has the required permission effect.
pub fn check_consent_gate(
    required_permission: &str,
    available_effects: &[Effect],
) -> Result<(), String> {
    let consent_effect = format!("Consent:{}", required_permission);
    if available_effects.iter().any(|e| match e {
        Effect::Named(n) => n == &consent_effect,
        _ => false,
    }) {
        Ok(())
    } else {
        Err(format!(
            "consent gate requires permission '{}' but it is not in scope",
            required_permission
        ))
    }
}

/// Check that a measured binary operation has compatible dimensions.
///
/// WokeLang's `measured in` annotations produce Resource types. Addition
/// and subtraction require matching dimensions; multiplication and division
/// produce derived dimensions.
pub fn check_measured_op(op: &str, lhs_dim: &Dimension, rhs_dim: &Dimension) -> Result<Dimension, String> {
    match op {
        "+" | "-" => {
            if lhs_dim == rhs_dim {
                Ok(*lhs_dim)
            } else {
                Err(format!(
                    "cannot {} values with dimensions {} and {}",
                    op, lhs_dim, rhs_dim
                ))
            }
        }
        "*" => Ok(lhs_dim.multiply(rhs_dim)),
        "/" => Ok(lhs_dim.divide(rhs_dim)),
        _ => Err(format!("unsupported measured operation: {}", op)),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consent_gate_present() {
        let effects = vec![Effect::Named("Consent:camera".to_string())];
        assert!(check_consent_gate("camera", &effects).is_ok());
    }

    #[test]
    fn test_consent_gate_missing() {
        let effects = vec![Effect::IO];
        assert!(check_consent_gate("camera", &effects).is_err());
    }

    #[test]
    fn test_measured_add_same_dim() {
        let m = Dimension::length();
        assert!(check_measured_op("+", &m, &m).is_ok());
    }

    #[test]
    fn test_measured_add_diff_dim() {
        let m = Dimension::length();
        let s = Dimension::time();
        assert!(check_measured_op("+", &m, &s).is_err());
    }

    #[test]
    fn test_measured_div_gives_velocity() {
        let m = Dimension::length();
        let s = Dimension::time();
        let result = check_measured_op("/", &m, &s).unwrap();
        assert_eq!(result, Dimension::velocity());
    }
}
