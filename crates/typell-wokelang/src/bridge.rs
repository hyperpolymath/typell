// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from WokeLang's type representations to TypeLL's unified types.
//!
//! ## WokeLang Type → TypeLL Mapping
//!
//! | WokeLang Type           | TypeLL Type                              |
//! |-------------------------|------------------------------------------|
//! | `TInt`                  | `Type::Primitive(PrimitiveType::Int)`    |
//! | `TFloat`                | `Type::Primitive(PrimitiveType::Float)`  |
//! | `TString`               | `Type::Primitive(PrimitiveType::String)` |
//! | `TBool`                 | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `TUnit`                 | `Type::Primitive(PrimitiveType::Unit)`   |
//! | `TArray(t)`             | `Type::Array { elem: t }`                |
//! | `TMaybe(t)`             | `Type::Named("Maybe", [t])`              |
//! | `TCustom(s)`            | `Type::Named(s, [])`                     |
//! | `EMeasured(e, unit)`    | `Type::Resource { dimension }`           |
//! | `SConsent(perm, body)`  | Effect::Named("Consent:perm")            |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Dimension, Effect, PrimitiveType, Type, TypeDiscipline, UnifiedType,
    UsageQuantifier,
};

/// A WokeLang type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum WokeType {
    String,
    Int,
    Float,
    Bool,
    Unit,
    Array { elem: Box<WokeType> },
    Maybe { inner: Box<WokeType> },
    Custom { name: String },
}

/// Convert a WokeLang type to a TypeLL base type.
pub fn woke_to_typell(woke: &WokeType) -> Type {
    match woke {
        WokeType::String => Type::Primitive(PrimitiveType::String),
        WokeType::Int => Type::Primitive(PrimitiveType::Int),
        WokeType::Float => Type::Primitive(PrimitiveType::Float),
        WokeType::Bool => Type::Primitive(PrimitiveType::Bool),
        WokeType::Unit => Type::Primitive(PrimitiveType::Unit),
        WokeType::Array { elem } => Type::Array {
            elem: Box::new(woke_to_typell(elem)),
            length: None,
        },
        WokeType::Maybe { inner } => Type::Named {
            name: "Maybe".to_string(),
            args: vec![woke_to_typell(inner)],
        },
        WokeType::Custom { name } => Type::Named {
            name: name.clone(),
            args: vec![],
        },
    }
}

/// Convert a WokeLang type to a full TypeLL unified type.
///
/// WokeLang types default to unrestricted discipline (no linear tracking).
pub fn woke_to_unified(woke: &WokeType) -> UnifiedType {
    let base = woke_to_typell(woke);
    UnifiedType {
        base,
        usage: UsageQuantifier::Omega,
        discipline: TypeDiscipline::Unrestricted,
        dependent_indices: Vec::new(),
        effects: Vec::new(),
        refinements: Vec::new(),
    }
}

/// Create a resource type for a WokeLang measured value.
///
/// Maps WokeLang unit names to TypeLL dimensions for compile-time
/// dimensional analysis.
pub fn measured_type(base: &WokeType, unit: &str) -> Type {
    let dim = unit_to_dimension(unit);
    Type::Resource {
        base: Box::new(woke_to_typell(base)),
        dimension: dim,
    }
}

/// Map a consent gate to a TypeLL effect.
///
/// Consent gates in WokeLang (`only if okay "camera" { ... }`) become
/// named effects in TypeLL, allowing the type checker to track which
/// permissions are required by a function.
pub fn consent_to_effect(permission: &str) -> Effect {
    Effect::Named(format!("Consent:{}", permission))
}

/// Map a WokeLang unit of measure to a TypeLL dimension.
pub fn unit_to_dimension(unit: &str) -> Dimension {
    match unit {
        "meters" | "meter" | "m" => Dimension::length(),
        "seconds" | "second" | "s" => Dimension::time(),
        "kilograms" | "kilogram" | "kg" => Dimension::mass(),
        "kelvin" | "K" => Dimension::temperature(),
        "joules" | "joule" | "J" => Dimension::energy(),
        "watts" | "watt" | "W" => Dimension::power(),
        _ => Dimension::dimensionless(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_conversion() {
        assert_eq!(
            woke_to_typell(&WokeType::Int),
            Type::Primitive(PrimitiveType::Int)
        );
        assert_eq!(
            woke_to_typell(&WokeType::String),
            Type::Primitive(PrimitiveType::String)
        );
    }

    #[test]
    fn test_measured_type_meters() {
        let ty = measured_type(&WokeType::Float, "meters");
        match ty {
            Type::Resource { dimension, .. } => {
                assert_eq!(dimension, Dimension::length());
            }
            _ => panic!("expected Resource type"),
        }
    }

    #[test]
    fn test_consent_effect() {
        let eff = consent_to_effect("camera");
        assert_eq!(eff, Effect::Named("Consent:camera".to_string()));
    }

    #[test]
    fn test_unified_is_unrestricted() {
        let unified = woke_to_unified(&WokeType::Int);
        assert_eq!(unified.discipline, TypeDiscipline::Unrestricted);
        assert_eq!(unified.usage, UsageQuantifier::Omega);
    }
}
