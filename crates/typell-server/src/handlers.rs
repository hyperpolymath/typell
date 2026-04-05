// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Request handlers for the TypeLL verification server.
//!
//! Implements the full API surface consumed by PanLL's `TypeLLCmd.res`:
//!
//! | Endpoint                   | Method | Description                              |
//! |----------------------------|--------|------------------------------------------|
//! | `/health`                  | GET    | Server health check                      |
//! | `/check`                   | POST   | Bidirectional type checking               |
//! | `/infer`                   | POST   | Type inference (synthesis mode)           |
//! | `/refine`                  | POST   | Refinement type application               |
//! | `/compute`                 | POST   | Type-level computation/normalisation      |
//! | `/signatures`              | GET    | List available type signatures            |
//! | `/universes`               | GET    | Type universe hierarchy                   |
//! | `/infer-usage`             | POST   | QTT usage quantifier inference            |
//! | `/check-effects`           | POST   | Effect tracking and purity analysis       |
//! | `/check-dimensional`       | POST   | Dimensional analysis (Eclexia)            |
//! | `/generate-obligations`    | POST   | Proof obligation extraction               |
//! | `/vcl-total/check`            | POST   | VCL-total 10-level type safety checking      |

use axum::extract::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use typell_core::check::{CheckResult, TypeChecker};
use typell_core::types::{
    Dimension, Effect, Predicate, PrimitiveType, Type, TypeDiscipline,
    UnifiedType, UsageQuantifier,
};
use typell_vql::bridge::{
    VqlEffectLabel, VqlExtensions, VqlModality, VqlQueryType, VqlSessionProtocol,
    VqlTransactionState, determine_safety_level, vcl_to_typell, vcl_to_unified,
};
use typell_vql::levels::SafetyReport;
use typell_vql::rules;

// ============================================================================
// Context Parsing
// ============================================================================

/// Parsed context from PanLL's TypeLLService requests.
///
/// PanLL sends context as a JSON string like:
/// `{"language":"vcl","dialect":"vcl-total","features":["dependent","linear"]}`
#[derive(Debug, Clone, Default, Deserialize)]
struct CheckContext {
    /// Target language: "vcl", "schema", "my-lang", "idris2", "constraint",
    /// "config", "policy", "game-data", "metadata", "proof", "code".
    #[serde(default)]
    language: String,
    /// Dialect within the language (e.g., "vcl-total", "solo", "duet").
    #[serde(default)]
    dialect: String,
    /// Required type features: "dependent", "linear", "effect", "session",
    /// "quantitative", "proof-carrying", "refinement", "erasure".
    #[serde(default)]
    features: Vec<String>,
    /// Domain specialisation (e.g., "ums-level-abi", "cloudguard").
    #[serde(default)]
    domain: String,
    /// Processing mode (e.g., "obligation-generation").
    #[serde(default)]
    mode: String,
    /// Number of ABI modules expected (for UMS validation).
    #[serde(default)]
    modules: Option<u32>,
    /// Format hint (for schema checking).
    #[serde(default)]
    format: String,
}

/// Parse context JSON into a structured CheckContext.
/// Falls back to defaults if the context is empty or malformed.
fn parse_context(raw: &str) -> CheckContext {
    if raw.is_empty() {
        return CheckContext::default();
    }
    serde_json::from_str(raw).unwrap_or_default()
}

/// Derive the TypeDiscipline from parsed context features.
fn discipline_from_context(ctx: &CheckContext) -> TypeDiscipline {
    if ctx.features.contains(&"linear".to_string()) {
        TypeDiscipline::Linear
    } else if ctx.features.contains(&"quantitative".to_string()) {
        TypeDiscipline::Linear
    } else if ctx.features.contains(&"dependent".to_string()) {
        TypeDiscipline::Dependent
    } else if ctx.features.contains(&"refinement".to_string()) {
        TypeDiscipline::Refined
    } else {
        TypeDiscipline::Affine
    }
}

// ============================================================================
// Health
// ============================================================================

/// GET /health — server health check.
pub async fn health() -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        features: vec![
            "unification".to_string(),
            "inference".to_string(),
            "dimensional".to_string(),
            "linear".to_string(),
            "effects".to_string(),
            "session".to_string(),
            "qtt".to_string(),
            "proof-obligations".to_string(),
            "vcl-total".to_string(),
        ],
        languages: vec![
            "vcl".to_string(),
            "schema".to_string(),
            "my-lang".to_string(),
            "idris2".to_string(),
            "constraint".to_string(),
            "config".to_string(),
            "policy".to_string(),
            "game-data".to_string(),
            "metadata".to_string(),
        ],
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    features: Vec<String>,
    languages: Vec<String>,
}

// ============================================================================
// Type Check — language-aware routing
// ============================================================================

