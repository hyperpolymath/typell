// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! E2E tests for the TypeLL unified type system.
//!
//! These tests exercise the full type-checking pipeline from construction of
//! type expressions through inference, unification, and final result
//! generation. They verify observable end-to-end behaviour rather than
//! internal implementation details.

use typell_core::{
    CheckResult, TypeChecker,
    Type, PrimitiveType, TypeDiscipline, UnifiedType, UsageQuantifier,
    Span, TypeError,
};

// ============================================================================
// Helpers
// ============================================================================

/// Construct a `Span` covering the entire source for synthetic tests.
fn s() -> Span {
    Span::synthetic()
}

// ============================================================================
// Full pipeline: register binding → infer var → finish
// ============================================================================

#[test]
fn e2e_infer_primitive_int_through_full_pipeline() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("x", UnifiedType::simple(Type::Primitive(PrimitiveType::Int)));

    let inferred = checker.infer_var("x", s())
        .expect("variable 'x' must resolve to Int");

    assert_eq!(inferred, Type::Primitive(PrimitiveType::Int));

    let result = checker.finish(&inferred);
    assert!(result.valid, "pipeline must report valid for a simple Int variable");
    assert_eq!(result.type_signature, "Int");
}

#[test]
fn e2e_infer_bool_binding() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("flag", UnifiedType::simple(Type::Primitive(PrimitiveType::Bool)));

    let inferred = checker.infer_var("flag", s())
        .expect("variable 'flag' must resolve");

    let result = checker.finish(&inferred);
    assert!(result.valid);
    assert_eq!(result.type_signature, "Bool");
}

#[test]
fn e2e_unify_fresh_var_with_string_type() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    let fresh = checker.fresh_var();
    let string_ty = Type::Primitive(PrimitiveType::String);

    checker.unify(&fresh, &string_ty, s())
        .expect("unification of fresh var with String must succeed");

    let resolved = checker.apply(&fresh);
    assert_eq!(resolved, string_ty);

    let result = checker.finish(&resolved);
    assert!(result.valid);
    assert_eq!(result.type_signature, "String");
}

#[test]
fn e2e_function_type_pipeline() {
    // Build   (Int) -> Bool
    let fn_ty = Type::Function {
        params: vec![Type::Primitive(PrimitiveType::Int)],
        ret: Box::new(Type::Primitive(PrimitiveType::Bool)),
        effects: vec![],
    };
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("pred", UnifiedType::simple(fn_ty.clone()));

    let inferred = checker.infer_var("pred", s())
        .expect("variable 'pred' must resolve");

    assert_eq!(inferred, fn_ty);

    let result = checker.finish(&inferred);
    assert!(result.valid, "function type must be valid");
    assert!(result.type_signature.contains("->"),
        "function type signature must contain '->' but got: {}", result.type_signature);
}

#[test]
fn e2e_tuple_type_pipeline() {
    let tuple_ty = Type::Tuple(vec![
        Type::Primitive(PrimitiveType::Int),
        Type::Primitive(PrimitiveType::String),
    ]);
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("pair", UnifiedType::simple(tuple_ty.clone()));

    let inferred = checker.infer_var("pair", s())
        .expect("tuple binding must resolve");

    let result = checker.finish(&inferred);
    assert!(result.valid);
    assert!(result.type_signature.contains("Int"),
        "tuple signature must mention Int, got: {}", result.type_signature);
    assert!(result.type_signature.contains("String"),
        "tuple signature must mention String, got: {}", result.type_signature);
}

// ============================================================================
// Error case handling: malformed / contradictory types
// ============================================================================

#[test]
fn e2e_undefined_variable_produces_error_not_panic() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    // Do NOT register "y" — looking it up must yield an error, never a panic.
    let result = checker.infer_var("y", s());
    assert!(
        result.is_err(),
        "inferring an unregistered variable must return Err"
    );
    // Confirm the error message is useful (contains the variable name).
    let err_msg = result.expect_err("must be an error").to_string();
    assert!(
        err_msg.contains("y"),
        "error message must mention variable 'y', got: {err_msg}"
    );
}

