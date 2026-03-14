// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Quantitative Type Theory (QTT) for the TypeLL kernel.
//!
//! QTT extends the type system with usage annotations on every binding,
//! forming a semiring over quantities {0, 1, omega}. This module provides
//! the semiring operations and context-level usage operations.
//!
//! ## The QTT Semiring
//!
//! | +     | 0 | 1   | w |
//! |-------|---|-----|---|
//! | **0** | 0 | 1   | w |
//! | **1** | 1 | w   | w |
//! | **w** | w | w   | w |
//!
//! | *     | 0 | 1 | w |
//! |-------|---|---|---|
//! | **0** | 0 | 0 | 0 |
//! | **1** | 0 | 1 | w |
//! | **w** | 0 | w | w |
//!
//! ## References
//!
//! - Atkey, "Syntax and Semantics of Quantitative Type Theory" (2018)
//! - McBride, "I Got Plenty o' Nuttin'" (2016)
//! - Idris 2 — first production language with QTT

use crate::types::UsageQuantifier;
use std::collections::HashMap;

/// A QTT typing context — maps variable names to their usage quantifiers.
///
/// Operations on contexts are pointwise lifts of the semiring operations:
/// adding contexts adds corresponding entries, scaling scales all entries.
#[derive(Debug, Clone, Default)]
pub struct QttContext {
    /// Variable name -> usage quantifier.
    entries: HashMap<String, UsageQuantifier>,
}

impl QttContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Declare a variable with a usage quantifier.
    pub fn declare(&mut self, name: String, usage: UsageQuantifier) {
        self.entries.insert(name, usage);
    }

    /// Look up a variable's quantifier.
    pub fn lookup(&self, name: &str) -> Option<&UsageQuantifier> {
        self.entries.get(name)
    }

    /// Pointwise addition of two contexts.
    ///
    /// Used when two sub-expressions both use variables from the same scope
    /// (e.g., both branches of an `if`, or the two arguments of a function
    /// application).
    pub fn add(&self, other: &QttContext) -> QttContext {
        let mut result = self.clone();
        for (name, q) in &other.entries {
            let entry = result
                .entries
                .entry(name.clone())
                .or_insert(UsageQuantifier::Zero);
            *entry = entry.add(q);
        }
        result
    }

    /// Scale all entries by a quantifier.
    ///
    /// Used when a sub-expression is used under a binder with a specific
    /// quantifier (e.g., inside a lambda with usage annotation).
    pub fn scale(&self, q: &UsageQuantifier) -> QttContext {
        let mut result = QttContext::new();
        for (name, entry) in &self.entries {
            result.entries.insert(name.clone(), multiply(entry, q));
        }
        result
    }

    /// Check that this context is compatible with declared constraints.
    ///
    /// Returns a list of violations (variable name, expected, actual).
    pub fn check_against(
        &self,
        declared: &QttContext,
    ) -> Vec<(String, UsageQuantifier, UsageQuantifier)> {
        let mut violations = Vec::new();
        for (name, expected) in &declared.entries {
            let actual = self.entries.get(name).unwrap_or(&UsageQuantifier::Zero);
            if !actual.compatible_with(expected) {
                violations.push((name.clone(), *expected, *actual));
            }
        }
        violations
    }
}

/// Multiply two usage quantifiers (semiring multiplication).
fn multiply(a: &UsageQuantifier, b: &UsageQuantifier) -> UsageQuantifier {
    match (a, b) {
        (UsageQuantifier::Zero, _) | (_, UsageQuantifier::Zero) => UsageQuantifier::Zero,
        (UsageQuantifier::One, q) | (q, UsageQuantifier::One) => *q,
        _ => UsageQuantifier::Omega,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semiring_multiply() {
        assert_eq!(multiply(&UsageQuantifier::Zero, &UsageQuantifier::One), UsageQuantifier::Zero);
        assert_eq!(multiply(&UsageQuantifier::One, &UsageQuantifier::One), UsageQuantifier::One);
        assert_eq!(multiply(&UsageQuantifier::One, &UsageQuantifier::Omega), UsageQuantifier::Omega);
        assert_eq!(multiply(&UsageQuantifier::Omega, &UsageQuantifier::Omega), UsageQuantifier::Omega);
    }

    #[test]
    fn test_context_addition() {
        let mut c1 = QttContext::new();
        c1.declare("x".to_string(), UsageQuantifier::One);

        let mut c2 = QttContext::new();
        c2.declare("x".to_string(), UsageQuantifier::One);

        let combined = c1.add(&c2);
        assert_eq!(
            combined.lookup("x"),
            Some(&UsageQuantifier::Bounded(2))
        );
    }

    #[test]
    fn test_context_scaling() {
        let mut ctx = QttContext::new();
        ctx.declare("x".to_string(), UsageQuantifier::One);

        let scaled = ctx.scale(&UsageQuantifier::Zero);
        assert_eq!(scaled.lookup("x"), Some(&UsageQuantifier::Zero));
    }
}
