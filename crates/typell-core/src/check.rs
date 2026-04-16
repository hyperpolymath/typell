// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Type checking coordinator for the TypeLL kernel.
//!
//! This module provides the high-level `TypeChecker` that orchestrates
//! unification, inference, linearity checking, effect tracking, and
//! dimensional analysis into a single coherent pipeline.
//!
//! Language-specific frontends (e.g., `typell-eclexia`) convert their
//! ASTs into TypeLL's representation, then call into the checker.

use crate::error::{Span, TypeError, TypeResult};
use crate::infer::InferCtx;
use crate::linear::UsageTracker;
use crate::types::{
    Effect, Type, TypeDiscipline, TypeScheme, UnifiedType, UsageQuantifier,
};
use crate::unify::Unifier;
use serde::{Deserialize, Serialize};

/// Result of checking a single expression or declaration.
///
/// Wire-compatible with `typeCheckResult` in `TypeLLModel.res`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    /// Whether the expression type-checks.
    pub valid: bool,
    /// Inferred or checked type signature (display string).
    pub type_signature: String,
    /// Human-readable explanation.
    pub explanation: String,
    /// Proof obligations generated (dependent types).
    pub proof_obligations: Vec<String>,
    /// Effects detected.
    pub effects: Vec<String>,
    /// Linearity issues found.
    pub linearity_issues: Vec<String>,
    /// Session protocol notes.
    pub session_notes: Vec<String>,
    /// Active type feature codes.
    pub features: Vec<String>,
    /// Usage quantifier determined.
    pub usage: String,
    /// Type discipline in effect.
    pub discipline: String,
    /// How the type was determined.
    pub inference_source: String,
}

impl CheckResult {
    /// Create a successful result.
    pub fn ok(ty: &Type, discipline: TypeDiscipline) -> Self {
        let sig = ty.to_string();
        Self {
            valid: true,
            type_signature: sig.clone(),
            explanation: format!("Expression has type: {sig}"),
            proof_obligations: Vec::new(),
            effects: Vec::new(),
            linearity_issues: Vec::new(),
            session_notes: Vec::new(),
            features: Vec::new(),
            usage: "w".to_string(),
            discipline: discipline.to_string(),
            inference_source: "inferred".to_string(),
        }
    }

    /// Create a failure result from errors.
    pub fn err(errors: &[TypeError]) -> Self {
        let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
        Self {
            valid: false,
            type_signature: "<error>".to_string(),
            explanation: messages.join("; "),
            proof_obligations: Vec::new(),
            effects: Vec::new(),
            linearity_issues: Vec::new(),
            session_notes: Vec::new(),
            features: Vec::new(),
            usage: "w".to_string(),
            discipline: "unrestricted".to_string(),
            inference_source: "inferred".to_string(),
        }
    }
}

/// The TypeLL type checker — orchestrates all analysis phases.
pub struct TypeChecker {
    /// Unification engine.
    pub unifier: Unifier,
    /// Inference context (variable bindings, fresh variables).
    pub ctx: InferCtx,
    /// Linear/affine usage tracker.
    pub usage_tracker: UsageTracker,
    /// Active type discipline for the current scope.
    pub discipline: TypeDiscipline,
    /// Collected errors.
    pub errors: Vec<TypeError>,
    /// Collected effects discovered during checking.
    pub discovered_effects: Vec<Effect>,
    /// Proof obligations generated during checking.
    pub proof_obligations: Vec<String>,
}

impl TypeChecker {
    /// Create a new type checker with the given discipline.
    pub fn new(discipline: TypeDiscipline) -> Self {
        Self {
            unifier: Unifier::new(),
            ctx: InferCtx::new(),
            usage_tracker: UsageTracker::new(),
            discipline,
            errors: Vec::new(),
            discovered_effects: Vec::new(),
            proof_obligations: Vec::new(),
        }
    }

    /// Register a binding in the type environment.
    pub fn register_binding(&mut self, name: &str, ty: UnifiedType) {
        self.ctx.insert(name.to_string(), ty.clone());

        // Track usage if we're in a linear/affine discipline
        match self.discipline {
            TypeDiscipline::Linear => {
                self.usage_tracker.declare(name.to_string(), UsageQuantifier::One);
            }
            TypeDiscipline::Affine => {
                self.usage_tracker.declare(name.to_string(), UsageQuantifier::One);
            }
            _ => {}
        }
    }

    /// Register a polymorphic binding.
    pub fn register_scheme(&mut self, name: &str, scheme: TypeScheme) {
        self.ctx.insert_scheme(name.to_string(), scheme);
    }

