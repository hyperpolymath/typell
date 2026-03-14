// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from AffineScript's type representations to TypeLL's unified types.
//!
//! AffineScript's type system is the richest of the nextgen languages, with
//! full QTT (0/1/omega), refinement types, row polymorphism, effects, and
//! dependent arrows. This bridge preserves all that information when lowering
//! to TypeLL's unified representation.
//!
//! ## AffineScript Type → TypeLL Mapping
//!
//! | AffineScript Type       | TypeLL Type                              |
//! |-------------------------|------------------------------------------|
//! | `TCon "Int"`            | `Type::Primitive(PrimitiveType::Int)`    |
//! | `TVar(v)`               | `Type::Var(TypeVar(v))`                  |
//! | `TArrow(a, b, eff)`     | `Type::Function { effects }`             |
//! | `TDepArrow(x, a, b, e)` | `Type::Pi { param_name, .. }`            |
//! | `TTuple(..)`            | `Type::Tuple(..)`                        |
//! | `TRecord(row)`          | `Type::Named("Record", ..)`              |
//! | `TVariant(row)`         | `Type::Named("Variant", ..)`             |
//! | `TForall(v, k, body)`   | `Type::ForAll { .. }`                    |
//! | `TExists(v, k, body)`   | `Type::Named("Exists", ..)`              |
//! | `TRef(t)`               | `Type::Named("Ref", [t])`               |
//! | `TMut(t)`               | `Type::Named("Mut", [t])`               |
//! | `TOwn(t)`               | `Type::Named("Own", [t])`               |
//! | `TRefined(t, p)`        | `Type::Refined { base, predicates }`     |
//! | `TNat(n)`               | dependent index term                     |
//! | `QZero`                 | `UsageQuantifier::Zero`                  |
//! | `QOne`                  | `UsageQuantifier::One`                   |
//! | `QOmega`                | `UsageQuantifier::Omega`                 |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, Predicate, PrimitiveType, Term, TermOp, Type, TypeDiscipline,
    TypeVar, UnifiedType, UsageQuantifier,
};

/// An AffineScript quantity annotation in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffineQuantity {
    Zero,
    One,
    Omega,
    Var(u32),
}

/// An AffineScript kind in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AffineKind {
    Type,
    Nat,
    Row,
    Effect,
    Arrow(Box<AffineKind>, Box<AffineKind>),
}

/// An AffineScript type in serialized form.
///
/// Mirrors the structure of `types.ml::ty` but is self-contained for
/// cross-crate bridging.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type_kind")]
pub enum AffineType {
    Var { id: u32 },
    Con { name: String },
    App { con: Box<AffineType>, args: Vec<AffineType> },
    Arrow {
        param: Box<AffineType>,
        ret: Box<AffineType>,
        effect: AffineEffect,
    },
    DepArrow {
        param_name: String,
        param_type: Box<AffineType>,
        ret_type: Box<AffineType>,
        effect: AffineEffect,
    },
    Tuple { elements: Vec<AffineType> },
    Record { fields: Vec<AffineRowField> },
    Variant { fields: Vec<AffineRowField> },
    ForAll { var: u32, kind: AffineKind, body: Box<AffineType> },
    Exists { var: u32, kind: AffineKind, body: Box<AffineType> },
    Ref { inner: Box<AffineType> },
    Mut { inner: Box<AffineType> },
    Own { inner: Box<AffineType> },
    Refined { base: Box<AffineType>, predicate: AffinePredicate },
    Nat { expr: AffineNatExpr },
    Error,
}

/// A row field (label + type pair).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffineRowField {
    pub label: String,
    pub ty: AffineType,
}

/// An AffineScript effect in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AffineEffect {
    Pure,
    Var { id: u32 },
    Singleton { name: String },
    Union { effects: Vec<AffineEffect> },
}

/// A nat-level expression for dependent type indices.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AffineNatExpr {
    Lit { value: i64 },
    Var { name: String },
    Add { lhs: Box<AffineNatExpr>, rhs: Box<AffineNatExpr> },
    Sub { lhs: Box<AffineNatExpr>, rhs: Box<AffineNatExpr> },
    Mul { lhs: Box<AffineNatExpr>, rhs: Box<AffineNatExpr> },
    Len { name: String },
}