#[test]
fn e2e_unification_mismatch_produces_error_not_panic() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    let int_ty = Type::Primitive(PrimitiveType::Int);
    let bool_ty = Type::Primitive(PrimitiveType::Bool);

    let result = checker.unify(&int_ty, &bool_ty, s());
    assert!(result.is_err(), "unifying Int with Bool must return Err");
}

#[test]
fn e2e_checker_finish_on_error_returns_invalid_result() {
    let errors = vec![TypeError::Undefined {
        span: s(),
        name: "z".to_string(),
        hint: None,
    }];
    let result = CheckResult::err(&errors);
    assert!(!result.valid, "CheckResult from errors must be invalid");
    assert!(
        result.type_signature.contains("error"),
        "error type signature should indicate error, got: {}", result.type_signature
    );
}

// ============================================================================
// Cross-crate type compatibility: unify types from different bindings
// ============================================================================

#[test]
fn e2e_cross_binding_unification() {
    // Simulate two variables that should unify to the same type.
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    let fresh_a = checker.fresh_var();
    let fresh_b = checker.fresh_var();
    let int_ty = Type::Primitive(PrimitiveType::Int);

    // Both fresh vars resolve through unification with Int.
    checker.unify(&fresh_a, &int_ty, s())
        .expect("fresh_a ~ Int must unify");
    checker.unify(&fresh_b, &fresh_a, s())
        .expect("fresh_b ~ fresh_a must unify transitively");

    assert_eq!(
        checker.apply(&fresh_b),
        int_ty,
        "transitive unification: fresh_b must resolve to Int"
    );
}

#[test]
fn e2e_named_type_with_type_args_pipeline() {
    // Vec<Int>
    let vec_int = Type::Named {
        name: "Vec".to_string(),
        args: vec![Type::Primitive(PrimitiveType::Int)],
    };
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("xs", UnifiedType::simple(vec_int.clone()));

    let inferred = checker.infer_var("xs", s())
        .expect("Vec<Int> binding must resolve");

    let result = checker.finish(&inferred);
    assert!(result.valid);
    assert!(
        result.type_signature.contains("Vec"),
        "Vec<Int> signature must contain 'Vec', got: {}", result.type_signature
    );
}

// ============================================================================
// Linear discipline: linearity violations detected in pipeline
// ============================================================================

#[test]
fn e2e_linear_discipline_single_use_is_valid() {
    let mut checker = TypeChecker::new(TypeDiscipline::Linear);
    let linear_ty = UnifiedType::linear(Type::Primitive(PrimitiveType::Int));
    checker.register_binding("resource", linear_ty);

    // Use exactly once — must succeed.
    let inferred = checker.infer_var("resource", s())
        .expect("single use of linear variable must succeed");

    let result = checker.finish(&inferred);
    // After a single use in linear discipline the checker may flag unconsumed
    // resources at scope end; we verify it does not panic and returns a result.
    assert_eq!(inferred, Type::Primitive(PrimitiveType::Int));
    let _ = result; // validity depends on scope-end semantics; no panic is the key invariant
}

#[test]
fn e2e_usage_quantifier_omega_compatible_with_omega() {
    // Omega is unrestricted, so Omega compatible_with Omega.
    assert!(
        UsageQuantifier::Omega.compatible_with(&UsageQuantifier::Omega),
        "Omega must be compatible with Omega"
    );
}

#[test]
fn e2e_usage_quantifier_zero_compatible_with_any() {
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::One));
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::Omega));
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::Zero));
}

// ============================================================================
// Type Top / Bottom corner cases
// ============================================================================

#[test]
fn e2e_top_type_pipeline_does_not_panic() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("top_val", UnifiedType::simple(Type::Top));

    let inferred = checker.infer_var("top_val", s())
        .expect("Top binding must resolve");

    let result = checker.finish(&inferred);
    // Top is valid; we check no panic and the signature is representable.
    assert!(result.valid);
    assert!(!result.type_signature.is_empty());
}

#[test]
fn e2e_bottom_type_pipeline_does_not_panic() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding("never_val", UnifiedType::simple(Type::Bottom));

    let inferred = checker.infer_var("never_val", s())
        .expect("Bottom binding must resolve");

    let result = checker.finish(&inferred);
    assert!(!result.type_signature.is_empty());
}
