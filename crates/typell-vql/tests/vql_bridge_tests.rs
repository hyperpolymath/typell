// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Comprehensive test suite for the TypeLL-VQL bridge.
//
// Tests cover:
// - All 10 safety levels: names, ordering, subsumption, round-trip
// - All 9 VQL modalities: serialisation/deserialisation
// - VQL extensions: consume_after, session_protocol, effects, transaction_state
// - Bridge mapping: VQL types to correct TypeLL UnifiedTypes
// - Level checking: queries at level N satisfy all levels <= N
// - Safety report generation for various query configurations
// - Typing rules: consume_after, usage_limit, session/effect compatibility,
//   transaction transitions, federation source

use typell_vql::bridge::{
    determine_safety_level, session_protocol_to_session, vql_to_typell, vql_to_unified,
    VqlEffectLabel, VqlExtensions, VqlModality, VqlQueryType, VqlSessionProtocol,
    VqlTransactionState,
};
use typell_vql::levels::{QueryPath, SafetyLevel, ALL_LEVELS};
use typell_vql::rules::{
    check_consume_after, check_federate_requires_source, check_session_effects_compatible,
    check_transaction_transition, check_usage_limit,
};

use typell_core::types::{
    Effect, SessionType, Type, TypeDiscipline, UsageQuantifier,
};

// ============================================================================
// Safety Level — exhaustive per-level tests
// ============================================================================

#[test]
fn test_all_10_levels_have_unique_u8_values() {
    let values: Vec<u8> = ALL_LEVELS.iter().map(|l| l.as_u8()).collect();
    for i in 0..values.len() {
        for j in (i + 1)..values.len() {
            assert_ne!(values[i], values[j], "levels {} and {} share value {}", i, j, values[i]);
        }
    }
}

#[test]
fn test_level_values_are_1_through_10() {
    for (idx, level) in ALL_LEVELS.iter().enumerate() {
        assert_eq!(level.as_u8(), (idx as u8) + 1);
    }
}

#[test]
fn test_each_level_name_is_non_empty() {
    for level in ALL_LEVELS {
        assert!(!level.name().is_empty(), "level {:?} has empty name", level);
    }
}

#[test]
fn test_each_level_typell_concept_is_non_empty() {
    for level in ALL_LEVELS {
        assert!(
            !level.typell_concept().is_empty(),
            "level {:?} has empty typell_concept",
            level
        );
    }
}

#[test]
fn test_level_names_match_expected() {
    assert_eq!(SafetyLevel::ParseTime.name(), "Parse-time safety");
    assert_eq!(SafetyLevel::SchemaBinding.name(), "Schema-binding safety");
    assert_eq!(SafetyLevel::TypeCompatible.name(), "Type-compatible operations");
    assert_eq!(SafetyLevel::NullSafe.name(), "Null-safety");
    assert_eq!(SafetyLevel::InjectionProof.name(), "Injection-proof safety");
    assert_eq!(SafetyLevel::ResultType.name(), "Result-type safety");
    assert_eq!(SafetyLevel::Cardinality.name(), "Cardinality safety");
    assert_eq!(SafetyLevel::EffectTracking.name(), "Effect-tracking safety");
    assert_eq!(SafetyLevel::Temporal.name(), "Temporal safety");
    assert_eq!(SafetyLevel::Linearity.name(), "Linearity safety");
}

#[test]
fn test_level_ordering_is_monotonic() {
    for i in 0..ALL_LEVELS.len() - 1 {
        assert!(
            ALL_LEVELS[i] < ALL_LEVELS[i + 1],
            "ordering violation at index {}: {:?} >= {:?}",
            i,
            ALL_LEVELS[i],
            ALL_LEVELS[i + 1]
        );
    }
}

#[test]
fn test_from_u8_round_trip_all_levels() {
    for level in ALL_LEVELS {
        let n = level.as_u8();
        let recovered = SafetyLevel::from_u8(n);
        assert_eq!(recovered, Some(level));
    }
}

#[test]
fn test_from_u8_out_of_range_returns_none() {
    assert_eq!(SafetyLevel::from_u8(0), None);
    assert_eq!(SafetyLevel::from_u8(11), None);
    assert_eq!(SafetyLevel::from_u8(255), None);
}