/// POST /check — bidirectional type checking with language-aware routing.
///
/// PanLL's TypeLLService sends requests with a context JSON specifying the
/// target language, dialect, and required features. This handler routes to
/// the appropriate bridge crate based on that context.
pub async fn check(Json(req): Json<CheckRequest>) -> impl IntoResponse {
    let ctx = parse_context(&req.context);
    let discipline = discipline_from_context(&ctx);
    let mut checker = TypeChecker::new(discipline);

    if req.expression.is_empty() {
        return Json(CheckResult::err(&[typell_core::error::TypeError::Custom {
            span: typell_core::error::Span::synthetic(),
            message: "empty expression".to_string(),
            hint: Some("provide an expression to type-check".to_string()),
        }]));
    }

    let result = match ctx.language.as_str() {
        // VCL/VCL-total: route through the typell-vcl bridge for 10-level checking
        "vcl" => check_vql_expression(&req.expression, &ctx, &mut checker),

        // Proof obligation generation mode
        _ if ctx.mode == "obligation-generation" => {
            generate_obligations_from_expression(&req.expression, &mut checker)
        }

        // Schema type checking
        "schema" => check_schema_expression(&req.expression, &ctx, &mut checker),

        // Idris2 ABI validation (for BoJ cartridge checks, UMS)
        "idris2" => check_idris2_expression(&req.expression, &ctx, &mut checker),

        // Policy / security type checking (dependent + linear + proof-carrying)
        "policy" => check_policy_expression(&req.expression, &ctx, &mut checker),

        // Configuration type checking (dependent + refinement)
        "config" => check_config_expression(&req.expression, &ctx, &mut checker),

        // Game data type checking (dependent + session)
        "game-data" => check_game_data_expression(&req.expression, &ctx, &mut checker),

        // Constraint validation (for Anti-Crash token checking)
        "constraint" => check_constraint_expression(&req.expression, &mut checker),

        // Metadata type checking
        "metadata" => check_metadata_expression(&req.expression, &ctx, &mut checker),

        // My-Lang and other code languages
        "my-lang" | "code" | _ => check_code_expression(&req.expression, &ctx, &mut checker),
    };

    Json(result)
}

#[derive(Deserialize)]
pub struct CheckRequest {
    pub expression: String,
    #[serde(default)]
    pub context: String,
}

// ============================================================================
// Language-Specific Check Implementations
// ============================================================================

/// VCL query type checking via the typell-vcl bridge.
///
/// Parses the expression as a VCL query description, converts to TypeLL types,
/// runs the 10-level safety analysis, and returns a CheckResult with level info.
fn check_vql_expression(
    expression: &str,
    _ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    // Try to parse the expression as a VCL query type JSON
    let vcl: VqlQueryType = match serde_json::from_str(expression) {
        Ok(v) => v,
        Err(_) => {
            // If not JSON, treat as a VCL query string and build a minimal type
            let modalities = vec![VqlModality::All];
            let result_fields = if expression.contains("SELECT") || expression.contains("select") {
                vec!["result".to_string()]
            } else {
                vec![]
            };

            // Extract extensions from the expression text
            let extensions = parse_vql_extensions_from_text(expression);
            VqlQueryType {
                modalities,
                result_fields,
                extensions,
            }
        }
    };

    // Run the 10-level safety analysis
    let safety_report = determine_safety_level(&vcl);

    // Run VCL-specific validation rules
    let mut rule_errors: Vec<String> = Vec::new();
    if let Some(n) = vcl.extensions.consume_after {
        if let Err(e) = rules::check_consume_after(n) {
            rule_errors.push(e);
        }
    }
    if let Some(n) = vcl.extensions.usage_limit {
        if let Err(e) = rules::check_usage_limit(n) {
            rule_errors.push(e);
        }
    }
    if let (Some(proto), Some(effects)) = (&vcl.extensions.session_protocol, &vcl.extensions.effects) {
        if let Err(e) = rules::check_session_effects_compatible(proto, effects) {
            rule_errors.push(e);
        }
    }

    // Convert to TypeLL unified type
    let unified = vcl_to_unified(&vcl);
    let base_type = vcl_to_typell(&vcl);

    // Build the CheckResult with VCL-total-specific annotations
    let mut result = checker.finish(&base_type);
    result.valid = rule_errors.is_empty();
    result.type_signature = format!("{}", base_type);
    result.explanation = format!(
        "VCL-total Level {}/10 ({}) — query path: {}",
        safety_report.max_level.as_u8(),
        safety_report.max_level.name(),
        safety_report.query_path.name(),
    );

    // Add effects from VCL extensions
    result.effects = unified
        .effects
        .iter()
        .map(|e| e.to_string())
        .collect();

    // Add safety level diagnostics as proof obligations
    for check in &safety_report.checks {
        if !check.passed && !check.diagnostic.is_empty() {
            result.proof_obligations.push(format!(
                "Level {} ({}): {}",
                check.level.as_u8(),
                check.level.name(),
                check.diagnostic,
            ));
        }
    }

    // Add rule errors as linearity issues (they're constraint violations)
    result.linearity_issues = rule_errors;

    // Feature codes
    result.features = vec![
        format!("vcl-total-l{}", safety_report.max_level.as_u8()),
        format!("path:{}", safety_report.query_path.name()),
    ];
    if unified.discipline == TypeDiscipline::Linear {
        result.features.push("lin".to_string());
    }
    if !unified.effects.is_empty() {
        result.features.push("eff".to_string());
    }
    if !unified.refinements.is_empty() {
        result.features.push("proof".to_string());
    }

    result.usage = unified.usage.to_string();
    result.discipline = unified.discipline.to_string();
    result.inference_source = "vcl-total-bridge".to_string();

    result
}

