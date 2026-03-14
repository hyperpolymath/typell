// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from Eclexia's type representations to TypeLL's unified types.
//!
//! This module converts Eclexia AST types (as defined in `eclexia-ast`)
//! into TypeLL's `Type` and `UnifiedType` representations. Since TypeLL
//! does not depend on Eclexia's crates directly (to keep the kernel
//! language-agnostic), the bridge works via a serialized intermediate
//! representation.
//!
//! ## Eclexia Type System → TypeLL Mapping
//!
//! | Eclexia Type           | TypeLL Type                        |
//! |------------------------|------------------------------------|
//! | `Ty::Primitive(Int)`   | `Type::Primitive(PrimitiveType::Int)` |
//! | `Ty::Var(v)`           | `Type::Var(TypeVar(v.0))`          |
//! | `Ty::Named { .. }`    | `Type::Named { .. }`               |
//! | `Ty::Function { .. }` | `Type::Function { .. }`            |
//! | `Ty::Tuple(..)`       | `Type::Tuple(..)`                  |
//! | `Ty::Array { .. }`    | `Type::Array { .. }`               |
//! | `Ty::Resource { .. }` | `Type::Resource { .. }`            |
//! | `Ty::ForAll { .. }`   | `Type::ForAll { .. }`              |
//! | `Ty::Error`           | `Type::Error`                      |
//! | `Ty::Never`           | `Type::Primitive(Never)`           |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Dimension, Effect, PrimitiveType, Type, TypeDiscipline, TypeVar,
    UnifiedType, UsageQuantifier,
};

/// An Eclexia type in serialized form (for cross-crate bridge).
///
/// This mirrors the structure of `eclexia_ast::types::Ty` but is
/// self-contained (no dependency on eclexia-ast). Eclexia's type
/// checker serializes its types to this format, and the bridge
/// converts them into TypeLL types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum EclexiaType {
    Primitive { name: String },
    Var { id: u32 },
    Named { name: String, args: Vec<EclexiaType> },
    Function { params: Vec<EclexiaType>, ret: Box<EclexiaType> },
    Tuple { elements: Vec<EclexiaType> },
    Array { elem: Box<EclexiaType>, size: Option<u64> },
    Resource { base: Box<EclexiaType>, dimension: EclexiaDimension },
    ForAll { vars: Vec<String>, body: Box<EclexiaType> },
    Error,
    Never,
}

/// Eclexia's dimension representation in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EclexiaDimension {
    pub mass: i8,
    pub length: i8,
    pub time: i8,
    pub current: i8,
    pub temperature: i8,
    pub amount: i8,
    pub luminosity: i8,
    pub money: i8,
    pub carbon: i8,
    pub information: i8,
}

/// Convert an Eclexia type to a TypeLL type.
pub fn eclexia_to_typell(ecl: &EclexiaType) -> Type {
    match ecl {
        EclexiaType::Primitive { name } => {
            Type::Primitive(map_primitive(name))
        }
        EclexiaType::Var { id } => Type::Var(TypeVar(*id)),
        EclexiaType::Named { name, args } => Type::Named {
            name: name.clone(),
            args: args.iter().map(eclexia_to_typell).collect(),
        },
        EclexiaType::Function { params, ret } => Type::Function {
            params: params.iter().map(eclexia_to_typell).collect(),
            ret: Box::new(eclexia_to_typell(ret)),
            effects: vec![], // Eclexia tracks effects separately
        },
        EclexiaType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(eclexia_to_typell).collect())
        }
        EclexiaType::Array { elem, size } => Type::Array {
            elem: Box::new(eclexia_to_typell(elem)),
            length: size.map(|s| typell_core::types::Term::Lit(s as i64)),
        },
        EclexiaType::Resource { base, dimension } => Type::Resource {
            base: Box::new(eclexia_to_typell(base)),
            dimension: map_dimension(dimension),
        },
        EclexiaType::ForAll { vars, body } => Type::ForAll {
            vars: vars.clone(),
            body: Box::new(eclexia_to_typell(body)),
        },
        EclexiaType::Error => Type::Error,
        EclexiaType::Never => Type::Primitive(PrimitiveType::Never),
    }
}

