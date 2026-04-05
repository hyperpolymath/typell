// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from VCL-total query types to TypeLL's unified types.
//!
//! Maps VCL-total's 10-level type safety hierarchy into TypeLL concepts.
//! Legacy VCL-dt++ extensions are preserved and mapped to their
//! corresponding VCL-total levels:
//!
//! | VCL-total Level | VCL-dt++ Extension          | TypeLL Concept                    |
//! |--------------|-----------------------------|-----------------------------------|
//! | 5            | `PROOF ATTACHED thm`        | Refinement predicate              |
//! | 8            | `EFFECTS { Read, Write }`   | `Effect::Named("Read")` etc.      |
//! | 9            | `WITH SESSION ReadOnly`     | `SessionType::Recv` chain         |
//! | 9            | `IN TRANSACTION Active`     | `Type::Named("TxState:Active")`   |
//! | 10           | `CONSUME AFTER N USE`       | `UsageQuantifier::Bounded(n)`     |
//! | 10           | `USAGE LIMIT n`             | `UsageQuantifier::Bounded(n)`     |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, Predicate, SessionType, Type, TypeDiscipline,
    UnifiedType, UsageQuantifier,
};

use crate::levels::{SafetyLevel, SafetyReport, LevelCheck, QueryPath};

/// VCL modality (one of the 8 query modalities).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VqlModality {
    Graph,
    Vector,
    Tensor,
    Semantic,
    Document,
    Temporal,
    Provenance,
    Spatial,
    All,
}

/// VCL session protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VqlSessionProtocol {
    ReadOnly,
    Mutation,
    Stream,
    Batch,
    Custom(String),
}

/// VCL effect label.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VqlEffectLabel {
    Read,
    Write,
    Cite,
    Audit,
    Transform,
    Federate,
    Custom(String),
}

/// VCL transaction state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VqlTransactionState {
    Fresh,
    Active,
    Committed,
    RolledBack,
    ReadSnapshot,
    Custom(String),
}

/// VCL-dt++ extension annotations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VqlExtensions {
    pub consume_after: Option<u64>,
    pub session_protocol: Option<VqlSessionProtocol>,
    pub effects: Option<Vec<VqlEffectLabel>>,
    pub transaction_state: Option<VqlTransactionState>,
    pub proof_attached: Option<String>,
    pub usage_limit: Option<u64>,
}

/// A VCL query type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VqlQueryType {
    pub modalities: Vec<VqlModality>,
    pub result_fields: Vec<String>,
    pub extensions: VqlExtensions,
}

/// Convert a VCL query type to a TypeLL base type.
///
/// A VCL query result is modelled as a Named type parameterised by its
/// modalities and result fields.
pub fn vcl_to_typell(vcl: &VqlQueryType) -> Type {
    Type::Named {
        name: "QueryResult".to_string(),
        args: vcl
            .modalities
            .iter()
            .map(|m| Type::Named {
                name: format!("Modality:{}", modality_name(m)),
                args: vec![],
            })
            .collect(),
    }
}

/// Convert a VCL query type with extensions to a full TypeLL unified type.
pub fn vcl_to_unified(vcl: &VqlQueryType) -> UnifiedType {
    let base = vcl_to_typell(vcl);

    // Usage from CONSUME AFTER or USAGE LIMIT
    let usage = if let Some(n) = vcl.extensions.consume_after {
        UsageQuantifier::Bounded(n)
    } else if let Some(n) = vcl.extensions.usage_limit {
        UsageQuantifier::Bounded(n)
    } else {
        UsageQuantifier::Omega
    };

    // Discipline based on usage
    let discipline = if matches!(usage, UsageQuantifier::Bounded(_)) {
        TypeDiscipline::Linear
    } else {
        TypeDiscipline::Unrestricted
    };

    // Effects from EFFECTS clause
    let effects = vcl
        .extensions
        .effects
        .as_ref()
        .map(|labels| labels.iter().map(map_vql_effect).collect())
        .unwrap_or_default();

    // Proof as refinement
    let refinements = vcl
        .extensions
        .proof_attached
        .as_ref()
        .map(|thm| vec![Predicate::Raw(format!("proof:{}", thm))])
        .unwrap_or_default();

    UnifiedType {
        base,
        usage,
        discipline,
        dependent_indices: Vec::new(),
        effects,
        refinements,
    }
}