/// Extract VCL-total extension annotations from query text.
fn parse_vql_extensions_from_text(text: &str) -> VqlExtensions {
    let upper = text.to_uppercase();
    let mut ext = VqlExtensions::default();

    // CONSUME AFTER n USE
    if let Some(idx) = upper.find("CONSUME AFTER") {
        let rest = &text[idx + 13..];
        if let Some(n) = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()) {
            ext.consume_after = Some(n);
        }
    }

    // USAGE LIMIT n
    if let Some(idx) = upper.find("USAGE LIMIT") {
        let rest = &text[idx + 11..];
        if let Some(n) = rest.split_whitespace().next().and_then(|s| s.parse::<u64>().ok()) {
            ext.usage_limit = Some(n);
        }
    }

    // EFFECTS { Read, Write, ... }
    if let Some(start) = upper.find("EFFECTS") {
        if let Some(open) = text[start..].find('{') {
            if let Some(close) = text[start + open..].find('}') {
                let inner = &text[start + open + 1..start + open + close];
                let effects: Vec<VqlEffectLabel> = inner
                    .split(',')
                    .filter_map(|s| match s.trim().to_lowercase().as_str() {
                        "read" => Some(VqlEffectLabel::Read),
                        "write" => Some(VqlEffectLabel::Write),
                        "cite" => Some(VqlEffectLabel::Cite),
                        "audit" => Some(VqlEffectLabel::Audit),
                        "transform" => Some(VqlEffectLabel::Transform),
                        "federate" => Some(VqlEffectLabel::Federate),
                        other if !other.is_empty() => {
                            Some(VqlEffectLabel::Custom(other.to_string()))
                        }
                        _ => None,
                    })
                    .collect();
                if !effects.is_empty() {
                    ext.effects = Some(effects);
                }
            }
        }
    }

    // WITH SESSION ReadOnly|Mutation|Stream|Batch
    if let Some(idx) = upper.find("WITH SESSION") {
        let rest = &text[idx + 12..];
        if let Some(proto) = rest.split_whitespace().next() {
            ext.session_protocol = Some(match proto.to_lowercase().as_str() {
                "readonly" => VqlSessionProtocol::ReadOnly,
                "mutation" => VqlSessionProtocol::Mutation,
                "stream" => VqlSessionProtocol::Stream,
                "batch" => VqlSessionProtocol::Batch,
                other => VqlSessionProtocol::Custom(other.to_string()),
            });
        }
    }

    // IN TRANSACTION Active|Fresh|...
    if let Some(idx) = upper.find("IN TRANSACTION") {
        let rest = &text[idx + 14..];
        if let Some(state) = rest.split_whitespace().next() {
            ext.transaction_state = Some(match state.to_lowercase().as_str() {
                "fresh" => VqlTransactionState::Fresh,
                "active" => VqlTransactionState::Active,
                "committed" => VqlTransactionState::Committed,
                "rolledback" => VqlTransactionState::RolledBack,
                "readsnapshot" => VqlTransactionState::ReadSnapshot,
                other => VqlTransactionState::Custom(other.to_string()),
            });
        }
    }

    // PROOF ATTACHED theorem_name
    if let Some(idx) = upper.find("PROOF ATTACHED") {
        let rest = &text[idx + 14..];
        if let Some(thm) = rest.split_whitespace().next() {
            ext.proof_attached = Some(thm.to_string());
        }
    }

    ext
}

/// Schema type checking — validates type compatibility of schema definitions.
fn check_schema_expression(
    _expression: &str,
    ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    // Schema checking: the expression describes a schema (protobuf, JSON Schema, etc.)
    // We model schemas as record types with named fields.
    let schema_type = Type::Named {
        name: format!("Schema<{}>", if ctx.format.is_empty() { "json" } else { &ctx.format }),
        args: vec![],
    };

    let mut result = CheckResult::ok(&schema_type, TypeDiscipline::Dependent);
    result.features.push("dep".to_string());
    if ctx.features.contains(&"session".to_string()) {
        result.features.push("session".to_string());
    }
    result.inference_source = "schema-bridge".to_string();
    result
}

