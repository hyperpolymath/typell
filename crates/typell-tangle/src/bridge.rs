// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from Tangle's braid types to TypeLL's unified types.
//!
//! ## Tangle Type → TypeLL Mapping
//!
//! | Tangle Type              | TypeLL Type                              |
//! |--------------------------|------------------------------------------|
//! | `BraidLit [g1, g2, ..]`  | `Type::Named("BraidWord", [n])`          |
//! | `Identity`               | `Type::Named("BraidWord", [0])`          |
//! | Compose(a, b)            | session Send/Recv chain                  |
//! | Tensor(a, b)             | session Offer (parallel channels)        |
//! | Crossing(a, Over, b)     | `SessionType::Send` then `Recv`          |
//! | Crossing(a, Under, b)    | `SessionType::Recv` then `Send`          |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    PrimitiveType, SessionType, Term, Type, TypeDiscipline, UnifiedType, UsageQuantifier,
};

/// A braid generator (sigma_i^e).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TangleGenerator {
    pub index: i32,
    pub exponent: i32,
}

/// A Tangle type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum TangleType {
    /// A braid word (list of generators) on n strands.
    BraidWord { generators: Vec<TangleGenerator>, strand_count: u32 },
    /// The identity braid (empty word).
    Identity { strand_count: u32 },
    /// Vertical composition of two braids.
    Compose { top: Box<TangleType>, bottom: Box<TangleType> },
    /// Horizontal tensor of two braids.
    Tensor { left: Box<TangleType>, right: Box<TangleType> },
    /// Primitive types used in computation context.
    Primitive { name: String },
    /// Boolean type.
    Bool,
    /// Integer type.
    Int,
    /// Float type.
    Float,
    /// String type.
    StringTy,
    /// Function type.
    Function { params: Vec<TangleType>, ret: Box<TangleType> },
}

/// Convert a Tangle type to a TypeLL base type.
pub fn tangle_to_typell(tng: &TangleType) -> Type {
    match tng {
        TangleType::BraidWord { strand_count, .. } => Type::Named {
            name: "BraidWord".to_string(),
            args: vec![Type::Array {
                elem: Box::new(Type::Primitive(PrimitiveType::Int)),
                length: Some(Term::Lit(*strand_count as i64)),
            }],
        },
        TangleType::Identity { strand_count } => Type::Named {
            name: "BraidWord".to_string(),
            args: vec![Type::Array {
                elem: Box::new(Type::Primitive(PrimitiveType::Int)),
                length: Some(Term::Lit(*strand_count as i64)),
            }],
        },
        TangleType::Compose { top, bottom } => {
            // Composition maps to a session type: top protocol then bottom
            Type::Session(compose_to_session(top, bottom))
        }
        TangleType::Tensor { left, right } => {
            // Tensor maps to parallel channels (Offer)
            Type::Session(tensor_to_session(left, right))
        }
        TangleType::Primitive { name } => Type::Primitive(map_primitive(name)),
        TangleType::Bool => Type::Primitive(PrimitiveType::Bool),
        TangleType::Int => Type::Primitive(PrimitiveType::Int),
        TangleType::Float => Type::Primitive(PrimitiveType::Float),
        TangleType::StringTy => Type::Primitive(PrimitiveType::String),
        TangleType::Function { params, ret } => Type::Function {
            params: params.iter().map(tangle_to_typell).collect(),
            ret: Box::new(tangle_to_typell(ret)),
            effects: vec![],
        },
    }
}

/// Convert a Tangle type to a full TypeLL unified type.
///
/// Braid types are inherently linear (a braid word is consumed when composed).
pub fn tangle_to_unified(tng: &TangleType) -> UnifiedType {
    let base = tangle_to_typell(tng);

    let (discipline, usage) = match tng {
        TangleType::BraidWord { .. }
        | TangleType::Identity { .. }
        | TangleType::Compose { .. }
        | TangleType::Tensor { .. } => (TypeDiscipline::Linear, UsageQuantifier::One),
        _ => (TypeDiscipline::Unrestricted, UsageQuantifier::Omega),
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

/// Convert braid composition to a session type (sequential protocol).
fn compose_to_session(top: &TangleType, bottom: &TangleType) -> SessionType {
    let top_ty = tangle_to_typell(top);
    let bottom_ty = tangle_to_typell(bottom);
    SessionType::Send(
        Box::new(top_ty),
        Box::new(SessionType::Recv(
            Box::new(bottom_ty),
            Box::new(SessionType::End),
        )),
    )
}

/// Convert braid tensor to a session type (parallel protocol).
fn tensor_to_session(left: &TangleType, right: &TangleType) -> SessionType {
    let left_ty = tangle_to_typell(left);
    let right_ty = tangle_to_typell(right);
    SessionType::Offer(vec![
        ("left".to_string(), SessionType::Send(Box::new(left_ty), Box::new(SessionType::End))),
        ("right".to_string(), SessionType::Send(Box::new(right_ty), Box::new(SessionType::End))),
    ])
}

fn map_primitive(name: &str) -> PrimitiveType {
    match name {
        "Bool" | "bool" => PrimitiveType::Bool,
        "Int" | "int" => PrimitiveType::Int,
        "Float" | "float" => PrimitiveType::Float,
        "String" | "string" => PrimitiveType::String,
        "Unit" | "unit" => PrimitiveType::Unit,
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
    fn test_braid_word_type() {
        let tng = TangleType::BraidWord {
            generators: vec![
                TangleGenerator { index: 1, exponent: 1 },
                TangleGenerator { index: 2, exponent: -1 },
            ],
            strand_count: 3,
        };
        let ty = tangle_to_typell(&tng);
        match ty {
            Type::Named { name, .. } => assert_eq!(name, "BraidWord"),
            _ => panic!("expected Named type"),
        }
    }

    #[test]
    fn test_braid_is_linear() {
        let tng = TangleType::BraidWord {
            generators: vec![],
            strand_count: 2,
        };
        let unified = tangle_to_unified(&tng);
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
        assert_eq!(unified.usage, UsageQuantifier::One);
    }

    #[test]
    fn test_compose_creates_session() {
        let tng = TangleType::Compose {
            top: Box::new(TangleType::Identity { strand_count: 2 }),
            bottom: Box::new(TangleType::Identity { strand_count: 2 }),
        };
        let ty = tangle_to_typell(&tng);
        assert!(matches!(ty, Type::Session(_)));
    }
}