#[test]
fn test_established_levels_are_1_through_6() {
    for level in ALL_LEVELS {
        if level.as_u8() <= 6 {
            assert!(level.is_established(), "{:?} should be established", level);
        } else {
            assert!(!level.is_established(), "{:?} should NOT be established", level);
        }
    }
}

#[test]
fn test_satisfied_levels_subsumption_for_each_level() {
    for level in ALL_LEVELS {
        let satisfied = level.satisfied_levels();
        let n = level.as_u8() as usize;
        assert_eq!(
            satisfied.len(),
            n,
            "level {:?} should satisfy {} levels, got {}",
            level,
            n,
            satisfied.len()
        );
        // Each satisfied level should be <= the target level.
        for s in &satisfied {
            assert!(*s <= level, "{:?} should be <= {:?}", s, level);
        }
        // The last satisfied level should be the level itself.
        assert_eq!(*satisfied.last().unwrap(), level);
    }
}

#[test]
fn test_level_10_satisfies_all_levels() {
    let satisfied = SafetyLevel::Linearity.satisfied_levels();
    assert_eq!(satisfied.len(), 10);
    assert_eq!(satisfied, ALL_LEVELS.to_vec());
}

#[test]
fn test_level_1_satisfies_only_itself() {
    let satisfied = SafetyLevel::ParseTime.satisfied_levels();
    assert_eq!(satisfied.len(), 1);
    assert_eq!(satisfied[0], SafetyLevel::ParseTime);
}

// ============================================================================
// QueryPath tests
// ============================================================================

#[test]
fn test_query_path_names() {
    assert_eq!(QueryPath::Slipstream.name(), "VQL (Slipstream)");
    assert_eq!(QueryPath::Dt.name(), "VQL-DT");
    assert_eq!(QueryPath::Ut.name(), "VQL-UT");
}

#[test]
fn test_query_path_max_achievable_ordering() {
    assert!(QueryPath::Slipstream.max_achievable() < QueryPath::Dt.max_achievable());
    assert!(QueryPath::Dt.max_achievable() < QueryPath::Ut.max_achievable());
}

// ============================================================================
// VQL Modalities — serialisation round-trips (all 9)
// ============================================================================

#[test]
fn test_all_9_modalities_serialise_round_trip() {
    let modalities = vec![
        VqlModality::Graph,
        VqlModality::Vector,
        VqlModality::Tensor,
        VqlModality::Semantic,
        VqlModality::Document,
        VqlModality::Temporal,
        VqlModality::Provenance,
        VqlModality::Spatial,
        VqlModality::All,
    ];
    for m in &modalities {
        let json = serde_json::to_string(m).expect("serialise modality");
        let recovered: VqlModality =
            serde_json::from_str(&json).expect("deserialise modality");
        // Verify round-trip produces a valid variant (matching Debug repr).
        assert_eq!(format!("{:?}", m), format!("{:?}", recovered));
    }
    assert_eq!(modalities.len(), 9);
}

#[test]
fn test_each_modality_produces_named_type_arg() {
    let modalities = vec![
        VqlModality::Graph,
        VqlModality::Vector,
        VqlModality::Tensor,
        VqlModality::Semantic,
        VqlModality::Document,
        VqlModality::Temporal,
        VqlModality::Provenance,
        VqlModality::Spatial,
        VqlModality::All,
    ];
    for m in modalities {
        let vql = VqlQueryType {
            modalities: vec![m],
            result_fields: vec![],
            extensions: VqlExtensions::default(),
        };
        let ty = vql_to_typell(&vql);
        match &ty {
            Type::Named { name, args } => {
                assert_eq!(name, "QueryResult");
                assert_eq!(args.len(), 1);
                // Each modality arg should be a Named type starting with "Modality:".
                match &args[0] {
                    Type::Named { name: mod_name, .. } => {
                        assert!(
                            mod_name.starts_with("Modality:"),
                            "expected Modality: prefix, got {}",
                            mod_name
                        );
                    }
                    other => panic!("expected Named type for modality, got {:?}", other),
                }
            }
            other => panic!("expected Named QueryResult, got {:?}", other),
        }
    }
}

