// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from Phronesis's policy types to TypeLL's unified types.
//!
//! ## Phronesis Type → TypeLL Mapping
//!
//! | Phronesis Type       | TypeLL Type                              |
//! |----------------------|------------------------------------------|
//! | `:integer`           | `Type::Primitive(PrimitiveType::Int)`    |
//! | `:float`             | `Type::Primitive(PrimitiveType::Float)`  |
//! | `:string`            | `Type::Primitive(PrimitiveType::String)` |
//! | `:boolean`           | `Type::Primitive(PrimitiveType::Bool)`   |
//! | `:ip_address`        | `Type::Named("IpAddress", [])`           |
//! | `:datetime`          | `Type::Named("DateTime", [])`            |
//! | `{:policy, ..}`      | `Type::Refined` + effects                |
//! | `{:execute, ..}`     | `Effect::Named("Execute")`               |
//! | `{:reject, ..}`      | `Effect::Named("Reject")`                |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, PrimitiveType, Type, TypeDiscipline, UnifiedType, UsageQuantifier,
};

/// A Phronesis literal type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhronesisLiteralType {
    Integer,
    Float,
    String,
    Boolean,
    IpAddress,
    DateTime,
}

/// A Phronesis type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum PhronesisType {
    Literal { lit_type: PhronesisLiteralType },
    Policy { name: String, priority: u32 },
    Action { action_type: PhronesisAction },
    Expression,
    Condition,
}

/// Phronesis action types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PhronesisAction {
    Execute,
    Report,
    Reject,
    Accept,
    Block,
    Conditional,
}

/// Convert a Phronesis type to a TypeLL base type.
pub fn phronesis_to_typell(phr: &PhronesisType) -> Type {
    match phr {
        PhronesisType::Literal { lit_type } => Type::Primitive(match lit_type {
            PhronesisLiteralType::Integer => PrimitiveType::Int,
            PhronesisLiteralType::Float => PrimitiveType::Float,
            PhronesisLiteralType::String => PrimitiveType::String,
            PhronesisLiteralType::Boolean => PrimitiveType::Bool,
            PhronesisLiteralType::IpAddress => return Type::Named {
                name: "IpAddress".to_string(),
                args: vec![],
            },
            PhronesisLiteralType::DateTime => return Type::Named {
                name: "DateTime".to_string(),
                args: vec![],
            },
        }),
        PhronesisType::Policy { name, .. } => Type::Named {
            name: format!("Policy:{}", name),
            args: vec![],
        },
        PhronesisType::Action { action_type } => Type::Named {
            name: format!("Action:{}", action_name(action_type)),
            args: vec![],
        },
        PhronesisType::Expression => Type::Named {
            name: "Expr".to_string(),
            args: vec![],
        },
        PhronesisType::Condition => Type::Primitive(PrimitiveType::Bool),
    }
}

/// Convert a Phronesis type to a full TypeLL unified type.
///
/// Policy actions carry effects (Execute = IO, Reject = exception-like).
pub fn phronesis_to_unified(phr: &PhronesisType) -> UnifiedType {
    let base = phronesis_to_typell(phr);

    let effects = match phr {
        PhronesisType::Action { action_type } => match action_type {
            PhronesisAction::Execute => vec![Effect::IO],
            PhronesisAction::Report => vec![Effect::IO],
            PhronesisAction::Reject => vec![Effect::Except("PolicyReject".to_string())],
            PhronesisAction::Accept => vec![],
            PhronesisAction::Block => vec![Effect::IO],
            PhronesisAction::Conditional => vec![],
        },
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

fn action_name(action: &PhronesisAction) -> &str {
    match action {
        PhronesisAction::Execute => "Execute",
        PhronesisAction::Report => "Report",
        PhronesisAction::Reject => "Reject",
        PhronesisAction::Accept => "Accept",
        PhronesisAction::Block => "Block",
        PhronesisAction::Conditional => "Conditional",
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_integer_literal() {
        let ty = phronesis_to_typell(&PhronesisType::Literal {
            lit_type: PhronesisLiteralType::Integer,
        });
        assert_eq!(ty, Type::Primitive(PrimitiveType::Int));
    }

    #[test]
    fn test_ip_address_type() {
        let ty = phronesis_to_typell(&PhronesisType::Literal {
            lit_type: PhronesisLiteralType::IpAddress,
        });
        match ty {
            Type::Named { name, .. } => assert_eq!(name, "IpAddress"),
            _ => panic!("expected Named type"),
        }
    }

    #[test]
    fn test_reject_action_has_except_effect() {
        let unified = phronesis_to_unified(&PhronesisType::Action {
            action_type: PhronesisAction::Reject,
        });
        assert_eq!(unified.effects.len(), 1);
        assert!(matches!(&unified.effects[0], Effect::Except(s) if s == "PolicyReject"));
    }

    #[test]
    fn test_execute_action_has_io_effect() {
        let unified = phronesis_to_unified(&PhronesisType::Action {
            action_type: PhronesisAction::Execute,
        });
        assert_eq!(unified.effects[0], Effect::IO);
    }
}
