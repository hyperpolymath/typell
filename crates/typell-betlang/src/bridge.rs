// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from BetLang's type representations to TypeLL's unified types.
//!
//! ## BetLang Type → TypeLL Mapping
//!
//! | BetLang Type       | TypeLL Type                              |
//! |--------------------|------------------------------------------|
//! | `Unit`             | `Type::Primitive(PrimitiveType::Unit)`   |
//! | `Bool`             | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `Ternary`          | `Type::Named("Ternary", [])`             |
//! | `Int`              | `Type::Primitive(PrimitiveType::Int)`    |
//! | `Float`            | `Type::Primitive(PrimitiveType::Float)`  |
//! | `String`           | `Type::Primitive(PrimitiveType::String)` |
//! | `Bytes`            | `Type::Named("Bytes", [])`               |
//! | `Fun(a, b)`        | `Type::Function { params, ret }`         |
//! | `Dist(t)`          | `Type::Named("Dist", [t])`               |
//! | `List(t)`          | `Type::Array { elem: t }`                |
//! | `Map(k, v)`        | `Type::Named("Map", [k, v])`             |
//! | `Set(t)`           | `Type::Named("Set", [t])`                |
//! | `Tuple(..)`        | `Type::Tuple(..)`                        |
//! | `Option(t)`        | `Type::Named("Option", [t])`             |
//! | `Result(t, e)`     | `Type::Named("Result", [t, e])`          |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, PrimitiveType, Type, TypeDiscipline, TypeVar, UnifiedType, UsageQuantifier,
};

/// A BetLang type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum BetType {
    Unit,
    Bool,
    Ternary,
    Int,
    Float,
    String,
    Bytes,
    Fun { param: Box<BetType>, ret: Box<BetType> },
    Dist { inner: Box<BetType> },
    List { elem: Box<BetType> },
    Map { key: Box<BetType>, value: Box<BetType> },
    Set { elem: Box<BetType> },
    Tuple { elements: Vec<BetType> },
    Option { inner: Box<BetType> },
    Result { ok: Box<BetType>, err: Box<BetType> },
    Var { id: u32 },
    Named { name: String },
}

/// Convert a BetLang type to a TypeLL base type.
pub fn bet_to_typell(bet: &BetType) -> Type {
    match bet {
        BetType::Unit => Type::Primitive(PrimitiveType::Unit),
        BetType::Bool => Type::Primitive(PrimitiveType::Bool),
        BetType::Ternary => Type::Named { name: "Ternary".to_string(), args: vec![] },
        BetType::Int => Type::Primitive(PrimitiveType::Int),
        BetType::Float => Type::Primitive(PrimitiveType::Float),
        BetType::String => Type::Primitive(PrimitiveType::String),
        BetType::Bytes => Type::Named { name: "Bytes".to_string(), args: vec![] },
        BetType::Fun { param, ret } => Type::Function {
            params: vec![bet_to_typell(param)],
            ret: Box::new(bet_to_typell(ret)),
            effects: vec![],
        },
        BetType::Dist { inner } => Type::Named {
            name: "Dist".to_string(),
            args: vec![bet_to_typell(inner)],
        },
        BetType::List { elem } => Type::Array {
            elem: Box::new(bet_to_typell(elem)),
            length: None,
        },
        BetType::Map { key, value } => Type::Named {
            name: "Map".to_string(),
            args: vec![bet_to_typell(key), bet_to_typell(value)],
        },
        BetType::Set { elem } => Type::Named {
            name: "Set".to_string(),
            args: vec![bet_to_typell(elem)],
        },
        BetType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(bet_to_typell).collect())
        }
        BetType::Option { inner } => Type::Named {
            name: "Option".to_string(),
            args: vec![bet_to_typell(inner)],
        },
        BetType::Result { ok, err } => Type::Named {
            name: "Result".to_string(),
            args: vec![bet_to_typell(ok), bet_to_typell(err)],
        },
        BetType::Var { id } => Type::Var(TypeVar(*id)),
        BetType::Named { name } => Type::Named { name: name.clone(), args: vec![] },
    }
}

/// Convert a BetLang type to a full TypeLL unified type.
///
/// Dist<T> types carry a non-determinism effect. All other types are
/// unrestricted.
pub fn bet_to_unified(bet: &BetType) -> UnifiedType {
    let base = bet_to_typell(bet);

    let effects = match bet {
        BetType::Dist { .. } => vec![Effect::Named("NonDet".to_string())],
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
    fn test_ternary_type() {
        let ty = bet_to_typell(&BetType::Ternary);
        match ty {
            Type::Named { name, .. } => assert_eq!(name, "Ternary"),
            _ => panic!("expected Named type"),
        }
    }

    #[test]
    fn test_dist_carries_nondet_effect() {
        let bet = BetType::Dist { inner: Box::new(BetType::Int) };
        let unified = bet_to_unified(&bet);
        assert_eq!(unified.effects.len(), 1);
        assert_eq!(unified.effects[0], Effect::Named("NonDet".to_string()));
    }

    #[test]
    fn test_function_type() {
        let bet = BetType::Fun {
            param: Box::new(BetType::Int),
            ret: Box::new(BetType::Bool),
        };
        let ty = bet_to_typell(&bet);
        match ty {
            Type::Function { params, ret, .. } => {
                assert_eq!(params.len(), 1);
                assert_eq!(*ret, Type::Primitive(PrimitiveType::Bool));
            }
            _ => panic!("expected Function type"),
        }
    }

    #[test]
    fn test_result_type() {
        let bet = BetType::Result {
            ok: Box::new(BetType::Int),
            err: Box::new(BetType::String),
        };
        let ty = bet_to_typell(&bet);
        match ty {
            Type::Named { name, args } => {
                assert_eq!(name, "Result");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected Named type"),
        }
    }
}