// ============================================================================
// VQL Extensions — serialisation round-trips
// ============================================================================

#[test]
fn test_extensions_default_is_all_none() {
    let ext = VqlExtensions::default();
    assert!(ext.consume_after.is_none());
    assert!(ext.session_protocol.is_none());
    assert!(ext.effects.is_none());
    assert!(ext.transaction_state.is_none());
    assert!(ext.proof_attached.is_none());
    assert!(ext.usage_limit.is_none());
}

#[test]
fn test_extensions_serialise_round_trip() {
    let ext = VqlExtensions {
        consume_after: Some(5),
        session_protocol: Some(VqlSessionProtocol::Batch),
        effects: Some(vec![VqlEffectLabel::Read, VqlEffectLabel::Cite]),
        transaction_state: Some(VqlTransactionState::Active),
        proof_attached: Some("theorem_1".to_string()),
        usage_limit: Some(10),
    };
    let json = serde_json::to_string(&ext).expect("serialise extensions");
    let recovered: VqlExtensions =
        serde_json::from_str(&json).expect("deserialise extensions");
    assert_eq!(recovered.consume_after, Some(5));
    assert_eq!(recovered.usage_limit, Some(10));
    assert!(recovered.proof_attached.as_deref() == Some("theorem_1"));
}

#[test]
fn test_session_protocol_serialise_round_trip() {
    let protocols = vec![
        VqlSessionProtocol::ReadOnly,
        VqlSessionProtocol::Mutation,
        VqlSessionProtocol::Stream,
        VqlSessionProtocol::Batch,
        VqlSessionProtocol::Custom("my_proto".to_string()),
    ];
    for p in &protocols {
        let json = serde_json::to_string(p).expect("serialise protocol");
        let recovered: VqlSessionProtocol =
            serde_json::from_str(&json).expect("deserialise protocol");
        assert_eq!(format!("{:?}", p), format!("{:?}", recovered));
    }
}

#[test]
fn test_effect_labels_serialise_round_trip() {
    let labels = vec![
        VqlEffectLabel::Read,
        VqlEffectLabel::Write,
        VqlEffectLabel::Cite,
        VqlEffectLabel::Audit,
        VqlEffectLabel::Transform,
        VqlEffectLabel::Federate,
        VqlEffectLabel::Custom("myEffect".to_string()),
    ];
    for label in &labels {
        let json = serde_json::to_string(label).expect("serialise label");
        let recovered: VqlEffectLabel =
            serde_json::from_str(&json).expect("deserialise label");
        assert_eq!(format!("{:?}", label), format!("{:?}", recovered));
    }
}

#[test]
fn test_transaction_state_serialise_round_trip() {
    let states = vec![
        VqlTransactionState::Fresh,
        VqlTransactionState::Active,
        VqlTransactionState::Committed,
        VqlTransactionState::RolledBack,
        VqlTransactionState::ReadSnapshot,
        VqlTransactionState::Custom("special".to_string()),
    ];
    for s in &states {
        let json = serde_json::to_string(s).expect("serialise state");
        let recovered: VqlTransactionState =
            serde_json::from_str(&json).expect("deserialise state");
        assert_eq!(format!("{:?}", s), format!("{:?}", recovered));
    }
}

// ============================================================================
// Bridge mapping — VQL types to TypeLL UnifiedTypes
// ============================================================================

#[test]
fn test_vql_to_typell_empty_modalities_gives_empty_args() {
    let vql = VqlQueryType {
        modalities: vec![],
        result_fields: vec![],
        extensions: VqlExtensions::default(),
    };
    let ty = vql_to_typell(&vql);
    match ty {
        Type::Named { name, args } => {
            assert_eq!(name, "QueryResult");
            assert_eq!(args.len(), 0);
        }
        other => panic!("expected Named QueryResult, got {:?}", other),
    }
}

#[test]
fn test_vql_to_typell_multiple_modalities() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph, VqlModality::Tensor, VqlModality::Spatial],
        result_fields: vec![],
        extensions: VqlExtensions::default(),
    };
    let ty = vql_to_typell(&vql);
    match ty {
        Type::Named { args, .. } => assert_eq!(args.len(), 3),
        other => panic!("expected Named type, got {:?}", other),
    }
}

