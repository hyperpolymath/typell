// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Linear and affine usage tracking for the TypeLL kernel.
//!
//! Implements QTT-based usage checking: each variable is declared with a
//! usage quantifier (0, 1, omega, or bounded n), and every use is counted.
//! At scope exit, the tracker verifies that each variable was used exactly
//! the right number of times.
//!
//! ## Discipline Semantics
//!
//! | Quantifier | Meaning                              | Violation on         |
//! |------------|--------------------------------------|----------------------|
//! | Zero       | Must never be used (erased witness)  | Any use              |
//! | One        | Must be used exactly once            | 0 or 2+ uses         |
//! | Omega      | No restriction                       | Never                |
//! | Bounded(n) | At most n uses                       | > n uses             |
//!
//! In affine mode, `One` means "at most once" (0 or 1 uses are both fine).
//! In linear mode, `One` means "exactly once" (must be consumed).

use crate::types::UsageQuantifier;
use std::collections::HashMap;

/// Tracks how many times each variable has been used.
#[derive(Debug, Clone)]
pub struct UsageTracker {
    /// Variable name -> (declared quantifier, actual use count).
    declarations: HashMap<String, (UsageQuantifier, u64)>,
    /// Whether we are in affine mode (at-most-once) or linear mode (exactly-once).
    affine_mode: bool,
}

/// A linearity violation.
#[derive(Debug, Clone)]
pub struct UsageViolation {
    /// The variable that was misused.
    pub variable: String,
    /// The declared usage quantifier.
    pub expected: UsageQuantifier,
    /// The actual usage quantifier observed.
    pub actual: UsageQuantifier,
    /// Human-readable message.
    pub message: String,
}

impl UsageTracker {
    /// Create a new usage tracker (linear mode by default).
    pub fn new() -> Self {
        Self {
            declarations: HashMap::new(),
            affine_mode: false,
        }
    }

    /// Create a tracker in affine mode.
    pub fn affine() -> Self {
        Self {
            declarations: HashMap::new(),
            affine_mode: true,
        }
    }

    /// Declare a variable with its expected usage quantifier.
    pub fn declare(&mut self, name: String, usage: UsageQuantifier) {
        self.declarations.insert(name, (usage, 0));
    }

    /// Record a use of a variable. Returns a violation if the use exceeds
    /// the declared quantifier.
    pub fn record_use(&mut self, name: &str) -> Option<UsageViolation> {
        if let Some((quantifier, count)) = self.declarations.get_mut(name) {
            *count += 1;
            match quantifier {
                UsageQuantifier::Zero => {
                    Some(UsageViolation {
                        variable: name.to_string(),
                        expected: UsageQuantifier::Zero,
                        actual: UsageQuantifier::Bounded(*count),
                        message: format!(
                            "variable '{}' has quantifier 0 (erased) but was used {} time(s)",
                            name, count
                        ),
                    })
                }
                UsageQuantifier::One if *count > 1 => {
                    Some(UsageViolation {
                        variable: name.to_string(),
                        expected: UsageQuantifier::One,
                        actual: UsageQuantifier::Bounded(*count),
                        message: format!(
                            "variable '{}' must be used {} but was used {} times",
                            name,
                            if self.affine_mode { "at most once" } else { "exactly once" },
                            count
                        ),
                    })
                }
                UsageQuantifier::Bounded(max) if *count > *max => {
                    Some(UsageViolation {
                        variable: name.to_string(),
                        expected: UsageQuantifier::Bounded(*max),
                        actual: UsageQuantifier::Bounded(*count),
                        message: format!(
                            "variable '{}' may be used at most {} times but was used {} times",
                            name, max, count
                        ),
                    })
                }
                _ => None,
            }
        } else {
            // Variable not tracked (unrestricted scope) — no violation
            None
        }
    }

    /// Check that all declared variables have been consumed appropriately.
    /// Called at scope exit. Returns violations for under-used variables.
    pub fn check_all_consumed(&self) -> Vec<UsageViolation> {
        let mut violations = Vec::new();
        for (name, (quantifier, count)) in &self.declarations {
            match quantifier {
                UsageQuantifier::One if *count == 0 && !self.affine_mode => {
                    violations.push(UsageViolation {
                        variable: name.clone(),
                        expected: UsageQuantifier::One,
                        actual: UsageQuantifier::Zero,
                        message: format!(
                            "linear variable '{}' was never consumed (must be used exactly once)",
                            name
                        ),
                    });
                }
                _ => {}
            }
        }
        violations
    }

    /// Get the current use count for a variable.
    pub fn use_count(&self, name: &str) -> Option<u64> {
        self.declarations.get(name).map(|(_, c)| *c)
    }
}

impl Default for UsageTracker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_variable_exactly_once() {
        let mut tracker = UsageTracker::new();
        tracker.declare("x".to_string(), UsageQuantifier::One);
        assert!(tracker.record_use("x").is_none()); // First use is fine
        assert!(tracker.record_use("x").is_some()); // Second use violates linearity
    }

    #[test]
    fn test_linear_variable_must_be_consumed() {
        let mut tracker = UsageTracker::new();
        tracker.declare("x".to_string(), UsageQuantifier::One);
        // Don't use x
        let violations = tracker.check_all_consumed();
        assert_eq!(violations.len(), 1);
    }

    #[test]
    fn test_affine_variable_may_be_unused() {
        let mut tracker = UsageTracker::affine();
        tracker.declare("x".to_string(), UsageQuantifier::One);
        // Don't use x — in affine mode this is fine
        let violations = tracker.check_all_consumed();
        assert_eq!(violations.len(), 0);
    }

    #[test]
    fn test_erased_variable_cannot_be_used() {
        let mut tracker = UsageTracker::new();
        tracker.declare("proof".to_string(), UsageQuantifier::Zero);
        assert!(tracker.record_use("proof").is_some());
    }

    #[test]
    fn test_omega_variable_unlimited() {
        let mut tracker = UsageTracker::new();
        tracker.declare("x".to_string(), UsageQuantifier::Omega);
        for _ in 0..100 {
            assert!(tracker.record_use("x").is_none());
        }
    }

    #[test]
    fn test_bounded_variable() {
        let mut tracker = UsageTracker::new();
        tracker.declare("x".to_string(), UsageQuantifier::Bounded(3));
        assert!(tracker.record_use("x").is_none());
        assert!(tracker.record_use("x").is_none());
        assert!(tracker.record_use("x").is_none());
        assert!(tracker.record_use("x").is_some()); // 4th use exceeds bound
    }
}