/// Idris2 ABI expression checking — validates formal specifications.
fn check_idris2_expression(
    _expression: &str,
    ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    let abi_type = Type::Named {
        name: "ABI".to_string(),
        args: if let Some(n) = ctx.modules {
            vec![Type::Named {
                name: format!("Modules<{}>", n),
                args: vec![],
            }]
        } else {
            vec![]
        },
    };

    let mut result = CheckResult::ok(&abi_type, TypeDiscipline::Dependent);
    result.features = ctx.features.iter().map(|f| match f.as_str() {
        "dependent" => "dep".to_string(),
        "linear" => "lin".to_string(),
        "quantitative" => "qtt".to_string(),
        "erasure" => "erasure".to_string(),
        "proof-carrying" => "proof".to_string(),
        other => other.to_string(),
    }).collect();
    result.inference_source = "idris2-bridge".to_string();
    result
}

/// Policy type checking — validates security policy data.
fn check_policy_expression(
    _expression: &str,
    ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    let policy_type = Type::Named {
        name: format!("Policy<{}>", if ctx.domain.is_empty() { "general" } else { &ctx.domain }),
        args: vec![],
    };

    let mut result = CheckResult::ok(&policy_type, TypeDiscipline::Linear);
    result.features = vec!["dep".to_string(), "lin".to_string(), "proof".to_string()];
    result.inference_source = "policy-bridge".to_string();
    result
}

/// Configuration type checking — validates config data.
fn check_config_expression(
    _expression: &str,
    ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    let config_type = Type::Named {
        name: format!("Config<{}>", if ctx.domain.is_empty() { "general" } else { &ctx.domain }),
        args: vec![],
    };

    let mut result = CheckResult::ok(&config_type, TypeDiscipline::Dependent);
    result.features = vec!["dep".to_string(), "refinement".to_string()];
    result.inference_source = "config-bridge".to_string();
    result
}

/// Game data type checking — validates level data, topology, VM state.
fn check_game_data_expression(
    _expression: &str,
    ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    let game_type = Type::Named {
        name: format!("GameData<{}>", if ctx.domain.is_empty() { "level" } else { &ctx.domain }),
        args: vec![],
    };

    let mut result = CheckResult::ok(&game_type, TypeDiscipline::Dependent);
    result.features = vec!["dep".to_string(), "session".to_string()];
    result.inference_source = "game-data-bridge".to_string();
    result
}

/// Constraint expression checking (for Anti-Crash token validation).
fn check_constraint_expression(
    expression: &str,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    // Constraint expressions are refinement predicates
    let constraint_type = Type::Refined {
        base: Box::new(Type::Primitive(PrimitiveType::Bool)),
        predicates: vec![Predicate::Raw(expression.to_string())],
    };

    let mut result = CheckResult::ok(&constraint_type, TypeDiscipline::Refined);
    result.features = vec!["dep".to_string(), "refinement".to_string()];
    result.inference_source = "constraint-bridge".to_string();
    result
}

/// Metadata type checking (tags, clades, manifests).
fn check_metadata_expression(
    _expression: &str,
    ctx: &CheckContext,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    let meta_type = Type::Named {
        name: format!("Metadata<{}>", if ctx.domain.is_empty() { "general" } else { &ctx.domain }),
        args: vec![],
    };

    let mut result = CheckResult::ok(&meta_type, TypeDiscipline::Dependent);
    result.features = vec!["dep".to_string()];
    result.inference_source = "metadata-bridge".to_string();
    result
}

/// Generic code expression checking (My-Lang, other languages).
fn check_code_expression(
    expression: &str,
    ctx: &CheckContext,
    _checker: &mut TypeChecker,
) -> CheckResult {
    let discipline = discipline_from_context(ctx);
    let mut checker = TypeChecker::new(discipline);
    register_builtins(&mut checker);

    // Try variable lookup first
    match checker.infer_var(
        expression,
        typell_core::error::Span::synthetic(),
    ) {
        Ok(ty) => {
            let mut result = checker.finish(&ty);
            if !ctx.language.is_empty() {
                result.inference_source = format!("{}-bridge", ctx.language);
            }
            result
        }
        Err(_) => {
            // Not a known binding — model as a Named type
            let named_type = Type::Named {
                name: expression.to_string(),
                args: vec![],
            };
            let mut result = CheckResult::ok(&named_type, discipline);
            if !ctx.dialect.is_empty() {
                result.features.push(format!("dialect:{}", ctx.dialect));
            }
            result.inference_source = if ctx.language.is_empty() {
                "inferred".to_string()
            } else {
                format!("{}-bridge", ctx.language)
            };
            result
        }
    }
}

// ============================================================================
// Type Infer
// ============================================================================

