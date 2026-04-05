// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! VCL-total 10-level type safety hierarchy.
//!
//! Each level builds on the previous, with monotonically increasing
//! type safety guarantees. A query at level N satisfies all levels <= N.

use serde::{Deserialize, Serialize};

/// The 10 VCL-total type safety levels.
///
/// Ordered from weakest (Level1) to strongest (Level10).
/// Each level subsumes all lower levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum SafetyLevel {
    /// Level 1: Parse-time safety — well-formed syntax, no grammar errors.
    ParseTime = 1,
    /// Level 2: Schema-binding safety — all table/column names resolve.
    SchemaBinding = 2,
    /// Level 3: Type-compatible operations — operators match operand types.
    TypeCompatible = 3,
    /// Level 4: Null-safety — Option types, no implicit null coercion.
    NullSafe = 4,
    /// Level 5: Injection-proof safety — parameterised queries, refinement predicates.
    InjectionProof = 5,
    /// Level 6: Result-type safety — return type is statically known.
    ResultType = 6,
    /// Level 7: Cardinality safety — bounded quantifiers, row count guarantees.
    Cardinality = 7,
    /// Level 8: Effect-tracking safety — algebraic effects (Read, Write, Cite, etc.).
    EffectTracking = 8,
    /// Level 9: Temporal safety — session types, transaction state machines.
    Temporal = 9,
    /// Level 10: Linearity safety — QTT bounded usage, linear/affine types.
    Linearity = 10,
}

impl SafetyLevel {
    /// Returns the numeric level (1–10).
    pub fn as_u8(self) -> u8 {
        self as u8
    }

    /// Returns the human-readable level name.
    pub fn name(self) -> &'static str {
        match self {
            Self::ParseTime => "Parse-time safety",
            Self::SchemaBinding => "Schema-binding safety",
            Self::TypeCompatible => "Type-compatible operations",
            Self::NullSafe => "Null-safety",
            Self::InjectionProof => "Injection-proof safety",
            Self::ResultType => "Result-type safety",
            Self::Cardinality => "Cardinality safety",
            Self::EffectTracking => "Effect-tracking safety",
            Self::Temporal => "Temporal safety",
            Self::Linearity => "Linearity safety",
        }
    }

    /// Returns the TypeLL concept this level maps to.
    pub fn typell_concept(self) -> &'static str {
        match self {
            Self::ParseTime => "Well-formed AST",
            Self::SchemaBinding => "Named type resolution",
            Self::TypeCompatible => "Unification + operator checking",
            Self::NullSafe => "Option types, totality checking",
            Self::InjectionProof => "Refinement predicates",
            Self::ResultType => "Return type inference",
            Self::Cardinality => "Bounded quantifiers",
            Self::EffectTracking => "Algebraic effects",
            Self::Temporal => "Session types, state machines",
            Self::Linearity => "QTT bounded usage, linear types",
        }
    }

    /// Returns all levels from Level 1 up to and including this level.
    pub fn satisfied_levels(self) -> Vec<SafetyLevel> {
        ALL_LEVELS.iter().copied().filter(|l| *l <= self).collect()
    }

    /// Parse from a numeric value (1–10).
    pub fn from_u8(n: u8) -> Option<Self> {
        match n {
            1 => Some(Self::ParseTime),
            2 => Some(Self::SchemaBinding),
            3 => Some(Self::TypeCompatible),
            4 => Some(Self::NullSafe),
            5 => Some(Self::InjectionProof),
            6 => Some(Self::ResultType),
            7 => Some(Self::Cardinality),
            8 => Some(Self::EffectTracking),
            9 => Some(Self::Temporal),
            10 => Some(Self::Linearity),
            _ => None,
        }
    }

    /// Whether this level is in the "established" tier (1–6) or
    /// "research-identified" tier (7–10).
    pub fn is_established(self) -> bool {
        self.as_u8() <= 6
    }
}

/// All 10 levels in order.
pub const ALL_LEVELS: [SafetyLevel; 10] = [
    SafetyLevel::ParseTime,
    SafetyLevel::SchemaBinding,
    SafetyLevel::TypeCompatible,
    SafetyLevel::NullSafe,
    SafetyLevel::InjectionProof,
    SafetyLevel::ResultType,
    SafetyLevel::Cardinality,
    SafetyLevel::EffectTracking,
    SafetyLevel::Temporal,
    SafetyLevel::Linearity,
];

/// Result of checking a query against the VCL-total safety hierarchy.
///
/// Records which levels passed, which failed, and the maximum
/// achieved safety level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyReport {
    /// The maximum safety level achieved (all levels below also pass).
    pub max_level: SafetyLevel,
    /// Per-level check results.
    pub checks: Vec<LevelCheck>,
    /// The query path used: VCL (slipstream), VCL-DT, or VCL-total.
    pub query_path: QueryPath,
}

/// Check result for a single safety level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelCheck {
    /// The level that was checked.
    pub level: SafetyLevel,
    /// Whether this level passed.
    pub passed: bool,
    /// Human-readable diagnostic (empty if passed).
    pub diagnostic: String,
}

/// The three VCL query paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueryPath {
    /// VCL Slipstream — runtime checks only, no static type safety.
    Slipstream,
    /// VCL-DT — dependent types via Lean (levels 1–6 + some level 5 proofs).
    Dt,
    /// VCL-total — full 10-level verification via Idris2.
    Ut,
}

impl QueryPath {
    /// Returns the human-readable name of this query path.
    pub fn name(self) -> &'static str {
        match self {
            Self::Slipstream => "VCL (Slipstream)",
            Self::Dt => "VCL-DT",
            Self::Ut => "VCL-total",
        }
    }

    /// Returns the maximum level achievable on this path.
    pub fn max_achievable(self) -> SafetyLevel {
        match self {
            Self::Slipstream => SafetyLevel::ParseTime,
            Self::Dt => SafetyLevel::ResultType,
            Self::Ut => SafetyLevel::Linearity,
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_level_ordering() {
        assert!(SafetyLevel::ParseTime < SafetyLevel::Linearity);
        assert!(SafetyLevel::NullSafe < SafetyLevel::EffectTracking);
    }

    #[test]
    fn test_satisfied_levels() {
        let levels = SafetyLevel::TypeCompatible.satisfied_levels();
        assert_eq!(levels.len(), 3);
        assert_eq!(levels[0], SafetyLevel::ParseTime);
        assert_eq!(levels[2], SafetyLevel::TypeCompatible);
    }

    #[test]
    fn test_round_trip_from_u8() {
        for level in ALL_LEVELS {
            assert_eq!(SafetyLevel::from_u8(level.as_u8()), Some(level));
        }
    }

    #[test]
    fn test_established_vs_research() {
        assert!(SafetyLevel::ResultType.is_established());
        assert!(!SafetyLevel::Cardinality.is_established());
        assert!(!SafetyLevel::Linearity.is_established());
    }

    #[test]
    fn test_query_path_max_achievable() {
        assert_eq!(QueryPath::Slipstream.max_achievable(), SafetyLevel::ParseTime);
        assert_eq!(QueryPath::Dt.max_achievable(), SafetyLevel::ResultType);
        assert_eq!(QueryPath::Ut.max_achievable(), SafetyLevel::Linearity);
    }
}