/// Convert a VCL session protocol to a TypeLL session type.
pub fn session_protocol_to_session(proto: &VqlSessionProtocol) -> SessionType {
    match proto {
        VqlSessionProtocol::ReadOnly => SessionType::Recv(
            Box::new(Type::Named {
                name: "QueryResult".to_string(),
                args: vec![],
            }),
            Box::new(SessionType::End),
        ),
        VqlSessionProtocol::Mutation => SessionType::Send(
            Box::new(Type::Named {
                name: "Mutation".to_string(),
                args: vec![],
            }),
            Box::new(SessionType::Recv(
                Box::new(Type::Named {
                    name: "MutationResult".to_string(),
                    args: vec![],
                }),
                Box::new(SessionType::End),
            )),
        ),
        VqlSessionProtocol::Stream => SessionType::Rec(
            "stream".to_string(),
            Box::new(SessionType::Recv(
                Box::new(Type::Named {
                    name: "StreamItem".to_string(),
                    args: vec![],
                }),
                Box::new(SessionType::RecVar("stream".to_string())),
            )),
        ),
        VqlSessionProtocol::Batch => SessionType::Send(
            Box::new(Type::Named {
                name: "BatchRequest".to_string(),
                args: vec![],
            }),
            Box::new(SessionType::Recv(
                Box::new(Type::Named {
                    name: "BatchResult".to_string(),
                    args: vec![],
                }),
                Box::new(SessionType::End),
            )),
        ),
        VqlSessionProtocol::Custom(name) => SessionType::Send(
            Box::new(Type::Named {
                name: format!("Custom:{}", name),
                args: vec![],
            }),
            Box::new(SessionType::End),
        ),
    }
}

/// Map a VCL effect label to a TypeLL effect.
fn map_vql_effect(label: &VqlEffectLabel) -> Effect {
    match label {
        VqlEffectLabel::Read => Effect::IO,
        VqlEffectLabel::Write => Effect::State("write".to_string()),
        VqlEffectLabel::Cite => Effect::Named("Cite".to_string()),
        VqlEffectLabel::Audit => Effect::Named("Audit".to_string()),
        VqlEffectLabel::Transform => Effect::Named("Transform".to_string()),
        VqlEffectLabel::Federate => Effect::Network,
        VqlEffectLabel::Custom(name) => Effect::Named(name.clone()),
    }
}

fn modality_name(m: &VqlModality) -> &str {
    match m {
        VqlModality::Graph => "Graph",
        VqlModality::Vector => "Vector",
        VqlModality::Tensor => "Tensor",
        VqlModality::Semantic => "Semantic",
        VqlModality::Document => "Document",
        VqlModality::Temporal => "Temporal",
        VqlModality::Provenance => "Provenance",
        VqlModality::Spatial => "Spatial",
        VqlModality::All => "All",
    }
}