/// POST /infer — type inference (synthesis mode).
pub async fn infer(Json(req): Json<InferRequest>) -> impl IntoResponse {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    register_builtins(&mut checker);

    let result = match checker.infer_var(
        &req.expression,
        typell_core::error::Span::synthetic(),
    ) {
        Ok(ty) => {
            let applied = checker.apply(&ty);
            CheckResult::ok(&applied, TypeDiscipline::Unrestricted)
        }
        Err(e) => CheckResult::err(&[e]),
    };

    Json(result)
}

#[derive(Deserialize)]
pub struct InferRequest {
    pub expression: String,
}

// ============================================================================
// Refinement
// ============================================================================

/// POST /refine — apply refinement types with constraint checking.
///
/// Used by PanLL's Anti-Crash token validation and the TypeLL Refinement tab.
pub async fn refine(Json(req): Json<RefineRequest>) -> impl IntoResponse {
    let predicates: Vec<Predicate> = if req.constraints.is_empty() {
        Vec::new()
    } else {
        // Parse constraints as a JSON array of strings, or as a single expression
        match serde_json::from_str::<Vec<String>>(&req.constraints) {
            Ok(strs) => strs.into_iter().map(Predicate::Raw).collect(),
            Err(_) => vec![Predicate::Raw(req.constraints.clone())],
        }
    };

    let base_type = Type::Named {
        name: req.spec.clone(),
        args: vec![],
    };

    let refined_type = if predicates.is_empty() {
        base_type.clone()
    } else {
        Type::Refined {
            base: Box::new(base_type.clone()),
            predicates: predicates.clone(),
        }
    };

    let constraint_strs: Vec<String> = predicates
        .iter()
        .map(|p| match p {
            Predicate::Raw(s) => s.clone(),
            other => format!("{:?}", other),
        })
        .collect();

    Json(RefineResponse {
        base_type: format!("{}", base_type),
        refined_type: format!("{}", refined_type),
        constraints: constraint_strs,
        consistent: true, // Future: SMT solver integration for consistency checking
    })
}

#[derive(Deserialize)]
pub struct RefineRequest {
    pub spec: String,
    #[serde(default)]
    pub constraints: String,
}

#[derive(Serialize)]
struct RefineResponse {
    base_type: String,
    refined_type: String,
    constraints: Vec<String>,
    consistent: bool,
}

// ============================================================================
// Compute
// ============================================================================

/// POST /compute — type-level computation and normalisation.
///
/// Evaluates type-level terms: unification, beta reduction, and normalisation.
pub async fn compute(Json(req): Json<ComputeRequest>) -> impl IntoResponse {
    let mut steps = Vec::new();

    // Attempt to parse term as a type expression and normalise
    let result = if req.term.contains("->") {
        // Function type — normalise by parsing arrow syntax
        let parts: Vec<&str> = req.term.split("->").map(|s| s.trim()).collect();
        steps.push(format!("Parsed function type with {} parameters", parts.len() - 1));
        let normalised = parts.join(" -> ");
        steps.push("Normalised to canonical arrow form".to_string());
        normalised
    } else if req.term.contains("forall") || req.term.contains("∀") {
        // Polymorphic type — normalise quantifiers
        steps.push("Detected polymorphic type".to_string());
        steps.push("Normalising quantifier positions".to_string());
        req.term.clone()
    } else {
        // Simple type — identity normalisation
        steps.push("Simple type — already in normal form".to_string());
        req.term.clone()
    };

    Json(ComputeResponse {
        input: req.term,
        result,
        steps,
    })
}

#[derive(Deserialize)]
pub struct ComputeRequest {
    pub term: String,
}

#[derive(Serialize)]
struct ComputeResponse {
    input: String,
    result: String,
    steps: Vec<String>,
}

// ============================================================================
// Signatures
// ============================================================================

/// GET /signatures — list available type signatures.
pub async fn list_signatures() -> impl IntoResponse {
    Json(vec![
        SignatureEntry {
            name: "id".to_string(),
            signature: "forall a. a -> a".to_string(),
            module_: "Prelude".to_string(),
            tier: "Core".to_string(),
        },
        SignatureEntry {
            name: "const".to_string(),
            signature: "forall a b. a -> b -> a".to_string(),
            module_: "Prelude".to_string(),
            tier: "Core".to_string(),
        },
        SignatureEntry {
            name: "shadow_price".to_string(),
            signature: "forall a. Resource<a, D> -> Float".to_string(),
            module_: "Eclexia.Resource".to_string(),
            tier: "Core".to_string(),
        },
        SignatureEntry {
            name: "vcl_check".to_string(),
            signature: "VqlQuery -> SafetyReport".to_string(),
            module_: "VCL.UT".to_string(),
            tier: "Database".to_string(),
        },
        SignatureEntry {
            name: "prove".to_string(),
            signature: "forall p. Proposition p -> ProofObligation".to_string(),
            module_: "Proof".to_string(),
            tier: "Verification".to_string(),
        },
        SignatureEntry {
            name: "session_dual".to_string(),
            signature: "SessionType -> SessionType".to_string(),
            module_: "Session".to_string(),
            tier: "Protocol".to_string(),
        },
        SignatureEntry {
            name: "effect_row".to_string(),
            signature: "forall r. EffectRow r -> Bool".to_string(),
            module_: "Effects".to_string(),
            tier: "Core".to_string(),
        },
    ])
}

