// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Phronesis-specific typing rules for TypeLL.
//!
//! Implements policy priority ordering and metadata validation.

/// Check that policy priority is valid (non-negative).
pub fn check_priority(priority: i32) -> Result<(), String> {
    if priority >= 0 {
        Ok(())
    } else {
        Err(format!("policy priority must be non-negative, got {}", priority))
    }
}

/// Check that a policy's expiry is valid.
pub fn check_expiry(expires: &str) -> Result<(), String> {
    match expires {
        "never" => Ok(()),
        s if s.len() >= 10 => Ok(()), // Rough datetime format check
        _ => Err(format!("invalid expiry: {}", expires)),
    }
}

/// Check that a comparison operator is valid for the given literal type.
pub fn check_comparison_valid(op: &str, lit_type: &str) -> Result<(), String> {
    match (op, lit_type) {
        ("eq" | "neq", _) => Ok(()),
        ("gt" | "gte" | "lt" | "lte", "integer" | "float" | "datetime") => Ok(()),
        ("in", "string" | "ip_address") => Ok(()),
        (op, ty) => Err(format!(
            "comparison '{}' not valid for type '{}'",
            op, ty
        )),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_priority() {
        assert!(check_priority(0).is_ok());
        assert!(check_priority(100).is_ok());
    }

    #[test]
    fn test_invalid_priority() {
        assert!(check_priority(-1).is_err());
    }

    #[test]
    fn test_never_expiry() {
        assert!(check_expiry("never").is_ok());
    }

    #[test]
    fn test_comparison_eq_always_valid() {
        assert!(check_comparison_valid("eq", "string").is_ok());
        assert!(check_comparison_valid("eq", "integer").is_ok());
    }

    #[test]
    fn test_comparison_gt_numeric() {
        assert!(check_comparison_valid("gt", "integer").is_ok());
        assert!(check_comparison_valid("gt", "string").is_err());
    }
}