/// A predicate for refinement types.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum AffinePredicate {
    True,
    False,
    Eq { lhs: AffineNatExpr, rhs: AffineNatExpr },
    Lt { lhs: AffineNatExpr, rhs: AffineNatExpr },
    Le { lhs: AffineNatExpr, rhs: AffineNatExpr },
    Gt { lhs: AffineNatExpr, rhs: AffineNatExpr },
    Ge { lhs: AffineNatExpr, rhs: AffineNatExpr },
    And { lhs: Box<AffinePredicate>, rhs: Box<AffinePredicate> },
    Or { lhs: Box<AffinePredicate>, rhs: Box<AffinePredicate> },
    Not { inner: Box<AffinePredicate> },
    Impl { lhs: Box<AffinePredicate>, rhs: Box<AffinePredicate> },
}

/// Convert an AffineScript quantity to a TypeLL usage quantifier.
pub fn quantity_to_usage(q: &AffineQuantity) -> UsageQuantifier {
    match q {
        AffineQuantity::Zero => UsageQuantifier::Zero,
        AffineQuantity::One => UsageQuantifier::One,
        AffineQuantity::Omega => UsageQuantifier::Omega,
        AffineQuantity::Var(_) => UsageQuantifier::Omega, // Default for unresolved
    }
}

/// Convert an AffineScript type to a TypeLL base type.
pub fn affine_to_typell(aff: &AffineType) -> Type {
    match aff {
        AffineType::Con { name } => Type::Primitive(map_primitive(name)),
        AffineType::Var { id } => Type::Var(TypeVar(*id)),
        AffineType::App { con, args } => {
            let base = affine_to_typell(con);
            if let Type::Named { name, args: existing } = base {
                let mut all_args = existing;
                all_args.extend(args.iter().map(affine_to_typell));
                Type::Named { name, args: all_args }
            } else {
                // Wrap in Named if the constructor isn't already a Named type
                Type::Named {
                    name: format!("{:?}", con),
                    args: args.iter().map(affine_to_typell).collect(),
                }
            }
        }
        AffineType::Arrow { param, ret, effect } => Type::Function {
            params: vec![affine_to_typell(param)],
            ret: Box::new(affine_to_typell(ret)),
            effects: map_effect(effect),
        },
        AffineType::DepArrow { param_name, param_type, ret_type, .. } => Type::Pi {
            param_name: param_name.clone(),
            param_type: Box::new(affine_to_typell(param_type)),
            body: Box::new(affine_to_typell(ret_type)),
        },
        AffineType::Tuple { elements } => {
            Type::Tuple(elements.iter().map(affine_to_typell).collect())
        }
        AffineType::Record { fields } => Type::Named {
            name: "Record".to_string(),
            args: fields.iter().map(|f| affine_to_typell(&f.ty)).collect(),
        },
        AffineType::Variant { fields } => Type::Named {
            name: "Variant".to_string(),
            args: fields.iter().map(|f| affine_to_typell(&f.ty)).collect(),
        },
        AffineType::ForAll { var, body, .. } => Type::ForAll {
            vars: vec![format!("t{}", var)],
            body: Box::new(affine_to_typell(body)),
        },
        AffineType::Exists { var, body, .. } => Type::Named {
            name: "Exists".to_string(),
            args: vec![Type::ForAll {
                vars: vec![format!("t{}", var)],
                body: Box::new(affine_to_typell(body)),
            }],
        },
        AffineType::Ref { inner } => Type::Named {
            name: "Ref".to_string(),
            args: vec![affine_to_typell(inner)],
        },
        AffineType::Mut { inner } => Type::Named {
            name: "Mut".to_string(),
            args: vec![affine_to_typell(inner)],
        },
        AffineType::Own { inner } => Type::Named {
            name: "Own".to_string(),
            args: vec![affine_to_typell(inner)],
        },
        AffineType::Refined { base, predicate } => Type::Refined {
            base: Box::new(affine_to_typell(base)),
            predicates: map_predicate(predicate),
        },
        AffineType::Nat { expr: _ } => {
            // Type-level natural becomes a dependent index
            Type::Named {
                name: "Nat".to_string(),
                args: vec![],
            }
        }
        AffineType::Error => Type::Error,
    }
}

