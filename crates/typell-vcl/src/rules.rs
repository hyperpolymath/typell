// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! VCL-specific typing rules for TypeLL.
//!
//! Implements query extension validation and modality compatibility checks.

use crate::bridge::{VqlEffectLabel, VqlSessionProtocol, VqlTransactionState};

/// Check that a CONSUME AFTER count is valid (must be >= 1).
pub fn check_consume_after(count: u64) -> Result<(), String> {
    if count >= 1 {
        Ok(())
    } else {
        Err("CONSUME AFTER count must be >= 1".to_string())
    }
}

/// Check that a USAGE LIMIT is valid (must be >= 1).
pub fn check_usage_limit(limit: u64) -> Result<(), String> {
    if limit >= 1 {
        Ok(())
    } else {
        Err("USAGE LIMIT must be >= 1".to_string())
    }
}

/// Check that the declared effects are compatible with the session protocol.
///
/// ReadOnly sessions cannot have Write effects.
/// Mutation sessions must have Write effects.
pub fn check_session_effects_compatible(
    protocol: &VqlSessionProtocol,
    effects: &[VqlEffectLabel],
) -> Result<(), String> {
    match protocol {
        VqlSessionProtocol::ReadOnly => {
            if effects.iter().any(|e| matches!(e, VqlEffectLabel::Write)) {
                return Err(
                    "ReadOnly session cannot declare Write effect".to_string()
                );
            }
        }
        VqlSessionProtocol::Mutation => {
            if !effects.iter().any(|e| matches!(e, VqlEffectLabel::Write)) {
                return Err(
                    "Mutation session must declare Write effect".to_string()
                );
            }
        }
        _ => {}
    }
    Ok(())
}

/// Check that a transaction state transition is valid.
///
/// Valid transitions:
/// - Fresh -> Active
/// - Active -> Committed | RolledBack
/// - ReadSnapshot (standalone, no transitions)
pub fn check_transaction_transition(
    from: &VqlTransactionState,
    to: &VqlTransactionState,
) -> Result<(), String> {
    match (from, to) {
        (VqlTransactionState::Fresh, VqlTransactionState::Active) => Ok(()),
        (VqlTransactionState::Active, VqlTransactionState::Committed) => Ok(()),
        (VqlTransactionState::Active, VqlTransactionState::RolledBack) => Ok(()),
        _ => Err(format!(
            "invalid transaction transition: {:?} -> {:?}",
            from, to
        )),
    }
}

/// Check that a federate effect requires at least one non-local modality.
pub fn check_federate_requires_source(
    effects: &[VqlEffectLabel],
    has_federation_source: bool,
) -> Result<(), String> {
    let has_federate = effects
        .iter()
        .any(|e| matches!(e, VqlEffectLabel::Federate));
    if has_federate && !has_federation_source {
        Err("Federate effect requires a Federation source".to_string())
    } else {
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_consume_after_valid() {
        assert!(check_consume_after(1).is_ok());
        assert!(check_consume_after(10).is_ok());
    }

    #[test]
    fn test_consume_after_zero_invalid() {
        assert!(check_consume_after(0).is_err());
    }

    #[test]
    fn test_readonly_no_write_ok() {
        let effects = vec![VqlEffectLabel::Read];
        assert!(check_session_effects_compatible(&VqlSessionProtocol::ReadOnly, &effects).is_ok());
    }

    #[test]
    fn test_readonly_with_write_err() {
        let effects = vec![VqlEffectLabel::Read, VqlEffectLabel::Write];
        assert!(
            check_session_effects_compatible(&VqlSessionProtocol::ReadOnly, &effects).is_err()
        );
    }

    #[test]
    fn test_mutation_requires_write() {
        let effects = vec![VqlEffectLabel::Read];
        assert!(
            check_session_effects_compatible(&VqlSessionProtocol::Mutation, &effects).is_err()
        );
    }

    #[test]
    fn test_mutation_with_write_ok() {
        let effects = vec![VqlEffectLabel::Write];
        assert!(
            check_session_effects_compatible(&VqlSessionProtocol::Mutation, &effects).is_ok()
        );
    }

    #[test]
    fn test_transaction_fresh_to_active() {
        assert!(check_transaction_transition(
            &VqlTransactionState::Fresh,
            &VqlTransactionState::Active
        )
        .is_ok());
    }

    #[test]
    fn test_transaction_active_to_committed() {
        assert!(check_transaction_transition(
            &VqlTransactionState::Active,
            &VqlTransactionState::Committed
        )
        .is_ok());
    }

    #[test]
    fn test_transaction_invalid() {
        assert!(check_transaction_transition(
            &VqlTransactionState::Fresh,
            &VqlTransactionState::Committed
        )
        .is_err());
    }

    #[test]
    fn test_federate_needs_source() {
        let effects = vec![VqlEffectLabel::Federate];
        assert!(check_federate_requires_source(&effects, false).is_err());
        assert!(check_federate_requires_source(&effects, true).is_ok());
    }
}