/// Determine the VCL-total safety level achieved by a query.
///
/// Checks each level in order and stops at the first failure.
/// Returns a safety report with per-level diagnostics.
pub fn determine_safety_level(vcl: &VqlQueryType) -> SafetyReport {
    let mut checks = Vec::new();
    let mut max_level = SafetyLevel::ParseTime;

    // Level 1: Parse-time safety — if we have a VqlQueryType, parsing succeeded.
    checks.push(LevelCheck {
        level: SafetyLevel::ParseTime,
        passed: true,
        diagnostic: String::new(),
    });

    // Level 2: Schema-binding — result fields must be non-empty (schema resolved).
    let l2_pass = !vcl.result_fields.is_empty();
    checks.push(LevelCheck {
        level: SafetyLevel::SchemaBinding,
        passed: l2_pass,
        diagnostic: if l2_pass { String::new() } else { "No result fields bound to schema".to_string() },
    });
    if l2_pass { max_level = SafetyLevel::SchemaBinding; }

    // Level 3: Type-compatible operations — modalities are valid enum variants.
    let l3_pass = !vcl.modalities.is_empty();
    checks.push(LevelCheck {
        level: SafetyLevel::TypeCompatible,
        passed: l3_pass,
        diagnostic: if l3_pass { String::new() } else { "No modalities specified".to_string() },
    });
    if l2_pass && l3_pass { max_level = SafetyLevel::TypeCompatible; }

    // Level 4: Null-safety — all fields are typed (no raw strings without schema).
    // At this stage we treat schema-bound fields as null-safe.
    let l4_pass = l2_pass && l3_pass;
    checks.push(LevelCheck {
        level: SafetyLevel::NullSafe,
        passed: l4_pass,
        diagnostic: if l4_pass { String::new() } else { "Schema binding required for null-safety".to_string() },
    });
    if l4_pass { max_level = SafetyLevel::NullSafe; }

    // Level 5: Injection-proof — proof attachment provides refinement predicates.
    let l5_pass = l4_pass && vcl.extensions.proof_attached.is_some();
    checks.push(LevelCheck {
        level: SafetyLevel::InjectionProof,
        passed: l5_pass,
        diagnostic: if l5_pass { String::new() } else { "PROOF ATTACHED clause required for injection-proof safety".to_string() },
    });
    if l5_pass { max_level = SafetyLevel::InjectionProof; }

    // Level 6: Result-type safety — always passes if schema-bound (type is inferred).
    let l6_pass = l4_pass;
    checks.push(LevelCheck {
        level: SafetyLevel::ResultType,
        passed: l6_pass,
        diagnostic: if l6_pass { String::new() } else { "Result type cannot be inferred".to_string() },
    });
    if l5_pass && l6_pass { max_level = SafetyLevel::ResultType; }

    // Level 7: Cardinality safety — usage limit provides bounded quantifiers.
    let l7_pass = l6_pass && vcl.extensions.usage_limit.is_some();
    checks.push(LevelCheck {
        level: SafetyLevel::Cardinality,
        passed: l7_pass,
        diagnostic: if l7_pass { String::new() } else { "USAGE LIMIT clause required for cardinality safety".to_string() },
    });
    if l5_pass && l7_pass { max_level = SafetyLevel::Cardinality; }

    // Level 8: Effect-tracking — effects clause required.
    let l8_pass = l7_pass && vcl.extensions.effects.as_ref().map_or(false, |e| !e.is_empty());
    checks.push(LevelCheck {
        level: SafetyLevel::EffectTracking,
        passed: l8_pass,
        diagnostic: if l8_pass { String::new() } else { "EFFECTS clause required for effect-tracking safety".to_string() },
    });
    if l8_pass { max_level = SafetyLevel::EffectTracking; }

    // Level 9: Temporal safety — session protocol or transaction state required.
    let l9_pass = l8_pass && (vcl.extensions.session_protocol.is_some()
                              || vcl.extensions.transaction_state.is_some());
    checks.push(LevelCheck {
        level: SafetyLevel::Temporal,
        passed: l9_pass,
        diagnostic: if l9_pass { String::new() } else { "WITH SESSION or IN TRANSACTION clause required for temporal safety".to_string() },
    });
    if l9_pass { max_level = SafetyLevel::Temporal; }

    // Level 10: Linearity safety — consume_after or usage_limit with linear discipline.
    let l10_pass = l9_pass && vcl.extensions.consume_after.is_some();
    checks.push(LevelCheck {
        level: SafetyLevel::Linearity,
        passed: l10_pass,
        diagnostic: if l10_pass { String::new() } else { "CONSUME AFTER clause required for linearity safety".to_string() },
    });
    if l10_pass { max_level = SafetyLevel::Linearity; }

    // Determine query path based on max level.
    let query_path = if max_level.as_u8() >= 7 {
        QueryPath::Ut
    } else if max_level.as_u8() >= 2 {
        QueryPath::Dt
    } else {
        QueryPath::Slipstream
    };

    SafetyReport {
        max_level,
        checks,
        query_path,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query_type() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Graph, VqlModality::Vector],
            result_fields: vec!["name".to_string()],
            extensions: VqlExtensions::default(),
        };
        let ty = vcl_to_typell(&vcl);
        match ty {
            Type::Named { name, args } => {
                assert_eq!(name, "QueryResult");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("expected Named type"),
        }
    }

    #[test]
    fn test_consume_after_gives_bounded_usage() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Graph],
            result_fields: vec![],
            extensions: VqlExtensions {
                consume_after: Some(3),
                ..Default::default()
            },
        };
        let unified = vcl_to_unified(&vcl);
        assert_eq!(unified.usage, UsageQuantifier::Bounded(3));
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
    }

    #[test]
    fn test_effects_clause() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Document],
            result_fields: vec![],
            extensions: VqlExtensions {
                effects: Some(vec![VqlEffectLabel::Read, VqlEffectLabel::Write]),
                ..Default::default()
            },
        };
        let unified = vcl_to_unified(&vcl);
        assert_eq!(unified.effects.len(), 2);
        assert_eq!(unified.effects[0], Effect::IO);
    }

    #[test]
    fn test_proof_attached_creates_refinement() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Provenance],
            result_fields: vec![],
            extensions: VqlExtensions {
                proof_attached: Some("integrity_theorem".to_string()),
                ..Default::default()
            },
        };
        let unified = vcl_to_unified(&vcl);
        assert_eq!(unified.refinements.len(), 1);
    }

    #[test]
    fn test_read_only_session() {
        let session = session_protocol_to_session(&VqlSessionProtocol::ReadOnly);
        assert!(matches!(session, SessionType::Recv(_, _)));
    }

    #[test]
    fn test_stream_session_is_recursive() {
        let session = session_protocol_to_session(&VqlSessionProtocol::Stream);
        assert!(matches!(session, SessionType::Rec(_, _)));
    }

    #[test]
    fn test_safety_level_basic_query() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Graph],
            result_fields: vec!["name".to_string()],
            extensions: VqlExtensions::default(),
        };
        let report = determine_safety_level(&vcl);
        assert_eq!(report.max_level, SafetyLevel::NullSafe);
        assert_eq!(report.query_path, QueryPath::Dt);
    }

    #[test]
    fn test_safety_level_full_ut() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Graph],
            result_fields: vec!["name".to_string()],
            extensions: VqlExtensions {
                consume_after: Some(3),
                session_protocol: Some(VqlSessionProtocol::ReadOnly),
                effects: Some(vec![VqlEffectLabel::Read]),
                transaction_state: None,
                proof_attached: Some("integrity".to_string()),
                usage_limit: Some(10),
            },
        };
        let report = determine_safety_level(&vcl);
        assert_eq!(report.max_level, SafetyLevel::Linearity);
        assert_eq!(report.query_path, QueryPath::Ut);
        assert_eq!(report.checks.len(), 10);
        assert!(report.checks.iter().all(|c| c.passed));
    }

    #[test]
    fn test_safety_level_partial_ut() {
        let vcl = VqlQueryType {
            modalities: vec![VqlModality::Document],
            result_fields: vec!["content".to_string()],
            extensions: VqlExtensions {
                effects: Some(vec![VqlEffectLabel::Read, VqlEffectLabel::Write]),
                proof_attached: Some("access_control".to_string()),
                usage_limit: Some(5),
                ..Default::default()
            },
        };
        let report = determine_safety_level(&vcl);
        // Has effects + proof + usage_limit but no session/consume_after
        assert_eq!(report.max_level, SafetyLevel::EffectTracking);
        assert_eq!(report.query_path, QueryPath::Ut);
    }
}
