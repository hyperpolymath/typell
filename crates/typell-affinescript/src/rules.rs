// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! AffineScript-specific typing rules for TypeLL.
//!
//! Implements the QTT semiring rules, ownership checking, and row
//! polymorphism constraints that are specific to AffineScript.
//!
//! ## QTT Rules
//!
//! AffineScript's quantities form a semiring {0, 1, omega} where:
//! - 0 * q = 0 (erased stays erased)
//! - 1 * 1 = 1 (linear is preserved)
//! - omega * q = omega for q != 0 (unrestricted absorbs)
//!
//! ## Ownership Rules
//!
//! - `own T` — affine by default, must be consumed or explicitly dropped
//! - `ref T` — unrestricted, can be copied
//! - `mut T` — linear, must be used exactly once then released

use typell_core::types::{TypeDiscipline, UnifiedType, UsageQuantifier};

use crate::bridge::AffineQuantity;

/// Check QTT semiring multiplication: q1 * q2.
///
/// Used when a binding with quantity q1 is used in a context requiring q2.
pub fn qtt_multiply(q1: &AffineQuantity, q2: &AffineQuantity) -> AffineQuantity {
    match (q1, q2) {
        (AffineQuantity::Zero, _) | (_, AffineQuantity::Zero) => AffineQuantity::Zero,
        (AffineQuantity::One, AffineQuantity::One) => AffineQuantity::One,
        (AffineQuantity::Omega, q) | (q, AffineQuantity::Omega) => {
            match q {
                AffineQuantity::Zero => AffineQuantity::Zero,
                _ => AffineQuantity::Omega,
            }
        }
        (AffineQuantity::Var(_), _) | (_, AffineQuantity::Var(_)) => {
            AffineQuantity::Omega // Conservative fallback
        }
    }
}

/// Check QTT semiring addition: q1 + q2.
///
/// Used when merging usage from two branches (e.g., if-then-else).
pub fn qtt_add(q1: &AffineQuantity, q2: &AffineQuantity) -> AffineQuantity {
    match (q1, q2) {
        (AffineQuantity::Zero, q) | (q, AffineQuantity::Zero) => q.clone(),
        (AffineQuantity::One, AffineQuantity::One) => AffineQuantity::Omega,
        _ => AffineQuantity::Omega,
    }
}

/// Determine the TypeLL discipline for an AffineScript ownership modifier.
///
/// | Modifier | Discipline  | Usage  |
/// |----------|-------------|--------|
/// | `own`    | Affine      | 1      |
/// | `ref`    | Unrestricted| omega  |
/// | `mut`    | Linear      | 1      |
pub fn ownership_to_discipline(ownership: &str) -> (TypeDiscipline, UsageQuantifier) {
    match ownership {
        "own" | "Own" => (TypeDiscipline::Affine, UsageQuantifier::One),
        "ref" | "Ref" => (TypeDiscipline::Unrestricted, UsageQuantifier::Omega),
        "mut" | "Mut" => (TypeDiscipline::Linear, UsageQuantifier::One),
        _ => (TypeDiscipline::Affine, UsageQuantifier::Omega),
    }
}

/// Check that a row extension is compatible (no duplicate labels).
pub fn check_row_extension(
    existing_labels: &[String],
    new_label: &str,
) -> Result<(), String> {
    if existing_labels.contains(&new_label.to_string()) {
        Err(format!("duplicate row label: {}", new_label))
    } else {
        Ok(())
    }
}

/// Check that a function marked `total` has no Diverge effect.
pub fn check_totality(unified: &UnifiedType) -> Result<(), String> {
    for effect in &unified.effects {
        if matches!(effect, typell_core::types::Effect::Diverge) {
            return Err("total function cannot have Diverge effect".to_string());
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
    fn test_qtt_multiply_zero_absorbs() {
        let result = qtt_multiply(&AffineQuantity::Zero, &AffineQuantity::Omega);
        assert!(matches!(result, AffineQuantity::Zero));
    }

    #[test]
    fn test_qtt_multiply_linear_preserves() {
        let result = qtt_multiply(&AffineQuantity::One, &AffineQuantity::One);
        assert!(matches!(result, AffineQuantity::One));
    }

    #[test]
    fn test_qtt_add_one_one_gives_omega() {
        let result = qtt_add(&AffineQuantity::One, &AffineQuantity::One);
        assert!(matches!(result, AffineQuantity::Omega));
    }

    #[test]
    fn test_ownership_own() {
        let (disc, usage) = ownership_to_discipline("own");
        assert_eq!(disc, TypeDiscipline::Affine);
        assert_eq!(usage, UsageQuantifier::One);
    }

    #[test]
    fn test_ownership_ref() {
        let (disc, usage) = ownership_to_discipline("ref");
        assert_eq!(disc, TypeDiscipline::Unrestricted);
        assert_eq!(usage, UsageQuantifier::Omega);
    }

    #[test]
    fn test_row_extension_duplicate() {
        let labels = vec!["x".to_string(), "y".to_string()];
        assert!(check_row_extension(&labels, "x").is_err());
        assert!(check_row_extension(&labels, "z").is_ok());
    }
}
