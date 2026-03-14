// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from JtV's type representations to TypeLL's unified types.
//!
//! ## JtV Type → TypeLL Mapping
//!
//! | JtV Type            | TypeLL Type                              |
//! |---------------------|------------------------------------------|
//! | `Int`               | `Type::Primitive(PrimitiveType::Int)`    |
//! | `Float`             | `Type::Primitive(PrimitiveType::Float)`  |
//! | `Rational`          | `Type::Named("Rational", [])`            |
//! | `Complex`           | `Type::Named("Complex", [])`             |
//! | `Hex`               | `Type::Named("Hex", [])`                 |
//! | `Binary`            | `Type::Named("Binary", [])`              |
//! | `Symbolic`          | `Type::Named("Symbolic", [])`            |
//! | `Bool`              | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `String`            | `Type::Primitive(PrimitiveType::String)` |
//! | `Unit`              | `Type::Primitive(PrimitiveType::Unit)`   |
//! | `List(t)`           | `Type::Array { elem: t }`                |
//! | `Tuple(..)`         | `Type::Tuple(..)`                        |
//! | `Function(p, r)`    | `Type::Function { .. }`                  |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, PrimitiveType, Type, TypeDiscipline, UnifiedType, UsageQuantifier,
};

/// JtV purity annotation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum JtvPurity {
    /// Pure data function (no side effects, total).
    Pure,
    /// Impure control function (may have effects).
    Impure,
}

/// A JtV type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum JtvType {
    Int,
    Float,
    Rational,
    Complex,
    Hex,
    Binary,
    Symbolic,
    Bool,
    String,
    Unit,
    List { elem: Box<JtvType> },
    Tuple { elements: Vec<JtvType> },
    Function { params: Vec<JtvType>, ret: Box<JtvType> },
    Any,
}

/// Convert a JtV type to a TypeLL base type.
pub fn jtv_to_typell(jtv: &JtvType) -> Type {
    match jtv {
        JtvType::Int => Type::Primitive(PrimitiveType::Int),
        JtvType::Float => Type::Primitive(PrimitiveType::Float),
        JtvType::Rational => Type::Named { name: "Rational".to_string(), args: vec![] },
        JtvType::Complex => Type::Named { name: "Complex".to_string(), args: vec![] },
        JtvType::Hex => Type::Named { name: "Hex".to_string(), args: vec![] },
        JtvType::Binary => Type::Named { name: "Binary".to_string(), args: vec![] },
        JtvType::Symbolic => Type::Named { name: "Symbolic".to_string(), args: vec![] },
        JtvType::Bool => Type::Primitive(PrimitiveType::Bool),
        JtvType::String => Type::Primitive(PrimitiveType::String),
        JtvType::Unit => Type::Primitive(PrimitiveType::Unit),
        JtvType::List { elem } => Type::Array {
            elem: Box::new(jtv_to_typell(elem)),
            length: None,
        },
        JtvType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(jtv_to_typell).collect())
        }
        JtvType::Function { params, ret } => Type::Function {
            params: params.iter().map(jtv_to_typell).collect(),
            ret: Box::new(jtv_to_typell(ret)),
            effects: vec![],
        },
        JtvType::Any => Type::Var(typell_core::types::TypeVar(0)), // Placeholder
    }
}

/// Convert a JtV type with purity annotation to a full TypeLL unified type.
///
/// Pure (Data) functions have no effects. Impure (Control) functions carry IO.
pub fn jtv_to_unified(jtv: &JtvType, purity: &JtvPurity) -> UnifiedType {
    let base = jtv_to_typell(jtv);

    let effects = match purity {
        JtvPurity::Pure => vec![],
        JtvPurity::Impure => vec![Effect::IO],
    };

    UnifiedType {
        base,
        usage: UsageQuantifier::Omega,
        discipline: TypeDiscipline::Unrestricted,
        dependent_indices: Vec::new(),
        effects,
        refinements: Vec::new(),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seven_number_systems() {
        // All 7 number systems should produce distinct types
        let types = [
            JtvType::Int, JtvType::Float, JtvType::Rational,
            JtvType::Complex, JtvType::Hex, JtvType::Binary,
            JtvType::Symbolic,
        ];
        let converted: Vec<_> = types.iter().map(jtv_to_typell).collect();
        // Check all distinct
        for i in 0..converted.len() {
            for j in (i + 1)..converted.len() {
                assert_ne!(converted[i], converted[j]);
            }
        }
    }

    #[test]
    fn test_pure_has_no_effects() {
        let unified = jtv_to_unified(&JtvType::Int, &JtvPurity::Pure);
        assert!(unified.effects.is_empty());
    }

    #[test]
    fn test_impure_has_io_effect() {
        let unified = jtv_to_unified(&JtvType::Int, &JtvPurity::Impure);
        assert_eq!(unified.effects.len(), 1);
        assert_eq!(unified.effects[0], Effect::IO);
    }

    #[test]
    fn test_rational_type() {
        let ty = jtv_to_typell(&JtvType::Rational);
        match ty {
            Type::Named { name, .. } => assert_eq!(name, "Rational"),
            _ => panic!("expected Named type"),
        }
    }
}