/// Convert an Eclexia type to a full TypeLL unified type.
///
/// Eclexia types default to affine discipline with omega usage
/// (Eclexia doesn't yet have native linear type support — that's
/// what TypeLL adds).
pub fn eclexia_to_unified(ecl: &EclexiaType) -> UnifiedType {
    let base = eclexia_to_typell(ecl);

    // Determine discipline based on type shape
    let discipline = if matches!(&base, Type::Resource { .. }) {
        // Resource types should be tracked linearly
        TypeDiscipline::Linear
    } else {
        TypeDiscipline::Affine
    };

    let usage = if discipline == TypeDiscipline::Linear {
        UsageQuantifier::One
    } else {
        UsageQuantifier::Omega
    };

    UnifiedType {
        base,
        usage,
        discipline,
        dependent_indices: Vec::new(),
        effects: Vec::new(),
        refinements: Vec::new(),
    }
}

/// Convert Eclexia effects to TypeLL effects.
pub fn map_eclexia_effects(effects: &[String]) -> Vec<Effect> {
    effects
        .iter()
        .map(|e| match e.as_str() {
            "IO" | "io" => Effect::IO,
            "Alloc" | "alloc" => Effect::Alloc,
            "Network" | "network" => Effect::Network,
            "FileSystem" | "filesystem" | "fs" => Effect::FileSystem,
            "Diverge" | "diverge" => Effect::Diverge,
            s if s.starts_with("State") => {
                Effect::State(s.trim_start_matches("State").trim().to_string())
            }
            s if s.starts_with("Except") => {
                Effect::Except(s.trim_start_matches("Except").trim().to_string())
            }
            other => Effect::Named(other.to_string()),
        })
        .collect()
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Map Eclexia primitive type names to TypeLL primitive types.
fn map_primitive(name: &str) -> PrimitiveType {
    match name {
        "Bool" | "bool" => PrimitiveType::Bool,
        "Int" | "int" => PrimitiveType::Int,
        "I8" | "i8" => PrimitiveType::I8,
        "I16" | "i16" => PrimitiveType::I16,
        "I32" | "i32" => PrimitiveType::I32,
        "I64" | "i64" => PrimitiveType::I64,
        "I128" | "i128" => PrimitiveType::I128,
        "U8" | "u8" => PrimitiveType::U8,
        "U16" | "u16" => PrimitiveType::U16,
        "U32" | "u32" => PrimitiveType::U32,
        "U64" | "u64" => PrimitiveType::U64,
        "U128" | "u128" => PrimitiveType::U128,
        "Float" | "float" | "f64" => PrimitiveType::Float,
        "F32" | "f32" => PrimitiveType::F32,
        "F64" => PrimitiveType::F64,
        "Char" | "char" => PrimitiveType::Char,
        "String" | "string" => PrimitiveType::String,
        "Unit" | "unit" | "()" => PrimitiveType::Unit,
        _ => PrimitiveType::Unit, // Fallback
    }
}

/// Map Eclexia dimension to TypeLL dimension.
fn map_dimension(dim: &EclexiaDimension) -> Dimension {
    Dimension {
        mass: dim.mass,
        length: dim.length,
        time: dim.time,
        current: dim.current,
        temperature: dim.temperature,
        amount: dim.amount,
        luminosity: dim.luminosity,
        money: dim.money,
        carbon: dim.carbon,
        information: dim.information,
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
        let ecl = EclexiaType::Primitive { name: "Int".to_string() };
        let ty = eclexia_to_typell(&ecl);
        assert_eq!(ty, Type::Primitive(PrimitiveType::Int));
    }

    #[test]
    fn test_resource_gets_linear_discipline() {
        let ecl = EclexiaType::Resource {
            base: Box::new(EclexiaType::Primitive { name: "Float".to_string() }),
            dimension: EclexiaDimension {
                mass: 1, length: 2, time: -2,
                current: 0, temperature: 0, amount: 0,
                luminosity: 0, money: 0, carbon: 0, information: 0,
            },
        };
        let unified = eclexia_to_unified(&ecl);
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
        assert_eq!(unified.usage, UsageQuantifier::One);
    }

    #[test]
    fn test_function_conversion() {
        let ecl = EclexiaType::Function {
            params: vec![EclexiaType::Primitive { name: "Int".to_string() }],
            ret: Box::new(EclexiaType::Primitive { name: "Bool".to_string() }),
        };
        let ty = eclexia_to_typell(&ecl);
        match ty {
            Type::Function { params, ret, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(*ret, Type::Primitive(PrimitiveType::Bool));
            }
            _ => panic!("expected Function type"),
        }
    }

    #[test]
    fn test_effect_mapping() {
        let effects = vec![
            "IO".to_string(),
            "State s".to_string(),
            "Except Error".to_string(),
        ];
        let mapped = map_eclexia_effects(&effects);
        assert_eq!(mapped.len(), 3);
        assert_eq!(mapped[0], Effect::IO);
    }
}
