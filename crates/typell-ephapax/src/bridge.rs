// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from Ephapax's type representations to TypeLL's unified types.
//!
//! ## Ephapax Type → TypeLL Mapping
//!
//! | Ephapax Type              | TypeLL Type                              |
//! |---------------------------|------------------------------------------|
//! | `TypeExpr::Named("Int")`  | `Type::Primitive(PrimitiveType::Int)`    |
//! | `TypeExpr::Unit`          | `Type::Primitive(PrimitiveType::Unit)`   |
//! | `TypeExpr::Tuple(..)`     | `Type::Tuple(..)`                        |
//! | `TypeExpr::Array(t)`      | `Type::Array { elem: t }`                |
//! | `TypeExpr::Record(..)`    | `Type::Named("Record", ..)`              |
//! | `TypeExpr::Reference(t)`  | `Type::Named("Ref", [t])`               |
//! | `TypeExpr::Effect(t)`     | effect annotation                        |
//! | `Affinity::Linear`        | `UsageQuantifier::One`, Linear           |
//! | `Affinity::Affine`        | `UsageQuantifier::One`, Affine           |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, PrimitiveType, Type, TypeDiscipline, TypeVar, UnifiedType, UsageQuantifier,
};

/// Ephapax affinity mode.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EphapaxAffinity {
    /// Must be used exactly once.
    Linear,
    /// May be used at most once (can be dropped).
    Affine,
}

/// An Ephapax type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EphapaxType {
    Named { name: String },
    Unit,
    Tuple { elements: Vec<EphapaxType> },
    Array { elem: Box<EphapaxType> },
    Record { fields: Vec<(String, EphapaxType)> },
    Reference { mutable: bool, target: Box<EphapaxType> },
    Effect { inner: Box<EphapaxType> },
    Var { id: u32 },
}

/// Convert an Ephapax type to a TypeLL base type.
pub fn ephapax_to_typell(eph: &EphapaxType) -> Type {
    match eph {
        EphapaxType::Named { name } => Type::Primitive(map_primitive(name)),
        EphapaxType::Unit => Type::Primitive(PrimitiveType::Unit),
        EphapaxType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(ephapax_to_typell).collect())
        }
        EphapaxType::Array { elem } => Type::Array {
            elem: Box::new(ephapax_to_typell(elem)),
            length: None,
        },
        EphapaxType::Record { fields } => Type::Named {
            name: "Record".to_string(),
            args: fields.iter().map(|(_, ty)| ephapax_to_typell(ty)).collect(),
        },
        EphapaxType::Reference { mutable, target } => {
            let name = if *mutable { "MutRef" } else { "Ref" };
            Type::Named {
                name: name.to_string(),
                args: vec![ephapax_to_typell(target)],
            }
        }
        EphapaxType::Effect { inner } => {
            // Effect types wrap the inner type with an effect annotation
            ephapax_to_typell(inner)
        }
        EphapaxType::Var { id } => Type::Var(TypeVar(*id)),
    }
}

/// Convert an Ephapax type with affinity to a full TypeLL unified type.
pub fn ephapax_to_unified(eph: &EphapaxType, affinity: &EphapaxAffinity) -> UnifiedType {
    let base = ephapax_to_typell(eph);

    let (discipline, usage) = match affinity {
        EphapaxAffinity::Linear => (TypeDiscipline::Linear, UsageQuantifier::One),
        EphapaxAffinity::Affine => (TypeDiscipline::Affine, UsageQuantifier::One),
    };

    let effects = match eph {
        EphapaxType::Effect { .. } => vec![Effect::IO],
        _ => vec![],
    };

    UnifiedType {
        base,
        usage,
        discipline,
        dependent_indices: Vec::new(),
        effects,
        refinements: Vec::new(),
    }
}

fn map_primitive(name: &str) -> PrimitiveType {
    match name {
        "Bool" | "bool" => PrimitiveType::Bool,
        "Int" | "int" | "i64" => PrimitiveType::Int,
        "Float" | "float" | "f64" => PrimitiveType::Float,
        "String" | "string" => PrimitiveType::String,
        "Char" | "char" => PrimitiveType::Char,
        "Unit" | "unit" | "()" => PrimitiveType::Unit,
        _ => PrimitiveType::Unit,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear_affinity() {
        let ty = EphapaxType::Named { name: "Int".to_string() };
        let unified = ephapax_to_unified(&ty, &EphapaxAffinity::Linear);
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
        assert_eq!(unified.usage, UsageQuantifier::One);
    }

    #[test]
    fn test_affine_affinity() {
        let ty = EphapaxType::Named { name: "Int".to_string() };
        let unified = ephapax_to_unified(&ty, &EphapaxAffinity::Affine);
        assert_eq!(unified.discipline, TypeDiscipline::Affine);
        assert_eq!(unified.usage, UsageQuantifier::One);
    }

    #[test]
    fn test_reference_type() {
        let ty = EphapaxType::Reference {
            mutable: true,
            target: Box::new(EphapaxType::Named { name: "Int".to_string() }),
        };
        let result = ephapax_to_typell(&ty);
        match result {
            Type::Named { name, .. } => assert_eq!(name, "MutRef"),
            _ => panic!("expected Named type"),
        }
    }
}