    /// Record that a variable was used.
    pub fn record_use(&mut self, name: &str, span: Span) {
        if let Some(violation) = self.usage_tracker.record_use(name) {
            self.errors.push(TypeError::LinearityViolation {
                span,
                variable: name.to_string(),
                expected_usage: violation.expected,
                actual_usage: violation.actual,
                message: violation.message,
            });
        }
    }

    /// Record that an effect was observed.
    pub fn record_effect(&mut self, effect: Effect) {
        if !self.discovered_effects.contains(&effect) {
            self.discovered_effects.push(effect);
        }
    }

    /// Generate a proof obligation.
    pub fn add_proof_obligation(&mut self, obligation: String) {
        self.proof_obligations.push(obligation);
    }

    /// Check that all linearity constraints are satisfied at scope end.
    pub fn check_linearity_at_scope_end(&mut self, span: Span) {
        for violation in self.usage_tracker.check_all_consumed() {
            self.errors.push(TypeError::LinearityViolation {
                span,
                variable: violation.variable.clone(),
                expected_usage: violation.expected,
                actual_usage: violation.actual,
                message: violation.message,
            });
        }
    }

    /// Infer the type of a variable reference.
    pub fn infer_var(&mut self, name: &str, span: Span) -> TypeResult<Type> {
        self.record_use(name, span);
        self.ctx.synthesize_var(name, span)
    }

    /// Unify two types.
    pub fn unify(&mut self, t1: &Type, t2: &Type, span: Span) -> TypeResult<()> {
        self.unifier.unify(t1, t2, span)
    }

    /// Apply the current substitution to a type.
    pub fn apply(&self, ty: &Type) -> Type {
        self.unifier.substitution.apply(ty)
    }

    /// Generate a fresh type variable.
    pub fn fresh_var(&mut self) -> Type {
        self.ctx.fresh_var()
    }

    /// Generalize a type (let-polymorphism).
    pub fn generalize(&self, ty: &Type) -> TypeScheme {
        self.ctx.generalize(ty, &self.unifier.substitution)
    }

    /// Instantiate a type scheme with fresh variables.
    pub fn instantiate(&mut self, scheme: &TypeScheme) -> Type {
        self.ctx.instantiate(scheme)
    }

    /// Produce the final check result.
    pub fn finish(&mut self, ty: &Type) -> CheckResult {
        if self.errors.is_empty() {
            let applied = self.apply(ty);
            let mut result = CheckResult::ok(&applied, self.discipline);
            result.effects = self.discovered_effects.iter().map(|e| e.to_string()).collect();
            result.proof_obligations = self.proof_obligations.clone();
            result.linearity_issues = self.usage_tracker
                .check_all_consumed()
                .iter()
                .map(|v| v.message.clone())
                .collect();
            // Detect active features
            if has_dependent_features(ty) { result.features.push("dep".to_string()); }
            if self.discipline == TypeDiscipline::Linear { result.features.push("lin".to_string()); }
            if self.discipline == TypeDiscipline::Affine { result.features.push("aff".to_string()); }
            if !self.discovered_effects.is_empty() { result.features.push("eff".to_string()); }
            if has_resource_type(ty) { result.features.push("dep".to_string()); } // dimensional = dependent
            result
        } else {
            CheckResult::err(&self.errors)
        }
    }
}

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

/// Check whether a type contains resource types (dimensional analysis).
fn has_resource_type(ty: &Type) -> bool {
    match ty {
        Type::Resource { .. } => true,
        Type::Function { params, ret, .. } => {
            params.iter().any(has_resource_type) || has_resource_type(ret)
        }
        Type::Tuple(elems) => elems.iter().any(has_resource_type),
        Type::Named { args, .. } => args.iter().any(has_resource_type),
        Type::Array { elem, .. } => has_resource_type(elem),
        _ => false,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::PrimitiveType;

    #[test]
    fn test_checker_basic_inference() {
        let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
        checker.register_binding(
            "x",
            UnifiedType::simple(Type::Primitive(PrimitiveType::Int)),
        );
        let ty = checker.infer_var("x", Span::synthetic()).expect("TODO: handle error");
        assert_eq!(ty, Type::Primitive(PrimitiveType::Int));
    }

    #[test]
    fn test_checker_unification() {
        let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
        let var = checker.fresh_var();
        let int = Type::Primitive(PrimitiveType::Int);
        checker.unify(&var, &int, Span::synthetic()).expect("TODO: handle error");
        assert_eq!(checker.apply(&var), int);
    }

    #[test]
    fn test_checker_result_ok() {
        let ty = Type::Primitive(PrimitiveType::Bool);
        let result = CheckResult::ok(&ty, TypeDiscipline::Unrestricted);
        assert!(result.valid);
        assert_eq!(result.type_signature, "Bool");
    }
}