#[test]
fn test_vql_to_unified_no_extensions_gives_omega_unrestricted() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec![],
        extensions: VqlExtensions::default(),
    };
    let unified = vql_to_unified(&vql);
    assert_eq!(unified.usage, UsageQuantifier::Omega);
    assert_eq!(unified.discipline, TypeDiscipline::Unrestricted);
    assert!(unified.effects.is_empty());
    assert!(unified.refinements.is_empty());
}

#[test]
fn test_vql_to_unified_consume_after_gives_linear() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Vector],
        result_fields: vec![],
        extensions: VqlExtensions {
            consume_after: Some(7),
            ..Default::default()
        },
    };
    let unified = vql_to_unified(&vql);
    assert_eq!(unified.usage, UsageQuantifier::Bounded(7));
    assert_eq!(unified.discipline, TypeDiscipline::Linear);
}

#[test]
fn test_vql_to_unified_usage_limit_gives_linear() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Document],
        result_fields: vec![],
        extensions: VqlExtensions {
            usage_limit: Some(3),
            ..Default::default()
        },
    };
    let unified = vql_to_unified(&vql);
    assert_eq!(unified.usage, UsageQuantifier::Bounded(3));
    assert_eq!(unified.discipline, TypeDiscipline::Linear);
}

#[test]
fn test_vql_to_unified_consume_after_takes_priority_over_usage_limit() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec![],
        extensions: VqlExtensions {
            consume_after: Some(2),
            usage_limit: Some(100),
            ..Default::default()
        },
    };
    let unified = vql_to_unified(&vql);
    assert_eq!(unified.usage, UsageQuantifier::Bounded(2));
}

#[test]
fn test_vql_to_unified_all_effect_labels_map_correctly() {
    let labels = vec![
        VqlEffectLabel::Read,
        VqlEffectLabel::Write,
        VqlEffectLabel::Cite,
        VqlEffectLabel::Audit,
        VqlEffectLabel::Transform,
        VqlEffectLabel::Federate,
        VqlEffectLabel::Custom("CustomEffect".to_string()),
    ];
    let vql = VqlQueryType {
        modalities: vec![VqlModality::All],
        result_fields: vec![],
        extensions: VqlExtensions {
            effects: Some(labels),
            ..Default::default()
        },
    };
    let unified = vql_to_unified(&vql);
    assert_eq!(unified.effects.len(), 7);
    // Verify specific mappings.
    assert_eq!(unified.effects[0], Effect::IO); // Read -> IO
    assert_eq!(unified.effects[1], Effect::State("write".to_string())); // Write -> State
    assert_eq!(unified.effects[5], Effect::Network); // Federate -> Network
}

#[test]
fn test_vql_to_unified_proof_attached_creates_one_refinement() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Provenance],
        result_fields: vec![],
        extensions: VqlExtensions {
            proof_attached: Some("my_theorem".to_string()),
            ..Default::default()
        },
    };
    let unified = vql_to_unified(&vql);
    assert_eq!(unified.refinements.len(), 1);
}

// ============================================================================
// Session protocol to TypeLL session type
// ============================================================================

#[test]
fn test_session_protocol_readonly_is_recv() {
    let session = session_protocol_to_session(&VqlSessionProtocol::ReadOnly);
    assert!(matches!(session, SessionType::Recv(_, _)));
}

#[test]
fn test_session_protocol_mutation_is_send_then_recv() {
    let session = session_protocol_to_session(&VqlSessionProtocol::Mutation);
    match session {
        SessionType::Send(_, cont) => {
            assert!(matches!(*cont, SessionType::Recv(_, _)));
        }
        other => panic!("expected Send, got {:?}", other),
    }
}

#[test]
fn test_session_protocol_stream_is_recursive() {
    let session = session_protocol_to_session(&VqlSessionProtocol::Stream);
    assert!(matches!(session, SessionType::Rec(_, _)));
}

