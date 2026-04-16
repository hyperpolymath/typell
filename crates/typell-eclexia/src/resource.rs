// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Eclexia-specific resource type checking rules.
//!
//! Eclexia's resource types track shadow prices, energy consumption,
//! carbon emissions, and other sustainability metrics at the type level.
//! This module provides Eclexia-specific rules on top of TypeLL's
//! general dimensional analysis.
//!
//! ## Shadow Price Types
//!
//! Eclexia assigns shadow prices to computational resources:
//!
//! | Resource | Default Shadow Price | Dimension          |
//! |----------|---------------------|--------------------|
//! | Energy   | 0.000033 $/J        | mass·length²/time² |
//! | Time     | 0.001 $/s           | time               |
//! | Carbon   | 0.00005 $/gCO2e     | carbon             |
//! | Memory   | variable            | information        |
//! | Power    | derived             | mass·length²/time³ |
//!
//! ## Linear Resource Tracking
//!
//! Resources in Eclexia should be consumed exactly once (linear types).
//! TypeLL enforces this through its QTT-based usage tracking, adding
//! a guarantee that Eclexia's runtime resource tracker previously
//! could only check dynamically.

use typell_core::dimensional::{self, DimOp};
use typell_core::error::{Span, TypeError, TypeResult};
use typell_core::types::{
    Dimension, Effect, PrimitiveType, Type, UnifiedType, UsageQuantifier,
};

/// Default shadow price rates ($/unit in SI base).
pub struct ShadowPriceDefaults;

impl ShadowPriceDefaults {
    /// Energy shadow price: 0.000033 $/J
    pub const ENERGY: f64 = 0.000_033;
    /// Time shadow price: 0.001 $/s
    pub const TIME: f64 = 0.001;
    /// Carbon shadow price: 0.00005 $/gCO2e
    pub const CARBON: f64 = 0.000_05;
}

/// Create a TypeLL resource type for an Eclexia resource name.
///
/// Maps common Eclexia resource names (energy, time, carbon, etc.)
/// to TypeLL `Resource<Float, Dimension>` types.
pub fn resource_type_for_name(name: &str) -> Option<Type> {
    dimensional::resource_name_to_dimension(name).map(|dim| Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: dim,
    })
}

/// Create a unified resource type with linear usage tracking.
///
/// Resources in Eclexia are consumed exactly once — creating them
/// allocates a cost, and consuming them finalises the accounting.
pub fn linear_resource_type(name: &str) -> Option<UnifiedType> {
    resource_type_for_name(name).map(|base| UnifiedType {
        base,
        usage: UsageQuantifier::One,
        discipline: typell_core::types::TypeDiscipline::Linear,
        dependent_indices: Vec::new(),
        effects: vec![Effect::Alloc], // Resource creation is an allocation effect
        refinements: Vec::new(),
    })
}

/// Check that a resource binary operation is dimensionally valid.
///
/// Delegates to `typell_core::dimensional::check_binary_op` with
/// Eclexia-specific error messages.
pub fn check_resource_op(
    op: &str,
    lhs: &Type,
    rhs: &Type,
    span: Span,
) -> TypeResult<Type> {
    let dim_op = match op {
        "+" | "Add" => DimOp::Add,
        "-" | "Sub" => DimOp::Sub,
        "*" | "Mul" => DimOp::Mul,
        "/" | "Div" => DimOp::Div,
        "**" | "^" | "Pow" => DimOp::Pow,
        "<" | ">" | "<=" | ">=" | "==" | "!=" => DimOp::Compare,
        _ => {
            return Err(TypeError::Custom {
                span,
                message: format!("unsupported resource operation: {}", op),
                hint: None,
            });
        }
    };

    dimensional::check_binary_op(dim_op, lhs, rhs, span)
}

/// Get the dimension for an Eclexia unit suffix.
///
/// Maps Eclexia's unit literal suffixes (s, ms, J, kWh, gCO2e, etc.)
/// to their TypeLL dimensions.
pub fn unit_suffix_to_dimension(suffix: &str) -> Option<Dimension> {
    match suffix {
        // Time
        "s" | "ms" | "us" | "ns" | "min" | "h" => Some(Dimension::time()),
        // Energy
        "J" | "mJ" | "kJ" | "Wh" | "kWh" => Some(Dimension::energy()),
        // Power
        "W" | "mW" | "kW" => Some(Dimension::power()),
        // Carbon
        "gCO2" | "gCO2e" | "kgCO2" | "kgCO2e" | "tCO2" | "tCO2e" => {
            Some(Dimension::carbon())
        }
        // Memory
        "b" | "B" | "KB" | "MB" | "GB" | "KiB" | "MiB" | "GiB" => {
            Some(Dimension::information())
        }
        _ => None,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resource_type_for_energy() {
        let ty = resource_type_for_name("energy").expect("TODO: handle error");
        match ty {
            Type::Resource { dimension, .. } => {
                assert_eq!(dimension, Dimension::energy());
            }
            _ => panic!("expected Resource type"),
        }
    }

    #[test]
    fn test_linear_resource_is_linear() {
        let unified = linear_resource_type("energy").expect("TODO: handle error");
        assert_eq!(unified.usage, UsageQuantifier::One);
    }

    #[test]
    fn test_unit_suffix_lookup() {
        assert_eq!(unit_suffix_to_dimension("J"), Some(Dimension::energy()));
        assert_eq!(unit_suffix_to_dimension("s"), Some(Dimension::time()));
        assert_eq!(unit_suffix_to_dimension("gCO2e"), Some(Dimension::carbon()));
        assert_eq!(unit_suffix_to_dimension("MB"), Some(Dimension::information()));
        assert_eq!(unit_suffix_to_dimension("xyz"), None);
    }

    #[test]
    fn test_resource_addition_same_dimension() {
        let energy = resource_type_for_name("energy").expect("TODO: handle error");
        let result = check_resource_op("+", &energy, &energy, Span::synthetic());
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_addition_different_dimension() {
        let energy = resource_type_for_name("energy").expect("TODO: handle error");
        let time = resource_type_for_name("time").expect("TODO: handle error");
        let result = check_resource_op("+", &energy, &time, Span::synthetic());
        assert!(result.is_err());
    }

    #[test]
    fn test_energy_div_time_gives_power() {
        let energy = resource_type_for_name("energy").expect("TODO: handle error");
        let time = resource_type_for_name("time").expect("TODO: handle error");
        let result = check_resource_op("/", &energy, &time, Span::synthetic()).expect("TODO: handle error");
        match result {
            Type::Resource { dimension, .. } => {
                assert_eq!(dimension, Dimension::power());
            }
            _ => panic!("expected Resource type"),
        }
    }
}
