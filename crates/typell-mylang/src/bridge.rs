// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from My-Lang's type representations to TypeLL's unified types.
//!
//! ## My-Lang Type → TypeLL Mapping
//!
//! | My-Lang Type        | TypeLL Type                              |
//! |---------------------|------------------------------------------|
//! | `Ty::Int`           | `Type::Primitive(PrimitiveType::Int)`    |
//! | `Ty::Float`         | `Type::Primitive(PrimitiveType::Float)`  |
//! | `Ty::String`        | `Type::Primitive(PrimitiveType::String)` |
//! | `Ty::Bool`          | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `Ty::Unit`          | `Type::Primitive(PrimitiveType::Unit)`   |
//! | `Ty::AI(t)`         | base=t, effect=AI                        |
//! | `Ty::Effect(t)`     | base=t, effect=IO                        |
//! | `Ty::Function {..}` | `Type::Function { .. }`                  |
//! | `Ty::Array(t)`      | `Type::Array { elem: t }`                |
//! | `Ty::Ref { .. }`    | `Type::Named("Ref"/"MutRef", [t])`       |
//! | `Ty::Record(..)`    | `Type::Named("Record", ..)`              |
//! | `Ty::Tuple(..)`     | `Type::Tuple(..)`                        |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, PrimitiveType, Type, TypeDiscipline, TypeVar, UnifiedType, UsageQuantifier,
};

/// A My-Lang type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum MyLangType {
    Int,
    Float,
    String,
    Bool,
    Unit,
    Named { name: String },
    Function { params: Vec<MyLangType>, result: Box<MyLangType> },
    Array { elem: Box<MyLangType> },
    Ref { mutable: bool, inner: Box<MyLangType> },
    Tuple { elements: Vec<MyLangType> },
    Record { fields: Vec<(String, MyLangType)> },
    AI { inner: Box<MyLangType> },
    Effect { inner: Box<MyLangType> },
    Var { id: u32 },
    Error,
    Unknown,
}

/// Convert a My-Lang type to a TypeLL base type.
pub fn mylang_to_typell(ml: &MyLangType) -> Type {
    match ml {
        MyLangType::Int => Type::Primitive(PrimitiveType::Int),
        MyLangType::Float => Type::Primitive(PrimitiveType::Float),
        MyLangType::String => Type::Primitive(PrimitiveType::String),
        MyLangType::Bool => Type::Primitive(PrimitiveType::Bool),
        MyLangType::Unit => Type::Primitive(PrimitiveType::Unit),
        MyLangType::Named { name } => Type::Named { name: name.clone(), args: vec![] },
        MyLangType::Function { params, result } => Type::Function {
            params: params.iter().map(mylang_to_typell).collect(),
            ret: Box::new(mylang_to_typell(result)),
            effects: vec![],
        },
        MyLangType::Array { elem } => Type::Array {
            elem: Box::new(mylang_to_typell(elem)),
            length: None,
        },
        MyLangType::Ref { mutable, inner } => {
            let name = if *mutable { "MutRef" } else { "Ref" };
            Type::Named {
                name: name.to_string(),
                args: vec![mylang_to_typell(inner)],
            }
        }
        MyLangType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(mylang_to_typell).collect())
        }
        MyLangType::Record { fields } => Type::Named {
            name: "Record".to_string(),
            args: fields.iter().map(|(_, ty)| mylang_to_typell(ty)).collect(),
        },
        MyLangType::AI { inner } => mylang_to_typell(inner),
        MyLangType::Effect { inner } => mylang_to_typell(inner),
        MyLangType::Var { id } => Type::Var(TypeVar(*id as u32)),
        MyLangType::Error | MyLangType::Unknown => Type::Error,
    }
}

/// Convert a My-Lang type to a full TypeLL unified type.
///
/// AI<T> values carry an AI effect (non-deterministic inference).
/// Effect<T> values carry an IO effect.
pub fn mylang_to_unified(ml: &MyLangType) -> UnifiedType {
    let base = mylang_to_typell(ml);

    let effects = match ml {
        MyLangType::AI { .. } => vec![Effect::Named("AI".to_string())],
        MyLangType::Effect { .. } => vec![Effect::IO],
        _ => vec![],
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
    fn test_ai_type_carries_effect() {
        let ml = MyLangType::AI { inner: Box::new(MyLangType::String) };
        let unified = mylang_to_unified(&ml);
        assert_eq!(unified.effects.len(), 1);
        assert_eq!(unified.effects[0], Effect::Named("AI".to_string()));
        assert_eq!(unified.base, Type::Primitive(PrimitiveType::String));
    }

    #[test]
    fn test_effect_type_carries_io() {
        let ml = MyLangType::Effect { inner: Box::new(MyLangType::Int) };
        let unified = mylang_to_unified(&ml);
        assert_eq!(unified.effects.len(), 1);
        assert_eq!(unified.effects[0], Effect::IO);
    }

    #[test]
    fn test_function_type() {
        let ml = MyLangType::Function {
            params: vec![MyLangType::Int],
            result: Box::new(MyLangType::Bool),
        };
        let ty = mylang_to_typell(&ml);
        match ty {
            Type::Function { params, ret, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(*ret, Type::Primitive(PrimitiveType::Bool));
            }
            _ => panic!("expected Function type"),
        }
    }

    #[test]
    fn test_error_type() {
        let ty = mylang_to_typell(&MyLangType::Error);
        assert_eq!(ty, Type::Error);
    }
}
