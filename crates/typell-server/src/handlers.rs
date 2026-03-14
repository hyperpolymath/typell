// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Request handlers for the TypeLL JSON-RPC server.
//!
//! These handlers implement the API that PanLL's `TypeLLCmd.res` calls:
//!
//! | Endpoint          | Method | Description                          |
//! |-------------------|--------|--------------------------------------|
//! | `/health`         | GET    | Server health check                  |
//! | `/check`          | POST   | Bidirectional type checking           |
//! | `/infer`          | POST   | Type inference (synthesis mode)       |
//! | `/refine`         | POST   | Refinement type application           |
//! | `/compute`        | POST   | Type-level computation/normalisation  |
//! | `/signatures`     | GET    | List available type signatures        |
//! | `/universes`      | GET    | Type universe hierarchy               |

use axum::extract::Json;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use typell_core::check::{CheckResult, TypeChecker};
use typell_core::types::{PrimitiveType, Type, TypeDiscipline};

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
        ],
    })
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    features: Vec<String>,
}

// ============================================================================
// Type Check
// ============================================================================

/// POST /check — bidirectional type checking.
pub async fn check(Json(req): Json<CheckRequest>) -> impl IntoResponse {
    let discipline = parse_discipline(&req.context);
    let mut checker = TypeChecker::new(discipline);

    // For the initial scaffold, we do a simple expression-level check.
    // A full implementation would parse the expression and walk the AST.
    // For now, demonstrate the pipeline with a variable lookup.
    let result = if req.expression.is_empty() {
        CheckResult::err(&[typell_core::error::TypeError::Custom {
            span: typell_core::error::Span::synthetic(),
            message: "empty expression".to_string(),
            hint: Some("provide an expression to type-check".to_string()),
        }])
    } else {
        // Register some built-in types for demonstration
        register_builtins(&mut checker);

        // Try to look up as a variable
        match checker.infer_var(
            &req.expression,
            typell_core::error::Span::synthetic(),
        ) {
            Ok(ty) => checker.finish(&ty),
            Err(_) => {
                // If not a known variable, treat as a type expression
                // and return a placeholder result
                CheckResult::ok(
                    &Type::Named {
                        name: req.expression.clone(),
                        args: vec![],
                    },
                    discipline,
                )
            }
        }
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

/// POST /refine — apply refinement types.
pub async fn refine(Json(req): Json<RefineRequest>) -> impl IntoResponse {
    // TODO: implement full refinement type checking with SMT solver integration
    Json(RefineResponse {
        base_type: req.spec.clone(),
        refined_type: format!("{{x : {} | <constraints>}}", req.spec),
        constraints: vec![],
        consistent: true,
    })
}

#[derive(Deserialize)]
pub struct RefineRequest {
    pub spec: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub constraints: String,
}

#[derive(Serialize)]
#[allow(dead_code)]
struct RefineResponse {
    base_type: String,
    refined_type: String,
    constraints: Vec<String>,
    consistent: bool,
}

// ============================================================================
// Compute
// ============================================================================

/// POST /compute — type-level computation.
pub async fn compute(Json(req): Json<ComputeRequest>) -> impl IntoResponse {
    // TODO: implement type-level normalisation and evaluation
    Json(ComputeResponse {
        input: req.term.clone(),
        result: req.term, // Identity for now
        steps: vec![],
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
// Helpers
// ============================================================================

/// Parse a discipline from a context JSON string.
fn parse_discipline(context: &str) -> TypeDiscipline {
    if context.contains("\"linear\"") {
        TypeDiscipline::Linear
    } else if context.contains("\"dependent\"") {
        TypeDiscipline::Dependent
    } else if context.contains("\"refined\"") || context.contains("\"refinement\"") {
        TypeDiscipline::Refined
    } else if context.contains("\"unrestricted\"") {
        TypeDiscipline::Unrestricted
    } else {
        TypeDiscipline::Affine
    }
}

/// Register built-in types and functions in the checker.
fn register_builtins(checker: &mut TypeChecker) {
    use typell_core::types::UnifiedType;

    // Identity function
    let id_ty = Type::ForAll {
        vars: vec!["a".to_string()],
        body: Box::new(Type::Function {
            params: vec![Type::Var(typell_core::TypeVar(100))],
            ret: Box::new(Type::Var(typell_core::TypeVar(100))),
            effects: vec![],
        }),
    };
    checker.register_binding("id", UnifiedType::simple(id_ty));

    // println
    checker.register_binding(
        "println",
        UnifiedType::simple(Type::Function {
            params: vec![Type::Primitive(PrimitiveType::String)],
            ret: Box::new(Type::Primitive(PrimitiveType::Unit)),
            effects: vec![typell_core::Effect::IO],
        }),
    );

    // shadow_price
    checker.register_binding(
        "shadow_price",
        UnifiedType::simple(Type::Function {
            params: vec![Type::Var(typell_core::TypeVar(200))],
            ret: Box::new(Type::Primitive(PrimitiveType::Float)),
            effects: vec![],
        }),
    );
}
