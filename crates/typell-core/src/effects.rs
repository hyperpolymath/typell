// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Effect system for the TypeLL kernel.
//!
//! Tracks and validates effects (IO, State, Except, etc.) as part of
//! the type system. Effects appear in function signatures:
//!
//! ```text
//! fn readFile {IO, Except IOError} (path: String) -> String @ {IO, Except IOError}
//! ```
//!
//! ## Design
//!
//! TypeLL uses an algebraic effect system with row polymorphism:
//! - Effects are declared as part of the function type
//! - Effect handlers eliminate effects from the row
//! - Unhandled effects bubble up (must appear in the caller's signature)
//! - Pure functions have an empty effect row
//!
//! ## Current Status
//!
//! This is a working foundation with set-based effect tracking and
//! basic compatibility checking. Full row-polymorphic effect unification
//! with effect variables is marked as TODO.

use crate::error::{Span, TypeError, TypeResult};
use crate::types::Effect;
use std::collections::HashSet;

/// An effect row — the set of effects a computation may perform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectRow {
    /// Concrete effects in this row.
    pub effects: HashSet<String>,
    /// Whether this row is open (can contain additional effects via a row variable).
    /// When true, the row has an implicit "...rest" variable.
    pub open: bool,
}

impl EffectRow {
    /// Create an empty (pure) effect row.
    pub fn pure() -> Self {
        Self {
            effects: HashSet::new(),
            open: false,
        }
    }

    /// Create a closed row from effects.
    pub fn closed(effects: Vec<Effect>) -> Self {
        Self {
            effects: effects.iter().map(|e| e.to_string()).collect(),
            open: false,
        }
    }

    /// Create an open row (for effect-polymorphic functions).
    pub fn open(effects: Vec<Effect>) -> Self {
        Self {
            effects: effects.iter().map(|e| e.to_string()).collect(),
            open: true,
        }
    }

    /// Check if this row is pure (no effects).
    pub fn is_pure(&self) -> bool {
        self.effects.is_empty() && !self.open
    }

    /// Check whether this row is a subrow of another.
    ///
    /// `self <= other` means every effect in `self` is also in `other`.
    pub fn is_subrow_of(&self, other: &EffectRow) -> bool {
        if other.open {
            // Open rows accept any effects
            true
        } else {
            self.effects.is_subset(&other.effects)
        }
    }

    /// Merge two effect rows (union).
    pub fn merge(&self, other: &EffectRow) -> Self {
        Self {
            effects: self.effects.union(&other.effects).cloned().collect(),
            open: self.open || other.open,
        }
    }

    /// Remove handled effects from the row.
    pub fn handle(&self, handled: &[Effect]) -> Self {
        let handled_names: HashSet<String> =
            handled.iter().map(|e| e.to_string()).collect();
        Self {
            effects: self.effects.difference(&handled_names).cloned().collect(),
            open: self.open,
        }
    }
}

/// Check that a function's discovered effects are compatible with its
/// declared effect signature.
pub fn check_effects(
    declared: &[Effect],
    discovered: &[Effect],
    span: Span,
) -> TypeResult<Vec<Effect>> {
    let declared_row = EffectRow::closed(declared.to_vec());
    let mut undeclared = Vec::new();

    for effect in discovered {
        let name = effect.to_string();
        if !declared_row.effects.contains(&name) {
            undeclared.push(effect.clone());
        }
    }

    if undeclared.is_empty() {
        Ok(Vec::new())
    } else {
        // Return the first undeclared effect as an error
        Err(TypeError::UndeclaredEffect {
            span,
            effect: undeclared[0].clone(),
            hint: Some(format!(
                "add {{{}}} to the function's effect signature",
                undeclared.iter().map(|e| e.to_string()).collect::<Vec<_>>().join(", ")
            )),
        })
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_row() {
        let row = EffectRow::pure();
        assert!(row.is_pure());
    }

    #[test]
    fn test_subrow_closed() {
        let small = EffectRow::closed(vec![Effect::IO]);
        let big = EffectRow::closed(vec![Effect::IO, Effect::Alloc]);
        assert!(small.is_subrow_of(&big));
        assert!(!big.is_subrow_of(&small));
    }

    #[test]
    fn test_subrow_open() {
        let any = EffectRow::open(vec![]);
        let small = EffectRow::closed(vec![Effect::IO, Effect::Network]);
        assert!(small.is_subrow_of(&any));
    }

    #[test]
    fn test_handle_removes_effects() {
        let row = EffectRow::closed(vec![Effect::IO, Effect::Alloc, Effect::Network]);
        let handled = row.handle(&[Effect::IO]);
        assert!(!handled.effects.contains("IO"));
        assert!(handled.effects.contains("Alloc"));
        assert!(handled.effects.contains("Network"));
    }

    #[test]
    fn test_merge_rows() {
        let a = EffectRow::closed(vec![Effect::IO]);
        let b = EffectRow::closed(vec![Effect::Alloc]);
        let merged = a.merge(&b);
        assert!(merged.effects.contains("IO"));
        assert!(merged.effects.contains("Alloc"));
    }

    #[test]
    fn test_check_effects_ok() {
        let declared = vec![Effect::IO, Effect::Alloc];
        let discovered = vec![Effect::IO];
        assert!(check_effects(&declared, &discovered, Span::synthetic()).is_ok());
    }

    #[test]
    fn test_check_effects_undeclared() {
        let declared = vec![Effect::IO];
        let discovered = vec![Effect::IO, Effect::Network];
        assert!(check_effects(&declared, &discovered, Span::synthetic()).is_err());
    }
}
