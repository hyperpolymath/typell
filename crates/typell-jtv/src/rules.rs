// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! JtV-specific typing rules for TypeLL.
//!
//! Implements the number system coercion lattice and Harvard architecture
//! enforcement (Data functions must be pure/total).

use crate::bridge::{JtvPurity, JtvType};

/// Check if a numeric coercion is valid according to JtV's coercion lattice.
///
/// ```text
///          Complex
///         /       \
///     Float    Rational
///       |      /
///      Int ---+
///     / | \
///  Hex Binary Symbolic
/// ```
pub fn check_coercion(from: &JtvType, to: &JtvType) -> bool {
    match (from, to) {
        // Same type always coerces
        (a, b) if std::mem::discriminant(a) == std::mem::discriminant(b) => true,
        // Int promotes to Float, Rational, Complex
        (JtvType::Int, JtvType::Float)
        | (JtvType::Int, JtvType::Rational)
        | (JtvType::Int, JtvType::Complex) => true,
        // Hex and Binary are Int representations
        (JtvType::Hex, JtvType::Int) | (JtvType::Binary, JtvType::Int) => true,
        // Float promotes to Complex
        (JtvType::Float, JtvType::Complex) => true,
        // Any matches everything (inference placeholder)
        (JtvType::Any, _) | (_, JtvType::Any) => true,
        _ => false,
    }
}

/// Determine the result type of an addition between two JtV types.
///
/// Follows the coercion lattice: the result is the join (least upper bound)
/// of the two operand types.
pub fn addition_result(lhs: &JtvType, rhs: &JtvType) -> Option<JtvType> {
    match (lhs, rhs) {
        (JtvType::Int, JtvType::Int) => Some(JtvType::Int),
        (JtvType::Float, JtvType::Float) => Some(JtvType::Float),
        (JtvType::Rational, JtvType::Rational) => Some(JtvType::Rational),
        (JtvType::Complex, JtvType::Complex) => Some(JtvType::Complex),
        (JtvType::String, JtvType::String) => Some(JtvType::String),
        // Coercion results
        (JtvType::Int, JtvType::Float) | (JtvType::Float, JtvType::Int) => Some(JtvType::Float),
        (JtvType::Int, JtvType::Rational) | (JtvType::Rational, JtvType::Int) => {
            Some(JtvType::Rational)
        }
        (JtvType::Int, JtvType::Complex) | (JtvType::Complex, JtvType::Int) => {
            Some(JtvType::Complex)
        }
        (JtvType::Float, JtvType::Complex) | (JtvType::Complex, JtvType::Float) => {
            Some(JtvType::Complex)
        }
        (JtvType::Hex, JtvType::Int) | (JtvType::Int, JtvType::Hex) => Some(JtvType::Int),
        (JtvType::Binary, JtvType::Int) | (JtvType::Int, JtvType::Binary) => Some(JtvType::Int),
        _ => None,
    }
}

/// Check that a function marked as Pure (Data) has no control-flow effects.
///
/// Pure Data functions in JtV are guaranteed total (they always halt).
/// They cannot contain loops, IO, or recursive calls.
pub fn check_purity(purity: &JtvPurity, has_loops: bool, has_io: bool) -> Result<(), String> {
    match purity {
        JtvPurity::Pure => {
            if has_loops {
                return Err("pure Data function cannot contain loops".to_string());
            }
            if has_io {
                return Err("pure Data function cannot perform IO".to_string());
            }
            Ok(())
        }
        JtvPurity::Impure => Ok(()),
    }
}

/// Check that a `reverse` block (v2) only contains reversible operations.
pub fn check_reversible_block(ops: &[&str]) -> Result<(), String> {
    for op in ops {
        match *op {
            "add_assign" | "sub_assign" | "if" => {}
            other => {
                return Err(format!(
                    "operation '{}' is not reversible in a reverse block",
                    other
                ));
            }
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
    fn test_int_to_float_coercion() {
        assert!(check_coercion(&JtvType::Int, &JtvType::Float));
    }

    #[test]
    fn test_float_to_int_no_coercion() {
        assert!(!check_coercion(&JtvType::Float, &JtvType::Int));
    }

    #[test]
    fn test_hex_to_int_coercion() {
        assert!(check_coercion(&JtvType::Hex, &JtvType::Int));
    }

    #[test]
    fn test_addition_int_float() {
        let result = addition_result(&JtvType::Int, &JtvType::Float);
        assert!(matches!(result, Some(JtvType::Float)));
    }

    #[test]
    fn test_pure_no_loops_ok() {
        assert!(check_purity(&JtvPurity::Pure, false, false).is_ok());
    }

    #[test]
    fn test_pure_with_loops_err() {
        assert!(check_purity(&JtvPurity::Pure, true, false).is_err());
    }

    #[test]
    fn test_impure_anything_ok() {
        assert!(check_purity(&JtvPurity::Impure, true, true).is_ok());
    }
}
