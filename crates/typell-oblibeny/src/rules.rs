// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Oblibeny-specific typing rules for TypeLL.
//!
//! Implements reversibility checking and constrained-form validation.
//! In constrained form, only reversible operations (swap, incr/decr,
//! xor_assign) are permitted — irreversible operations (division,
//! overwrite) are rejected.

/// Check that an operation is reversible (valid in constrained form).
///
/// Reversible operations in Oblibeny:
/// - `swap(a, b)` — swap two values
/// - `incr(a, delta)` / `decr(a, delta)` — reversible increment/decrement
/// - `xor_assign(a, b)` — XOR assignment (self-inverse)
///
/// Irreversible operations (rejected):
/// - Plain assignment (overwrites previous value)
/// - Division (lossy for integers)
/// - Modulo (lossy)
pub fn check_reversible(op: &str) -> Result<(), String> {
    match op {
        "swap" | "incr" | "decr" | "xor_assign" | "checkpoint" | "trace"
        | "assert_invariant" => Ok(()),
        "assign" | "div" | "mod" => Err(format!(
            "operation '{}' is not reversible and cannot be used in constrained form",
            op
        )),
        _ => Err(format!("unknown operation: {}", op)),
    }
}

/// Check that a for-range loop has statically bounded iteration count.
///
/// Constrained form requires bounded loops (Turing-incomplete).
pub fn check_bounded_loop(start: i64, end: i64) -> Result<u64, String> {
    if end >= start {
        Ok((end - start) as u64)
    } else {
        Err(format!(
            "for-range loop must have end >= start: {} >= {} is false",
            end, start
        ))
    }
}

/// Check that a swap operation has compatible types.
pub fn check_swap_types(a_type: &str, b_type: &str) -> Result<(), String> {
    if a_type == b_type {
        Ok(())
    } else {
        Err(format!(
            "swap requires same types, got {} and {}",
            a_type, b_type
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
    fn test_swap_is_reversible() {
        assert!(check_reversible("swap").is_ok());
    }

    #[test]
    fn test_incr_is_reversible() {
        assert!(check_reversible("incr").is_ok());
    }

    #[test]
    fn test_assign_is_not_reversible() {
        assert!(check_reversible("assign").is_err());
    }

    #[test]
    fn test_div_is_not_reversible() {
        assert!(check_reversible("div").is_err());
    }

    #[test]
    fn test_bounded_loop_valid() {
        assert_eq!(check_bounded_loop(0, 10).unwrap(), 10);
    }

    #[test]
    fn test_bounded_loop_invalid() {
        assert!(check_bounded_loop(10, 5).is_err());
    }

    #[test]
    fn test_swap_same_types_ok() {
        assert!(check_swap_types("i64", "i64").is_ok());
    }

    #[test]
    fn test_swap_different_types_err() {
        assert!(check_swap_types("i64", "bool").is_err());
    }
}
