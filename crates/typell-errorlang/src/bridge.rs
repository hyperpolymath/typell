// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from Error-Lang's type representations to TypeLL's unified types.
//!
//! ## Error-Lang Type → TypeLL Mapping
//!
//! | Error-Lang Type      | TypeLL Type                              |
//! |----------------------|------------------------------------------|
//! | `TyInt`              | `Type::Primitive(PrimitiveType::Int)`    |
//! | `TyFloat`            | `Type::Primitive(PrimitiveType::Float)`  |
//! | `TyString`           | `Type::Primitive(PrimitiveType::String)` |
//! | `TyBool`             | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `TyArray(t)`         | `Type::Array { elem: t }`                |
//! | `TyIdent(s)`         | `Type::Named(s, [])`                     |
//! | `Ternary(a, b, c)`   | `Type::Named("Superposition", [a,b,c])` |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, Predicate, PrimitiveType, Term, Type, TypeDiscipline, UnifiedType,
    UsageQuantifier,
};

/// An Error-Lang type expression in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum ErrorLangType {
    Int,
    Float,
    String,
    Bool,
    Array { elem: Box<ErrorLangType> },
    Named { name: String },
}

/// Error-Lang stability factor (affects runtime behaviour).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StabilityFactor {
    MutableState { mutations: u32, readers: u32 },
    TypeInstability { reassignments: u32 },
    NullPropagation { depth: u32 },
    GlobalState { mutations: u32, dependencies: u32 },
    UnhandledError { paths: u32 },
}

/// Convert an Error-Lang type to a TypeLL base type.
pub fn errorlang_to_typell(el: &ErrorLangType) -> Type {
    match el {
        ErrorLangType::Int => Type::Primitive(PrimitiveType::Int),
        ErrorLangType::Float => Type::Primitive(PrimitiveType::Float),
        ErrorLangType::String => Type::Primitive(PrimitiveType::String),
        ErrorLangType::Bool => Type::Primitive(PrimitiveType::Bool),
        ErrorLangType::Array { elem } => Type::Array {
            elem: Box::new(errorlang_to_typell(elem)),
            length: None,
        },
        ErrorLangType::Named { name } => Type::Named {
            name: name.clone(),
            args: vec![],
        },
    }
}

/// Convert an Error-Lang type with stability context to a TypeLL unified type.
///
/// The stability score becomes a refinement predicate on the type:
/// `{score : Int | score >= 0 && score <= 100}`.
pub fn errorlang_to_unified(
    el: &ErrorLangType,
    stability_score: u32,
    factors: &[StabilityFactor],
) -> UnifiedType {
    let base = errorlang_to_typell(el);

    // Stability factors produce effects
    let mut effects = Vec::new();
    for factor in factors {
        match factor {
            StabilityFactor::MutableState { .. } => {
                effects.push(Effect::State("mutable".to_string()));
            }
            StabilityFactor::GlobalState { .. } => {
                effects.push(Effect::State("global".to_string()));
            }
            StabilityFactor::NullPropagation { .. } => {
                effects.push(Effect::Named("NullProp".to_string()));
            }
            StabilityFactor::UnhandledError { .. } => {
                effects.push(Effect::Except("Unhandled".to_string()));
            }
            StabilityFactor::TypeInstability { .. } => {
                effects.push(Effect::Named("TypeInstability".to_string()));
            }
        }
    }

    // Stability score as a refinement
    let refinements = if stability_score < 100 {
        vec![Predicate::Gte(
            Term::Var("stability".to_string()),
            Term::Lit(stability_score as i64),
        )]
    } else {
        vec![]
    };

    UnifiedType {
        base,
        usage: UsageQuantifier::Omega,
        discipline: TypeDiscipline::Unrestricted,
        dependent_indices: Vec::new(),
        effects,
        refinements,
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
            errorlang_to_typell(&ErrorLangType::Int),
            Type::Primitive(PrimitiveType::Int)
        );
    }

    #[test]
    fn test_stability_creates_refinement() {
        let unified = errorlang_to_unified(&ErrorLangType::Int, 75, &[]);
        assert_eq!(unified.refinements.len(), 1);
    }

    #[test]
    fn test_full_stability_no_refinement() {
        let unified = errorlang_to_unified(&ErrorLangType::Int, 100, &[]);
        assert!(unified.refinements.is_empty());
    }

    #[test]
    fn test_mutable_state_factor_produces_effect() {
        let factors = vec![StabilityFactor::MutableState {
            mutations: 5,
            readers: 2,
        }];
        let unified = errorlang_to_unified(&ErrorLangType::Int, 100, &factors);
        assert_eq!(unified.effects.len(), 1);
        assert!(matches!(&unified.effects[0], Effect::State(s) if s == "mutable"));
    }
}
