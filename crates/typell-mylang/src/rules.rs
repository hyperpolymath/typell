// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! My-Lang-specific typing rules for TypeLL.
//!
//! Implements AI effect propagation rules: AI<T> values must be validated
//! before being used in non-AI contexts (type narrowing).

use typell_core::types::Effect;

/// Check that AI effect is properly propagated.
///
/// A function that uses AI<T> values must either:
/// 1. Declare the AI effect in its signature, or
/// 2. Validate the AI value (narrowing from AI<T> to T)
pub fn check_ai_propagation(
    fn_effects: &[Effect],
    uses_ai: bool,
    validates_ai: bool,
) -> Result<(), String> {
    if uses_ai && !validates_ai {
        let has_ai = fn_effects.iter().any(|e| match e {
            Effect::Named(n) => n == "AI",
            _ => false,
        });
        if !has_ai {
            return Err(
                "function uses AI<T> values but neither declares AI effect nor validates them"
                    .to_string(),
            );
        }
    }
    Ok(())
}

/// Check that AI<T> is assignable to T (implicit unwrap).
///
/// My-Lang allows `AI<T>` to be assigned to `T`, but this is tracked
/// as an AI effect on the containing function.
pub fn ai_assignable_to_base() -> bool {
    true
}

/// The effect produced by AI inference operations.
pub fn ai_effect() -> Effect {
    Effect::Named("AI".to_string())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_propagation_declared_ok() {
        let effects = vec![Effect::Named("AI".to_string())];
        assert!(check_ai_propagation(&effects, true, false).is_ok());
    }

    #[test]
    fn test_ai_propagation_validated_ok() {
        let effects = vec![];
        assert!(check_ai_propagation(&effects, true, true).is_ok());
    }

    #[test]
    fn test_ai_propagation_missing_err() {
        let effects = vec![Effect::IO];
        assert!(check_ai_propagation(&effects, true, false).is_err());
    }

    #[test]
    fn test_no_ai_no_problem() {
        let effects = vec![];
        assert!(check_ai_propagation(&effects, false, false).is_ok());
    }
}