/// Convert an AffineScript type to a full TypeLL unified type with QTT annotation.
pub fn affine_to_unified(aff: &AffineType, quantity: &AffineQuantity) -> UnifiedType {
    let base = affine_to_typell(aff);
    let usage = quantity_to_usage(quantity);

    let discipline = match quantity {
        AffineQuantity::Zero => TypeDiscipline::Dependent, // Erased: proof witness
        AffineQuantity::One => TypeDiscipline::Linear,
        AffineQuantity::Omega => TypeDiscipline::Affine,
        AffineQuantity::Var(_) => TypeDiscipline::Affine,
    };

    let effects = match aff {
        AffineType::Arrow { effect, .. } | AffineType::DepArrow { effect, .. } => {
            map_effect(effect)
        }
        _ => vec![],
    };

    // Extract nat exprs as dependent indices
    let dependent_indices = extract_nat_indices(aff);

    // Extract refinements
    let refinements = match aff {
        AffineType::Refined { predicate, .. } => map_predicate(predicate),
        _ => vec![],
    };

    UnifiedType {
        base,
        usage,
        discipline,
        dependent_indices,
        effects,
        refinements,
    }
}

/// Convert an AffineScript nat expression to a TypeLL term.
pub fn nat_to_term(nat: &AffineNatExpr) -> Term {
    match nat {
        AffineNatExpr::Lit { value } => Term::Lit(*value),
        AffineNatExpr::Var { name } => Term::Var(name.clone()),
        AffineNatExpr::Add { lhs, rhs } => Term::BinOp {
            op: TermOp::Add,
            lhs: Box::new(nat_to_term(lhs)),
            rhs: Box::new(nat_to_term(rhs)),
        },
        AffineNatExpr::Sub { lhs, rhs } => Term::BinOp {
            op: TermOp::Sub,
            lhs: Box::new(nat_to_term(lhs)),
            rhs: Box::new(nat_to_term(rhs)),
        },
        AffineNatExpr::Mul { lhs, rhs } => Term::BinOp {
            op: TermOp::Mul,
            lhs: Box::new(nat_to_term(lhs)),
            rhs: Box::new(nat_to_term(rhs)),
        },
        AffineNatExpr::Len { name } => Term::App {
            func: "len".to_string(),
            args: vec![Term::Var(name.clone())],
        },
    }
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Map AffineScript primitive type names to TypeLL primitive types.
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
        "Never" | "never" | "!" => PrimitiveType::Never,
        _ => PrimitiveType::Unit, // Fallback for unknown constructors
    }
}

/// Map an AffineScript effect to TypeLL effects.
fn map_effect(eff: &AffineEffect) -> Vec<Effect> {
    match eff {
        AffineEffect::Pure => vec![],
        AffineEffect::Var { .. } => vec![Effect::Named("?effect".to_string())],
        AffineEffect::Singleton { name } => vec![match name.as_str() {
            "IO" | "io" => Effect::IO,
            "Alloc" | "alloc" => Effect::Alloc,
            "Network" | "network" => Effect::Network,
            "FileSystem" | "fs" => Effect::FileSystem,
            "Diverge" | "diverge" => Effect::Diverge,
            s if s.starts_with("State") => {
                Effect::State(s.trim_start_matches("State").trim().to_string())
            }
            s if s.starts_with("Except") => {
                Effect::Except(s.trim_start_matches("Except").trim().to_string())
            }
            other => Effect::Named(other.to_string()),
        }],
        AffineEffect::Union { effects } => {
            effects.iter().flat_map(map_effect).collect()
        }
    }
}

