// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from Oblibeny's type representations to TypeLL's unified types.
//!
//! ## Oblibeny Type → TypeLL Mapping
//!
//! | Oblibeny Type          | TypeLL Type                              |
//! |------------------------|------------------------------------------|
//! | `TPrim TI32`           | `Type::Primitive(PrimitiveType::I32)`    |
//! | `TPrim TI64`           | `Type::Primitive(PrimitiveType::I64)`    |
//! | `TPrim TU32`           | `Type::Primitive(PrimitiveType::U32)`    |
//! | `TPrim TU64`           | `Type::Primitive(PrimitiveType::U64)`    |
//! | `TPrim TBool`          | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `TPrim TUnit`          | `Type::Primitive(PrimitiveType::Unit)`   |
//! | `TArray(t, Some n)`    | `Type::Array { elem: t, length: n }`    |
//! | `TRef(t)`              | `Type::Named("MutRef", [t])`             |
//! | `TFun(args, ret)`      | `Type::Function { .. }`                  |
//! | `TStruct(name)`        | `Type::Named(name, [])`                  |
//! | `TTrace`               | `Type::Named("Trace", [])`               |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, PrimitiveType, Term, Type, TypeDiscipline, UnifiedType, UsageQuantifier,
};

/// An Oblibeny primitive type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OblibenyPrim {
    I32,
    I64,
    U32,
    U64,
    Bool,
    Unit,
}

/// An Oblibeny type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum OblibenyType {
    Prim { prim: OblibenyPrim },
    Array { elem: Box<OblibenyType>, size: Option<u64> },
    Ref { inner: Box<OblibenyType> },
    Fun { params: Vec<OblibenyType>, ret: Box<OblibenyType> },
    Struct { name: String },
    Trace,
}

/// Whether the code is in constrained (reversible) form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OblibenyForm {
    /// Constrained form: Turing-incomplete, reversible, accountable.
    Constrained,
    /// Factory form: Turing-complete, unrestricted.
    Factory,
}

/// Convert an Oblibeny type to a TypeLL base type.
pub fn oblibeny_to_typell(obl: &OblibenyType) -> Type {
    match obl {
        OblibenyType::Prim { prim } => Type::Primitive(match prim {
            OblibenyPrim::I32 => PrimitiveType::I32,
            OblibenyPrim::I64 => PrimitiveType::I64,
            OblibenyPrim::U32 => PrimitiveType::U32,
            OblibenyPrim::U64 => PrimitiveType::U64,
            OblibenyPrim::Bool => PrimitiveType::Bool,
            OblibenyPrim::Unit => PrimitiveType::Unit,
        }),
        OblibenyType::Array { elem, size } => Type::Array {
            elem: Box::new(oblibeny_to_typell(elem)),
            length: size.map(|s| Term::Lit(s as i64)),
        },
        OblibenyType::Ref { inner } => Type::Named {
            name: "MutRef".to_string(),
            args: vec![oblibeny_to_typell(inner)],
        },
        OblibenyType::Fun { params, ret } => Type::Function {
            params: params.iter().map(oblibeny_to_typell).collect(),
            ret: Box::new(oblibeny_to_typell(ret)),
            effects: vec![],
        },
        OblibenyType::Struct { name } => Type::Named {
            name: name.clone(),
            args: vec![],
        },
        OblibenyType::Trace => Type::Named {
            name: "Trace".to_string(),
            args: vec![],
        },
    }
}

/// Convert an Oblibeny type to a full TypeLL unified type.
///
/// Constrained-form values are linear (reversible operations require
/// exactly-once usage). Factory-form values are unrestricted.
/// Trace types always carry an Audit effect.
pub fn oblibeny_to_unified(obl: &OblibenyType, form: &OblibenyForm) -> UnifiedType {
    let base = oblibeny_to_typell(obl);

    let (discipline, usage) = match form {
        OblibenyForm::Constrained => (TypeDiscipline::Linear, UsageQuantifier::One),
        OblibenyForm::Factory => (TypeDiscipline::Unrestricted, UsageQuantifier::Omega),
    };

    let effects = match obl {
        OblibenyType::Trace => vec![Effect::Named("Audit".to_string())],
        OblibenyType::Ref { .. } => vec![Effect::State("mut".to_string())],
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitive_i64() {
        let ty = oblibeny_to_typell(&OblibenyType::Prim { prim: OblibenyPrim::I64 });
        assert_eq!(ty, Type::Primitive(PrimitiveType::I64));
    }

    #[test]
    fn test_constrained_form_is_linear() {
        let ty = OblibenyType::Prim { prim: OblibenyPrim::I64 };
        let unified = oblibeny_to_unified(&ty, &OblibenyForm::Constrained);
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
        assert_eq!(unified.usage, UsageQuantifier::One);
    }

    #[test]
    fn test_factory_form_is_unrestricted() {
        let ty = OblibenyType::Prim { prim: OblibenyPrim::I64 };
        let unified = oblibeny_to_unified(&ty, &OblibenyForm::Factory);
        assert_eq!(unified.discipline, TypeDiscipline::Unrestricted);
        assert_eq!(unified.usage, UsageQuantifier::Omega);
    }

    #[test]
    fn test_trace_carries_audit_effect() {
        let unified = oblibeny_to_unified(&OblibenyType::Trace, &OblibenyForm::Constrained);
        assert_eq!(unified.effects.len(), 1);
        assert_eq!(unified.effects[0], Effect::Named("Audit".to_string()));
    }

    #[test]
    fn test_array_with_size() {
        let ty = OblibenyType::Array {
            elem: Box::new(OblibenyType::Prim { prim: OblibenyPrim::I32 }),
            size: Some(10),
        };
        let result = oblibeny_to_typell(&ty);
        match result {
            Type::Array { length: Some(Term::Lit(10)), .. } => {}
            _ => panic!("expected Array with length 10"),
        }
    }
}
