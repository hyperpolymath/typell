// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Bridge from VQL-dt++ query types to TypeLL's unified types.
//!
//! ## VQL Extension → TypeLL Mapping
//!
//! | VQL Extension             | TypeLL Concept                           |
//! |---------------------------|------------------------------------------|
//! | `CONSUME AFTER N USE`     | `UsageQuantifier::Bounded(n)`            |
//! | `WITH SESSION ReadOnly`   | `SessionType::Recv` chain                |
//! | `EFFECTS { Read, Write }` | `Effect::Named("Read")` etc.             |
//! | `IN TRANSACTION Active`   | `Type::Named("TxState:Active", [])`      |
//! | `PROOF ATTACHED thm`      | Refinement predicate                     |
//! | `USAGE LIMIT n`           | `UsageQuantifier::Bounded(n)`            |

use serde::{Deserialize, Serialize};
use typell_core::types::{
    Effect, Predicate, SessionType, Type, TypeDiscipline,
    UnifiedType, UsageQuantifier,
};

/// VQL modality (one of the 8 query modalities).
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

/// VQL session protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VqlSessionProtocol {
    ReadOnly,
    Mutation,
    Stream,
    Batch,
    Custom(String),
}

/// VQL effect label.
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

/// VQL transaction state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VqlTransactionState {
    Fresh,
    Active,
    Committed,
    RolledBack,
    ReadSnapshot,
    Custom(String),
}

/// VQL-dt++ extension annotations.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VqlExtensions {
    pub consume_after: Option<u64>,
    pub session_protocol: Option<VqlSessionProtocol>,
    pub effects: Option<Vec<VqlEffectLabel>>,
    pub transaction_state: Option<VqlTransactionState>,
    pub proof_attached: Option<String>,
    pub usage_limit: Option<u64>,
}

/// A VQL query type in serialized form.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VqlQueryType {
    pub modalities: Vec<VqlModality>,
    pub result_fields: Vec<String>,
    pub extensions: VqlExtensions,
}

/// Convert a VQL query type to a TypeLL base type.
///
/// A VQL query result is modelled as a Named type parameterised by its
/// modalities and result fields.
pub fn vql_to_typell(vql: &VqlQueryType) -> Type {
    Type::Named {
        name: "QueryResult".to_string(),
        args: vql
            .modalities
            .iter()
            .map(|m| Type::Named {
                name: format!("Modality:{}", modality_name(m)),
                args: vec![],
            })
            .collect(),
    }
}

/// Convert a VQL query type with extensions to a full TypeLL unified type.
pub fn vql_to_unified(vql: &VqlQueryType) -> UnifiedType {
    let base = vql_to_typell(vql);

    // Usage from CONSUME AFTER or USAGE LIMIT
    let usage = if let Some(n) = vql.extensions.consume_after {
        UsageQuantifier::Bounded(n)
    } else if let Some(n) = vql.extensions.usage_limit {
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
    let effects = vql
        .extensions
        .effects
        .as_ref()
        .map(|labels| labels.iter().map(map_vql_effect).collect())
        .unwrap_or_default();

    // Proof as refinement
    let refinements = vql
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

/// Convert a VQL session protocol to a TypeLL session type.
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

/// Map a VQL effect label to a TypeLL effect.
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

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_query_type() {
        let vql = VqlQueryType {
            modalities: vec![VqlModality::Graph, VqlModality::Vector],
            result_fields: vec!["name".to_string()],
            extensions: VqlExtensions::default(),
        };
        let ty = vql_to_typell(&vql);
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
        let vql = VqlQueryType {
            modalities: vec![VqlModality::Graph],
            result_fields: vec![],
            extensions: VqlExtensions {
                consume_after: Some(3),
                ..Default::default()
            },
        };
        let unified = vql_to_unified(&vql);
        assert_eq!(unified.usage, UsageQuantifier::Bounded(3));
        assert_eq!(unified.discipline, TypeDiscipline::Linear);
    }

    #[test]
    fn test_effects_clause() {
        let vql = VqlQueryType {
            modalities: vec![VqlModality::Document],
            result_fields: vec![],
            extensions: VqlExtensions {
                effects: Some(vec![VqlEffectLabel::Read, VqlEffectLabel::Write]),
                ..Default::default()
            },
        };
        let unified = vql_to_unified(&vql);
        assert_eq!(unified.effects.len(), 2);
        assert_eq!(unified.effects[0], Effect::IO);
    }

    #[test]
    fn test_proof_attached_creates_refinement() {
        let vql = VqlQueryType {
            modalities: vec![VqlModality::Provenance],
            result_fields: vec![],
            extensions: VqlExtensions {
                proof_attached: Some("integrity_theorem".to_string()),
                ..Default::default()
            },
        };
        let unified = vql_to_unified(&vql);
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
}