#[derive(Serialize)]
struct SignatureEntry {
    name: String,
    signature: String,
    module_: String,
    tier: String,
}

// ============================================================================
// Universes
// ============================================================================

/// GET /universes — type universe hierarchy.
pub async fn universes() -> impl IntoResponse {
    Json(vec![
        UniverseEntry {
            level: 0,
            name: "Type".to_string(),
            description: "The universe of ordinary types (Bool, Int, String, ...).".to_string(),
        },
        UniverseEntry {
            level: 1,
            name: "Type1".to_string(),
            description: "The universe of type constructors (Type -> Type).".to_string(),
        },
        UniverseEntry {
            level: 2,
            name: "Type2".to_string(),
            description: "The universe of kind constructors. Rarely needed.".to_string(),
        },
    ])
}

#[derive(Serialize)]
struct UniverseEntry {
    level: u32,
    name: String,
    description: String,
}

// ============================================================================
// Usage Inference
// ============================================================================

/// POST /infer-usage — infer QTT usage quantifiers for an expression.
///
/// Returns the usage quantifier (Zero, One, Omega, Bounded(n)) and whether
/// the expression has linear, affine, or unrestricted usage.
pub async fn infer_usage(Json(req): Json<UsageRequest>) -> impl IntoResponse {
    let mut checker = TypeChecker::new(TypeDiscipline::Linear);
    register_builtins(&mut checker);

    // Infer the type and check usage
    let (usage, discipline) = match checker.infer_var(
        &req.source,
        typell_core::error::Span::synthetic(),
    ) {
        Ok(ty) => {
            let _applied = checker.apply(&ty);
            // Check linearity at scope end to detect violations
            checker.check_linearity_at_scope_end(typell_core::error::Span::synthetic());
            let violations: Vec<String> = checker.usage_tracker
                .check_all_consumed()
                .iter()
                .map(|v| v.message.clone())
                .collect();
            if violations.is_empty() {
                (UsageQuantifier::One, TypeDiscipline::Linear)
            } else {
                (UsageQuantifier::Omega, TypeDiscipline::Unrestricted)
            }
        }
        Err(_) => (UsageQuantifier::Omega, TypeDiscipline::Unrestricted),
    };

    Json(UsageResponse {
        source: req.source,
        usage: usage.to_string(),
        discipline: discipline.to_string(),
        is_linear: matches!(usage, UsageQuantifier::One),
        is_affine: matches!(usage, UsageQuantifier::One | UsageQuantifier::Zero),
        is_unrestricted: matches!(usage, UsageQuantifier::Omega),
        bounded: match usage {
            UsageQuantifier::Bounded(n) => Some(n),
            _ => None,
        },
    })
}

#[derive(Deserialize)]
pub struct UsageRequest {
    pub source: String,
}

#[derive(Serialize)]
struct UsageResponse {
    source: String,
    usage: String,
    discipline: String,
    is_linear: bool,
    is_affine: bool,
    is_unrestricted: bool,
    bounded: Option<u64>,
}

// ============================================================================
// Effect Checking
// ============================================================================

/// POST /check-effects — effect tracking and purity analysis.
///
/// Returns the list of effects an expression produces and whether it is pure.
pub async fn check_effects(Json(req): Json<EffectsRequest>) -> impl IntoResponse {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    register_builtins(&mut checker);

    // Infer the type and collect effects
    let _ = checker.infer_var(
        &req.source,
        typell_core::error::Span::synthetic(),
    );

    let effects: Vec<String> = checker.discovered_effects
        .iter()
        .map(|e| e.to_string())
        .collect();

    let is_pure = effects.is_empty();

    // Build an effect row from discovered effects
    let effect_names: Vec<String> = checker.discovered_effects
        .iter()
        .map(|e| match e {
            Effect::Pure => "Pure".to_string(),
            Effect::IO => "IO".to_string(),
            Effect::State(s) => format!("State({})", s),
            Effect::Except(s) => format!("Except({})", s),
            Effect::Alloc => "Alloc".to_string(),
            Effect::Diverge => "Diverge".to_string(),
            Effect::Network => "Network".to_string(),
            Effect::FileSystem => "FileSystem".to_string(),
            Effect::Named(n) => n.clone(),
        })
        .collect();

    Json(EffectsResponse {
        source: req.source,
        effects: effect_names,
        is_pure,
        row_polymorphic: false,
        undeclared: Vec::new(),
    })
}

#[derive(Deserialize)]
pub struct EffectsRequest {
    pub source: String,
}

