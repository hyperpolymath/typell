// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Dimensional analysis for resource types.
//!
//! Ported from Eclexia's `eclexia-ast::dimension` module and integrated
//! into the TypeLL kernel. This module provides compile-time dimensional
//! analysis rules for arithmetic on `Resource<D>` types.
//!
//! ## Rules (from Eclexia's type checker)
//!
//! | Operation     | Left            | Right           | Result             |
//! |---------------|-----------------|-----------------|--------------------|
//! | `+`, `-`      | Resource<D>     | Resource<D>     | Resource<D>        |
//! | `*`           | Resource<D1>    | Resource<D2>    | Resource<D1*D2>    |
//! | `/`           | Resource<D1>    | Resource<D2>    | Resource<D1/D2>    |
//! | `*`           | Resource<D>     | Scalar          | Resource<D>        |
//! | `^n`          | Resource<D>     | Int literal     | Resource<D^n>      |
//! | comparison    | Resource<D>     | Resource<D>     | Bool               |
//!
//! Dimension mismatches in additive operations are type errors.

use crate::error::{Span, TypeError, TypeResult};
use crate::types::{Dimension, PrimitiveType, Term, Type};
use crate::unify::eval_term_to_i64;

/// Check dimensional compatibility for a binary operation.
///
/// Returns the result type if the operation is well-typed, or a
/// `DimensionMismatch` error otherwise.
///
/// When `exponent_term` is provided for `Pow` operations, the function
/// attempts compile-time evaluation to determine the actual exponent.
pub fn check_binary_op(
    op: DimOp,
    lhs: &Type,
    rhs: &Type,
    span: Span,
) -> TypeResult<Type> {
    check_binary_op_with_exponent(op, lhs, rhs, None, span)
}

/// Extended version that accepts an optional compile-time exponent term.
pub fn check_binary_op_with_exponent(
    op: DimOp,
    lhs: &Type,
    rhs: &Type,
    exponent_term: Option<&Term>,
    span: Span,
) -> TypeResult<Type> {
    match (lhs, rhs, op) {
        // Resource + Resource: dimensions must match
        (
            Type::Resource { base: b1, dimension: d1 },
            Type::Resource { base: _, dimension: d2 },
            DimOp::Add | DimOp::Sub,
        ) => {
            if d1 != d2 {
                return Err(TypeError::DimensionMismatch {
                    span,
                    dim1: *d1,
                    dim2: *d2,
                    hint: Some(
                        "additive operations require matching dimensions".to_string(),
                    ),
                });
            }
            Ok(Type::Resource {
                base: b1.clone(),
                dimension: *d1,
            })
        }

        // Resource * Resource: dimensions multiply
        (
            Type::Resource { base: b1, dimension: d1 },
            Type::Resource { dimension: d2, .. },
            DimOp::Mul,
        ) => Ok(Type::Resource {
            base: b1.clone(),
            dimension: d1.multiply(d2),
        }),

        // Resource / Resource: dimensions divide
        (
            Type::Resource { base: b1, dimension: d1 },
            Type::Resource { dimension: d2, .. },
            DimOp::Div,
        ) => Ok(Type::Resource {
            base: b1.clone(),
            dimension: d1.divide(d2),
        }),

        // Resource * Scalar or Scalar * Resource: dimension preserved
        (
            Type::Resource { base, dimension },
            Type::Primitive(p),
            DimOp::Mul,
        )
        | (
            Type::Primitive(p),
            Type::Resource { base, dimension },
            DimOp::Mul,
        ) if is_numeric(p) => Ok(Type::Resource {
            base: base.clone(),
            dimension: *dimension,
        }),

        // Resource / Scalar: dimension preserved
        (
            Type::Resource { base, dimension },
            Type::Primitive(p),
            DimOp::Div,
        ) if is_numeric(p) => Ok(Type::Resource {
            base: base.clone(),
            dimension: *dimension,
        }),

        // Resource ^ Int: dimension exponentiated
        (
            Type::Resource { base, dimension },
            Type::Primitive(PrimitiveType::Int | PrimitiveType::I64),
            DimOp::Pow,
        ) => {
            // Try compile-time exponent evaluation from the term
            let exp = exponent_term
                .and_then(eval_term_to_i64)
                .map(|n| n as i8);

            match exp {
                Some(n) => {
                    // Exponent known at compile time — exact dimensional result
                    Ok(Type::Resource {
                        base: base.clone(),
                        dimension: dimension.pow(n),
                    })
                }
                None => {
                    // Exponent not statically known — generate a proof obligation.
                    // For safety, require the exponent to be resolved before codegen.
                    // Return dimensionless as a conservative fallback with a hint.
                    Err(TypeError::Custom {
                        span,
                        message: "compile-time exponent required for dimensional \
                                  exponentiation"
                            .to_string(),
                        hint: Some(
                            "use a literal integer exponent or a const expression \
                             that TypeLL can evaluate at compile time"
                                .to_string(),
                        ),
                    })
                }
            }
        }

        // Resource comparison: dimensions must match, result is Bool
        (
            Type::Resource { dimension: d1, .. },
            Type::Resource { dimension: d2, .. },
            DimOp::Compare,
        ) => {
            if d1 != d2 {
                return Err(TypeError::DimensionMismatch {
                    span,
                    dim1: *d1,
                    dim2: *d2,
                    hint: Some(
                        "comparison requires matching resource dimensions".to_string(),
                    ),
                });
            }
            Ok(Type::Primitive(PrimitiveType::Bool))
        }

        // Non-resource types fall through
        _ => Err(TypeError::Custom {
            span,
            message: format!(
                "dimensional analysis not applicable: {} {:?} {}",
                lhs, op, rhs
            ),
            hint: None,
        }),
    }
}

