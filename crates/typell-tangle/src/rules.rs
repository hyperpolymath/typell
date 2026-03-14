// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Tangle-specific typing rules for TypeLL.
//!
//! Implements braid width inference and composition compatibility checks.
//! In braid group theory, sigma_i requires at least i+1 strands, so the
//! width of a braid word is inferred from its generators.

use crate::bridge::TangleGenerator;

/// Infer the minimum strand count (width) for a braid word.
///
/// The width is max(generator_index) + 1, since sigma_i operates on
/// strands i and i+1 (1-indexed).
pub fn infer_width(generators: &[TangleGenerator]) -> u32 {
    if generators.is_empty() {
        return 0;
    }
    generators
        .iter()
        .map(|g| g.index.unsigned_abs() + 1)
        .max()
        .unwrap_or(0)
}

/// Check that two braid words can be composed (vertical stacking).
///
/// Composition requires both braids to have the same strand count.
pub fn check_compose_compatible(
    top_strands: u32,
    bottom_strands: u32,
) -> Result<(), String> {
    if top_strands == bottom_strands {
        Ok(())
    } else {
        Err(format!(
            "cannot compose braids: top has {} strands, bottom has {}",
            top_strands, bottom_strands
        ))
    }
}

/// Check that a braid word is valid (all generator indices in range).
///
/// For a braid on n strands, valid generators are sigma_1 through sigma_{n-1}.
pub fn check_generators_valid(
    generators: &[TangleGenerator],
    strand_count: u32,
) -> Result<(), String> {
    for generator in generators {
        let idx = generator.index.unsigned_abs();
        if idx == 0 || idx >= strand_count {
            return Err(format!(
                "generator sigma_{} invalid for {}-strand braid (valid: 1..{})",
                generator.index,
                strand_count,
                strand_count - 1
            ));
        }
    }
    Ok(())
}

/// Compute the tensor product width (horizontal juxtaposition).
///
/// Tensoring two braids of width m and n produces a braid of width m + n.
pub fn tensor_width(left: u32, right: u32) -> u32 {
    left + right
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_width_empty() {
        assert_eq!(infer_width(&[]), 0);
    }

    #[test]
    fn test_infer_width_single() {
        let gens = vec![TangleGenerator { index: 2, exponent: 1 }];
        assert_eq!(infer_width(&gens), 3);
    }

    #[test]
    fn test_infer_width_multiple() {
        let gens = vec![
            TangleGenerator { index: 1, exponent: 1 },
            TangleGenerator { index: 3, exponent: -1 },
            TangleGenerator { index: 2, exponent: 1 },
        ];
        assert_eq!(infer_width(&gens), 4);
    }

    #[test]
    fn test_compose_same_width_ok() {
        assert!(check_compose_compatible(3, 3).is_ok());
    }

    #[test]
    fn test_compose_different_width_err() {
        assert!(check_compose_compatible(3, 4).is_err());
    }

    #[test]
    fn test_generators_valid() {
        let gens = vec![
            TangleGenerator { index: 1, exponent: 1 },
            TangleGenerator { index: 2, exponent: -1 },
        ];
        assert!(check_generators_valid(&gens, 3).is_ok());
    }

    #[test]
    fn test_generators_out_of_range() {
        let gens = vec![TangleGenerator { index: 3, exponent: 1 }];
        assert!(check_generators_valid(&gens, 3).is_err());
    }

    #[test]
    fn test_tensor_width() {
        assert_eq!(tensor_width(3, 4), 7);
    }
}