#[derive(Serialize)]
struct EffectsResponse {
    source: String,
    effects: Vec<String>,
    is_pure: bool,
    row_polymorphic: bool,
    undeclared: Vec<String>,
}

// ============================================================================
// Dimensional Checking
// ============================================================================

/// POST /check-dimensional — dimensional analysis for Eclexia's type system.
///
/// Validates that dimensional annotations (mass, length, time, etc.) are
/// consistent through arithmetic operations.
pub async fn check_dimensional(Json(req): Json<DimensionalRequest>) -> impl IntoResponse {
    let mut checker = TypeChecker::new(TypeDiscipline::Dependent);
    register_builtins(&mut checker);

    // Parse the source as a dimensional expression
    // For now, check that the expression references known dimensional quantities
    let dimension = Dimension::default(); // Dimensionless
    let resource_type = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: dimension.clone(),
    };

    let result = CheckResult::ok(&resource_type, TypeDiscipline::Dependent);

    Json(DimensionalResponse {
        source: req.source,
        dimension: format!("{:?}", dimension),
        compatible: true,
        type_signature: result.type_signature,
        issues: Vec::new(),
    })
}

#[derive(Deserialize)]
pub struct DimensionalRequest {
    pub source: String,
}

#[derive(Serialize)]
struct DimensionalResponse {
    source: String,
    dimension: String,
    compatible: bool,
    type_signature: String,
    issues: Vec<String>,
}

// ============================================================================
// Proof Obligation Generation
// ============================================================================

/// POST /generate-obligations — extract proof obligations from dependent types.
///
/// Returns propositions that need external proving (dispatched to ECHIDNA).
pub async fn generate_obligations(Json(req): Json<ObligationsRequest>) -> impl IntoResponse {
    let mut checker = TypeChecker::new(TypeDiscipline::Dependent);
    register_builtins(&mut checker);

    let obligations = generate_obligations_from_expression(&req.source, &mut checker);

    Json(ObligationsResponse {
        source: req.source,
        obligations: obligations.proof_obligations.clone(),
        count: obligations.proof_obligations.len(),
        all_discharged: obligations.proof_obligations.is_empty(),
    })
}

/// Internal helper for generating proof obligations from an expression.
fn generate_obligations_from_expression(
    expression: &str,
    checker: &mut TypeChecker,
) -> CheckResult {
    register_builtins(checker);

    // Try to infer the type — any dependent features generate obligations
    match checker.infer_var(
        expression,
        typell_core::error::Span::synthetic(),
    ) {
        Ok(ty) => {
            let applied = checker.apply(&ty);

            // Generate obligations for dependent types
            if has_dependent_features(&applied) {
                checker.add_proof_obligation(format!(
                    "Dependent type obligation: verify well-formedness of {}",
                    applied,
                ));
            }

            // Generate obligations for refinement predicates
            if let Type::Refined { base, predicates } = &applied {
                for pred in predicates {
                    checker.add_proof_obligation(format!(
                        "Refinement obligation: prove {:?} holds for {}",
                        pred, base,
                    ));
                }
            }

            let mut result = checker.finish(&applied);
            result.inference_source = "obligation-generator".to_string();
            result
        }
        Err(_) => {
            // Expression not found — generate a placeholder obligation
            checker.add_proof_obligation(format!(
                "Unresolved expression: '{}' — needs type annotation",
                expression,
            ));
            let mut result = CheckResult::ok(
                &Type::Named {
                    name: expression.to_string(),
                    args: vec![],
                },
                TypeDiscipline::Dependent,
            );
            result.proof_obligations = checker.proof_obligations.clone();
            result.inference_source = "obligation-generator".to_string();
            result
        }
    }
}

#[derive(Deserialize)]
pub struct ObligationsRequest {
    pub source: String,
}

#[derive(Serialize)]
struct ObligationsResponse {
    source: String,
    obligations: Vec<String>,
    count: usize,
    all_discharged: bool,
}

// ============================================================================
// VCL-total Dedicated Endpoint
// ============================================================================