#[test]
fn test_session_protocol_batch_is_send_then_recv() {
    let session = session_protocol_to_session(&VqlSessionProtocol::Batch);
    match session {
        SessionType::Send(_, cont) => {
            assert!(matches!(*cont, SessionType::Recv(_, _)));
        }
        other => panic!("expected Send for Batch, got {:?}", other),
    }
}

#[test]
fn test_session_protocol_custom() {
    let session = session_protocol_to_session(&VqlSessionProtocol::Custom("foo".to_string()));
    match session {
        SessionType::Send(ty, cont) => {
            match *ty {
                Type::Named { ref name, .. } => assert!(name.contains("Custom:foo")),
                other => panic!("expected Named type, got {:?}", other),
            }
            assert!(matches!(*cont, SessionType::End));
        }
        other => panic!("expected Send for Custom, got {:?}", other),
    }
}

// ============================================================================
// Safety level determination — incremental coverage
// ============================================================================

#[test]
fn test_safety_empty_query_is_level_1_slipstream() {
    let vql = VqlQueryType {
        modalities: vec![],
        result_fields: vec![],
        extensions: VqlExtensions::default(),
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.max_level, SafetyLevel::ParseTime);
    assert_eq!(report.query_path, QueryPath::Slipstream);
}

#[test]
fn test_safety_schema_bound_no_modalities_is_level_2() {
    let vql = VqlQueryType {
        modalities: vec![],
        result_fields: vec!["id".to_string()],
        extensions: VqlExtensions::default(),
    };
    let report = determine_safety_level(&vql);
    // Level 3 fails (no modalities), so max_level = SchemaBinding.
    assert_eq!(report.max_level, SafetyLevel::SchemaBinding);
}

#[test]
fn test_safety_level_4_null_safe() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec!["name".to_string()],
        extensions: VqlExtensions::default(),
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.max_level, SafetyLevel::NullSafe);
    assert_eq!(report.query_path, QueryPath::Dt);
}

#[test]
fn test_safety_level_5_injection_proof() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec!["name".to_string()],
        extensions: VqlExtensions {
            proof_attached: Some("safe".to_string()),
            ..Default::default()
        },
    };
    let report = determine_safety_level(&vql);
    // Level 5 passes, level 6 also passes (schema-bound), but level 7
    // requires usage_limit, so max = ResultType (6).
    assert_eq!(report.max_level, SafetyLevel::ResultType);
}

#[test]
fn test_safety_level_8_effect_tracking() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Document],
        result_fields: vec!["content".to_string()],
        extensions: VqlExtensions {
            proof_attached: Some("safe".to_string()),
            usage_limit: Some(5),
            effects: Some(vec![VqlEffectLabel::Read]),
            ..Default::default()
        },
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.max_level, SafetyLevel::EffectTracking);
    assert_eq!(report.query_path, QueryPath::Ut);
}

#[test]
fn test_safety_level_9_temporal_with_session() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec!["x".to_string()],
        extensions: VqlExtensions {
            proof_attached: Some("thm".to_string()),
            usage_limit: Some(5),
            effects: Some(vec![VqlEffectLabel::Read]),
            session_protocol: Some(VqlSessionProtocol::ReadOnly),
            ..Default::default()
        },
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.max_level, SafetyLevel::Temporal);
}

#[test]
fn test_safety_level_9_temporal_with_transaction_state() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec!["x".to_string()],
        extensions: VqlExtensions {
            proof_attached: Some("thm".to_string()),
            usage_limit: Some(5),
            effects: Some(vec![VqlEffectLabel::Read]),
            transaction_state: Some(VqlTransactionState::Active),
            ..Default::default()
        },
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.max_level, SafetyLevel::Temporal);
}

#[test]
fn test_safety_level_10_full_linearity() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec!["x".to_string()],
        extensions: VqlExtensions {
            consume_after: Some(3),
            session_protocol: Some(VqlSessionProtocol::ReadOnly),
            effects: Some(vec![VqlEffectLabel::Read]),
            proof_attached: Some("safe".to_string()),
            usage_limit: Some(10),
            transaction_state: None,
        },
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.max_level, SafetyLevel::Linearity);
    assert_eq!(report.query_path, QueryPath::Ut);
    assert_eq!(report.checks.len(), 10);
    assert!(report.checks.iter().all(|c| c.passed));
}