/// Map a resource name to its dimension (Eclexia convention).
pub fn resource_name_to_dimension(name: &str) -> Option<Dimension> {
    match name {
        "energy" | "Energy" => Some(Dimension::energy()),
        "time" | "Time" | "latency" | "Latency" => Some(Dimension::time()),
        "memory" | "Memory" => Some(Dimension::memory()),
        "carbon" | "Carbon" => Some(Dimension::carbon()),
        "power" | "Power" => Some(Dimension::power()),
        "force" | "Force" => Some(Dimension::force()),
        "velocity" | "Velocity" => Some(Dimension::velocity()),
        "money" | "Money" | "currency" | "Currency" => Some(Dimension::money()),
        _ => None,
    }
}

/// Dimensional operation kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Compare,
}

/// Check whether a primitive type is numeric.
fn is_numeric(p: &PrimitiveType) -> bool {
    matches!(
        p,
        PrimitiveType::Int
            | PrimitiveType::I8
            | PrimitiveType::I16
            | PrimitiveType::I32
            | PrimitiveType::I64
            | PrimitiveType::I128
            | PrimitiveType::U8
            | PrimitiveType::U16
            | PrimitiveType::U32
            | PrimitiveType::U64
            | PrimitiveType::U128
            | PrimitiveType::Float
            | PrimitiveType::F32
            | PrimitiveType::F64
    )
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Span;

    fn energy_resource() -> Type {
        Type::Resource {
            base: Box::new(Type::Primitive(PrimitiveType::Float)),
            dimension: Dimension::energy(),
        }
    }

    fn time_resource() -> Type {
        Type::Resource {
            base: Box::new(Type::Primitive(PrimitiveType::Float)),
            dimension: Dimension::time(),
        }
    }

    #[test]
    fn test_add_same_dimension() {
        let result = check_binary_op(
            DimOp::Add,
            &energy_resource(),
            &energy_resource(),
            Span::synthetic(),
        );
        assert!(result.is_ok());
        if let Type::Resource { dimension, .. } = result.expect("TODO: handle error") {
            assert_eq!(dimension, Dimension::energy());
        } else {
            panic!("expected Resource type");
        }
    }

    #[test]
    fn test_add_different_dimension() {
        let result = check_binary_op(
            DimOp::Add,
            &energy_resource(),
            &time_resource(),
            Span::synthetic(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_mul_gives_combined_dimension() {
        let result = check_binary_op(
            DimOp::Mul,
            &energy_resource(),
            &time_resource(),
            Span::synthetic(),
        );
        assert!(result.is_ok());
        if let Type::Resource { dimension, .. } = result.expect("TODO: handle error") {
            // Energy * Time = kg*m^2/s^2 * s = kg*m^2/s
            let expected = Dimension::energy().multiply(&Dimension::time());
            assert_eq!(dimension, expected);
        }
    }

    #[test]
    fn test_div_energy_by_time_gives_power() {
        let result = check_binary_op(
            DimOp::Div,
            &energy_resource(),
            &time_resource(),
            Span::synthetic(),
        );
        assert!(result.is_ok());
        if let Type::Resource { dimension, .. } = result.expect("TODO: handle error") {
            assert_eq!(dimension, Dimension::power());
        }
    }

    #[test]
    fn test_resource_times_scalar() {
        let result = check_binary_op(
            DimOp::Mul,
            &energy_resource(),
            &Type::Primitive(PrimitiveType::Float),
            Span::synthetic(),
        );
        assert!(result.is_ok());
        if let Type::Resource { dimension, .. } = result.expect("TODO: handle error") {
            assert_eq!(dimension, Dimension::energy());
        }
    }

    #[test]
    fn test_resource_name_lookup() {
        assert_eq!(resource_name_to_dimension("energy"), Some(Dimension::energy()));
        assert_eq!(resource_name_to_dimension("time"), Some(Dimension::time()));
        assert_eq!(resource_name_to_dimension("unknown"), None);
    }

    // ========================================================================
    // Compile-time exponent evaluation tests
    // ========================================================================

    #[test]
    fn test_pow_with_literal_exponent() {
        // energy^3 with known exponent
        let result = check_binary_op_with_exponent(
            DimOp::Pow,
            &energy_resource(),
            &Type::Primitive(PrimitiveType::Int),
            Some(&Term::Lit(3)),
            Span::synthetic(),
        );
        assert!(result.is_ok());
        if let Type::Resource { dimension, .. } = result.expect("TODO: handle error") {
            assert_eq!(dimension, Dimension::energy().pow(3));
        }
    }

    #[test]
    fn test_pow_with_computed_exponent() {
        // energy^(1+2) = energy^3
        let exp = Term::BinOp {
            op: crate::types::TermOp::Add,
            lhs: Box::new(Term::Lit(1)),
            rhs: Box::new(Term::Lit(2)),
        };
        let result = check_binary_op_with_exponent(
            DimOp::Pow,
            &energy_resource(),
            &Type::Primitive(PrimitiveType::Int),
            Some(&exp),
            Span::synthetic(),
        );
        assert!(result.is_ok());
        if let Type::Resource { dimension, .. } = result.expect("TODO: handle error") {
            assert_eq!(dimension, Dimension::energy().pow(3));
        }
    }

    #[test]
    fn test_pow_with_variable_exponent_fails() {
        // energy^n where n is a variable — cannot evaluate at compile time
        let result = check_binary_op_with_exponent(
            DimOp::Pow,
            &energy_resource(),
            &Type::Primitive(PrimitiveType::Int),
            Some(&Term::Var("n".to_string())),
            Span::synthetic(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_pow_without_exponent_term_fails() {
        // No exponent term provided — cannot determine
        let result = check_binary_op_with_exponent(
            DimOp::Pow,
            &energy_resource(),
            &Type::Primitive(PrimitiveType::Int),
            None,
            Span::synthetic(),
        );
        assert!(result.is_err());
    }
}