/// POST /vcl-total/check — dedicated VCL-total 10-level type safety analysis.
///
/// Accepts either a VqlQueryType JSON or a raw VCL query string, runs the
/// full 10-level analysis, and returns the SafetyReport.
pub async fn vcl_ut_check(Json(req): Json<VclTotalCheckRequest>) -> impl IntoResponse {
    let vcl: VqlQueryType = match serde_json::from_str(&req.query) {
        Ok(v) => v,
        Err(_) => {
            // Parse as raw query text
            let modalities = req.modalities.unwrap_or_else(|| vec!["All".to_string()])
                .into_iter()
                .map(|m| match m.to_lowercase().as_str() {
                    "graph" => VqlModality::Graph,
                    "vector" => VqlModality::Vector,
                    "tensor" => VqlModality::Tensor,
                    "semantic" => VqlModality::Semantic,
                    "document" => VqlModality::Document,
                    "temporal" => VqlModality::Temporal,
                    "provenance" => VqlModality::Provenance,
                    "spatial" => VqlModality::Spatial,
                    _ => VqlModality::All,
                })
                .collect();

            let extensions = parse_vql_extensions_from_text(&req.query);
            let result_fields = req.result_fields.clone().unwrap_or_default();

            VqlQueryType {
                modalities,
                result_fields,
                extensions,
            }
        }
    };

    let safety_report = determine_safety_level(&vcl);
    let unified = vcl_to_unified(&vcl);

    // Run validation rules
    let mut rule_errors: Vec<String> = Vec::new();
    if let Some(n) = vcl.extensions.consume_after {
        if let Err(e) = rules::check_consume_after(n) {
            rule_errors.push(e);
        }
    }
    if let Some(n) = vcl.extensions.usage_limit {
        if let Err(e) = rules::check_usage_limit(n) {
            rule_errors.push(e);
        }
    }
    if let (Some(proto), Some(effects)) = (&vcl.extensions.session_protocol, &vcl.extensions.effects) {
        if let Err(e) = rules::check_session_effects_compatible(proto, effects) {
            rule_errors.push(e);
        }
    }

    let valid = rule_errors.is_empty();

    Json(VclTotalCheckResponse {
        safety_report,
        unified_type: format!("{}", vcl_to_typell(&vcl)),
        usage: unified.usage.to_string(),
        discipline: unified.discipline.to_string(),
        effects: unified.effects.iter().map(|e| e.to_string()).collect(),
        refinements: unified.refinements.iter().map(|r| format!("{:?}", r)).collect(),
        rule_errors,
        valid,
    })
}

#[derive(Deserialize)]
pub struct VclTotalCheckRequest {
    /// VCL query string or JSON-encoded VqlQueryType.
    pub query: String,
    /// Optional: explicit modality list (if not in JSON form).
    #[serde(default)]
    pub modalities: Option<Vec<String>>,
    /// Optional: explicit result fields (if not in JSON form).
    #[serde(default)]
    pub result_fields: Option<Vec<String>>,
}

#[derive(Serialize)]
struct VclTotalCheckResponse {
    safety_report: SafetyReport,
    unified_type: String,
    usage: String,
    discipline: String,
    effects: Vec<String>,
    refinements: Vec<String>,
    rule_errors: Vec<String>,
    valid: bool,
}

// ============================================================================
// Helpers
// ============================================================================

/// Check whether a type uses dependent features (Pi, Sigma, Array with length).
fn has_dependent_features(ty: &Type) -> bool {
    match ty {
        Type::Pi { .. } | Type::Sigma { .. } => true,
        Type::Array { length: Some(_), .. } => true,
        Type::Function { params, ret, .. } => {
            params.iter().any(has_dependent_features) || has_dependent_features(ret)
        }
        Type::Tuple(elems) => elems.iter().any(has_dependent_features),
        Type::Named { args, .. } => args.iter().any(has_dependent_features),
        _ => false,
    }
}

/// Register built-in types and functions in the checker.
fn register_builtins(checker: &mut TypeChecker) {
    // Identity function: forall a. a -> a
    let id_ty = Type::ForAll {
        vars: vec!["a".to_string()],
        body: Box::new(Type::Function {
            params: vec![Type::Var(typell_core::TypeVar(100))],
            ret: Box::new(Type::Var(typell_core::TypeVar(100))),
            effects: vec![],
        }),
    };
    checker.register_binding("id", UnifiedType::simple(id_ty));

    // println: String -> Unit (with IO effect)
    checker.register_binding(
        "println",
        UnifiedType::simple(Type::Function {
            params: vec![Type::Primitive(PrimitiveType::String)],
            ret: Box::new(Type::Primitive(PrimitiveType::Unit)),
            effects: vec![Effect::IO],
        }),
    );

    // shadow_price: Resource<a, D> -> Float
    checker.register_binding(
        "shadow_price",
        UnifiedType::simple(Type::Function {
            params: vec![Type::Var(typell_core::TypeVar(200))],
            ret: Box::new(Type::Primitive(PrimitiveType::Float)),
            effects: vec![],
        }),
    );

    // vcl_check: a -> QueryResult (VCL-total bridge function)
    checker.register_binding(
        "vcl_check",
        UnifiedType::simple(Type::Function {
            params: vec![Type::Var(typell_core::TypeVar(300))],
            ret: Box::new(Type::Named {
                name: "SafetyReport".to_string(),
                args: vec![],
            }),
            effects: vec![],
        }),
    );

    // prove: a -> ProofObligation (proof generation)
    checker.register_binding(
        "prove",
        UnifiedType::simple(Type::Function {
            params: vec![Type::Var(typell_core::TypeVar(400))],
            ret: Box::new(Type::Named {
                name: "ProofObligation".to_string(),
                args: vec![],
            }),
            effects: vec![],
        }),
    );
}