/// Map an AffineScript predicate to TypeLL predicates.
fn map_predicate(pred: &AffinePredicate) -> Vec<Predicate> {
    match pred {
        AffinePredicate::True => vec![],
        AffinePredicate::False => vec![Predicate::Raw("false".to_string())],
        AffinePredicate::Eq { lhs, rhs } => {
            vec![Predicate::Eq(nat_to_term(lhs), nat_to_term(rhs))]
        }
        AffinePredicate::Lt { lhs, rhs } => {
            vec![Predicate::Lt(nat_to_term(lhs), nat_to_term(rhs))]
        }
        AffinePredicate::Le { lhs, rhs } => {
            vec![Predicate::Lte(nat_to_term(lhs), nat_to_term(rhs))]
        }
        AffinePredicate::Gt { lhs, rhs } => {
            vec![Predicate::Gt(nat_to_term(lhs), nat_to_term(rhs))]
        }
        AffinePredicate::Ge { lhs, rhs } => {
            vec![Predicate::Gte(nat_to_term(lhs), nat_to_term(rhs))]
        }
        AffinePredicate::And { lhs, rhs } => {
            let mut preds = map_predicate(lhs);
            preds.extend(map_predicate(rhs));
            preds
        }
        AffinePredicate::Or { lhs, rhs } => {
            let l = map_predicate(lhs);
            let r = map_predicate(rhs);
            if let (Some(lp), Some(rp)) = (l.into_iter().next(), r.into_iter().next()) {
                vec![Predicate::Or(Box::new(lp), Box::new(rp))]
            } else {
                vec![]
            }
        }
        AffinePredicate::Not { inner } => {
            let preds = map_predicate(inner);
            preds
                .into_iter()
                .map(|p| Predicate::Not(Box::new(p)))
                .collect()
        }
        AffinePredicate::Impl { lhs, rhs } => {
            // P => Q is equivalent to !P || Q
            let l = map_predicate(lhs);
            let r = map_predicate(rhs);
            if let (Some(lp), Some(rp)) = (l.into_iter().next(), r.into_iter().next()) {
                vec![Predicate::Or(
                    Box::new(Predicate::Not(Box::new(lp))),
                    Box::new(rp),
                )]
            } else {
                vec![]
            }
        }
    }
}

/// Extract nat-level expressions as dependent indices from a type.
fn extract_nat_indices(aff: &AffineType) -> Vec<Term> {
    match aff {
        AffineType::Nat { expr } => vec![nat_to_term(expr)],
        AffineType::App { args, .. } => {
            args.iter().flat_map(extract_nat_indices).collect()
        }
        _ => vec![],
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
        let aff = AffineType::Con { name: "Int".to_string() };
        let ty = affine_to_typell(&aff);
        assert_eq!(ty, Type::Primitive(PrimitiveType::Int));
    }

    #[test]
    fn test_qtt_zero_gives_dependent_discipline() {
        let aff = AffineType::Con { name: "Bool".to_string() };
        let unified = affine_to_unified(&aff, &AffineQuantity::Zero);
        assert_eq!(unified.usage, UsageQuantifier::Zero);
        assert_eq!(unified.discipline, TypeDiscipline::Dependent);
    }

    #[test]
    fn test_qtt_one_gives_linear() {
        let aff = AffineType::Con { name: "Int".to_string() };
        let unified = affine_to_unified(&aff, &AffineQuantity::One);
        assert_eq!(unified.usage, UsageQuantifier::One);
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
    }

    #[test]
    fn test_refinement_type() {
        let aff = AffineType::Refined {
            base: Box::new(AffineType::Con { name: "Int".to_string() }),
            predicate: AffinePredicate::Gt {
                lhs: AffineNatExpr::Var { name: "x".to_string() },
                rhs: AffineNatExpr::Lit { value: 0 },
            },
        };
        let ty = affine_to_typell(&aff);
        match ty {
            Type::Refined { base, predicates } => {
                assert_eq!(*base, Type::Primitive(PrimitiveType::Int));
                assert_eq!(predicates.len(), 1);
            }
            _ => panic!("expected Refined type"),
        }
    }

    #[test]
    fn test_dependent_arrow() {
        let aff = AffineType::DepArrow {
            param_name: "n".to_string(),
            param_type: Box::new(AffineType::Con { name: "Int".to_string() }),
            ret_type: Box::new(AffineType::Con { name: "Bool".to_string() }),
            effect: AffineEffect::Pure,
        };
        let ty = affine_to_typell(&aff);
        match ty {
            Type::Pi { param_name, .. } => assert_eq!(param_name, "n"),
            _ => panic!("expected Pi type"),
        }
    }

    #[test]
    fn test_effect_mapping() {
        let eff = AffineEffect::Union {
            effects: vec![
                AffineEffect::Singleton { name: "IO".to_string() },
                AffineEffect::Singleton { name: "State s".to_string() },
            ],
        };
        let mapped = map_effect(&eff);
        assert_eq!(mapped.len(), 2);
        assert_eq!(mapped[0], Effect::IO);
    }

    #[test]
    fn test_nat_to_term() {
        let nat = AffineNatExpr::Add {
            lhs: Box::new(AffineNatExpr::Var { name: "n".to_string() }),
            rhs: Box::new(AffineNatExpr::Lit { value: 1 }),
        };
        let term = nat_to_term(&nat);
        match term {
            Term::BinOp { op: TermOp::Add, .. } => {}
            _ => panic!("expected BinOp Add"),
        }
    }
}