#[test]
fn test_safety_report_has_exactly_10_checks() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec![],
        extensions: VqlExtensions::default(),
    };
    let report = determine_safety_level(&vql);
    assert_eq!(report.checks.len(), 10);
}

#[test]
fn test_safety_report_failed_checks_have_diagnostics() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec![],
        extensions: VqlExtensions::default(),
    };
    let report = determine_safety_level(&vql);
    // Level 2 (SchemaBinding) should fail because result_fields is empty.
    let l2 = &report.checks[1];
    assert!(!l2.passed);
    assert!(!l2.diagnostic.is_empty());
}

#[test]
fn test_safety_report_serialise_round_trip() {
    let vql = VqlQueryType {
        modalities: vec![VqlModality::Graph],
        result_fields: vec!["x".to_string()],
        extensions: VqlExtensions::default(),
    };
    let report = determine_safety_level(&vql);
    let json = serde_json::to_string(&report).expect("serialise report");
    let recovered: typell_vql::levels::SafetyReport =
        serde_json::from_str(&json).expect("deserialise report");
    assert_eq!(recovered.max_level, report.max_level);
    assert_eq!(recovered.checks.len(), 10);
}

// ============================================================================
// Typing rules — additional coverage
// ============================================================================

#[test]
fn test_consume_after_boundary_values() {
    assert!(check_consume_after(1).is_ok());
    assert!(check_consume_after(u64::MAX).is_ok());
    assert!(check_consume_after(0).is_err());
}

#[test]
fn test_usage_limit_valid() {
    assert!(check_usage_limit(1).is_ok());
    assert!(check_usage_limit(100).is_ok());
}

#[test]
fn test_usage_limit_zero_invalid() {
    assert!(check_usage_limit(0).is_err());
}

#[test]
fn test_stream_session_allows_any_effects() {
    let effects = vec![VqlEffectLabel::Read, VqlEffectLabel::Write];
    assert!(check_session_effects_compatible(&VqlSessionProtocol::Stream, &effects).is_ok());
}

#[test]
fn test_batch_session_allows_any_effects() {
    let effects = vec![VqlEffectLabel::Write, VqlEffectLabel::Federate];
    assert!(check_session_effects_compatible(&VqlSessionProtocol::Batch, &effects).is_ok());
}

#[test]
fn test_custom_session_allows_any_effects() {
    let effects = vec![VqlEffectLabel::Write];
    assert!(check_session_effects_compatible(
        &VqlSessionProtocol::Custom("x".to_string()),
        &effects
    )
    .is_ok());
}

#[test]
fn test_transaction_active_to_rolled_back() {
    assert!(check_transaction_transition(
        &VqlTransactionState::Active,
        &VqlTransactionState::RolledBack
    )
    .is_ok());
}

#[test]
fn test_transaction_committed_to_active_invalid() {
    assert!(check_transaction_transition(
        &VqlTransactionState::Committed,
        &VqlTransactionState::Active
    )
    .is_err());
}

#[test]
fn test_transaction_rolled_back_to_committed_invalid() {
    assert!(check_transaction_transition(
        &VqlTransactionState::RolledBack,
        &VqlTransactionState::Committed
    )
    .is_err());
}

#[test]
fn test_transaction_fresh_to_committed_invalid() {
    assert!(check_transaction_transition(
        &VqlTransactionState::Fresh,
        &VqlTransactionState::Committed
    )
    .is_err());
}

#[test]
fn test_federate_without_source_ok_when_no_federate_effect() {
    let effects = vec![VqlEffectLabel::Read, VqlEffectLabel::Write];
    assert!(check_federate_requires_source(&effects, false).is_ok());
}

#[test]
fn test_federate_with_source_ok() {
    let effects = vec![VqlEffectLabel::Federate];
    assert!(check_federate_requires_source(&effects, true).is_ok());
}

#[test]
fn test_empty_effects_no_federate_source_ok() {
    let effects: Vec<VqlEffectLabel> = vec![];
    assert!(check_federate_requires_source(&effects, false).is_ok());
}
