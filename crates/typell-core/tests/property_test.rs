// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Property-based tests for the TypeLL unified type system.
//!
//! Uses proptest to verify algebraic invariants that must hold for all
//! well-formed type expressions:
//!
//! - Type inference idempotence: re-inferring a resolved type yields the same type
//! - Type equality reflexivity: a type equals itself
//! - Well-formed types never cause panics in the type checking pipeline
//! - UsageQuantifier semiring laws

use proptest::prelude::*;
use typell_core::{
    TypeChecker, TypeDiscipline, UnifiedType,
    Type, PrimitiveType, TypeVar,
    UsageQuantifier,
    Span,
};

// ============================================================================
// Arbitrary generators for type system values
// ============================================================================

/// Generate an arbitrary primitive type.
fn arb_primitive() -> impl Strategy<Value = PrimitiveType> {
    prop_oneof![
        Just(PrimitiveType::Bool),
        Just(PrimitiveType::Int),
        Just(PrimitiveType::I32),
        Just(PrimitiveType::I64),
        Just(PrimitiveType::U8),
        Just(PrimitiveType::U32),
        Just(PrimitiveType::Float),
        Just(PrimitiveType::F64),
        Just(PrimitiveType::Char),
        Just(PrimitiveType::String),
        Just(PrimitiveType::Unit),
        Just(PrimitiveType::Never),
    ]
}

/// Generate a simple (non-recursive) `Type` to avoid infinite recursion in
/// proptest strategies. Simple types are sufficient to exercise the invariants.
fn arb_simple_type() -> impl Strategy<Value = Type> {
    prop_oneof![
        // Primitives
        arb_primitive().prop_map(Type::Primitive),
        // Type variables
        (0u32..16).prop_map(|n| Type::Var(TypeVar(n))),
        // Top and Bottom
        Just(Type::Top),
        Just(Type::Bottom),
        // Named types with no arguments
        "[a-z]{3,8}".prop_map(|name| Type::Named { name, args: vec![] }),
        // Tuple of two primitives
        (arb_primitive(), arb_primitive()).prop_map(|(a, b)| {
            Type::Tuple(vec![Type::Primitive(a), Type::Primitive(b)])
        }),
        // Simple function type
        (arb_primitive(), arb_primitive()).prop_map(|(param, ret)| {
            Type::Function {
                params: vec![Type::Primitive(param)],
                ret: Box::new(Type::Primitive(ret)),
                effects: vec![],
            }
        }),
    ]
}

/// Generate an arbitrary usage quantifier.
fn arb_usage() -> impl Strategy<Value = UsageQuantifier> {
    prop_oneof![
        Just(UsageQuantifier::Zero),
        Just(UsageQuantifier::One),
        Just(UsageQuantifier::Omega),
        (1u64..=10).prop_map(UsageQuantifier::Bounded),
    ]
}

/// Generate an arbitrary type discipline.
fn arb_discipline() -> impl Strategy<Value = TypeDiscipline> {
    prop_oneof![
        Just(TypeDiscipline::Unrestricted),
        Just(TypeDiscipline::Affine),
        Just(TypeDiscipline::Linear),
        Just(TypeDiscipline::Refined),
        Just(TypeDiscipline::Dependent),
    ]
}

// ============================================================================
// Property: type equality is reflexive
// ============================================================================

proptest! {
    #[test]
    fn prop_type_equality_reflexive(ty in arb_simple_type()) {
        prop_assert_eq!(&ty, &ty, "every type must equal itself");
    }
}

// ============================================================================
// Property: infer(infer(expr)) == infer(expr)  (idempotence via unification)
//
// We model this as: once a type variable is unified with a concrete type,
// applying the substitution a second time yields the same result as the first.
// ============================================================================

proptest! {
    #[test]
    fn prop_substitution_application_idempotent(ty in arb_simple_type()) {
        let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
        let fresh = checker.fresh_var();

        // Only attempt unification when the concrete type is variable-free.
        // If ty already contains vars, unification may fail with occurs check;
        // skip those cases to keep the property well-defined.
        if !ty.has_vars() {
            checker.unify(&fresh, &ty, Span::synthetic())
                .expect("unification with a ground type must succeed");

            let first_apply  = checker.apply(&fresh);
            let second_apply = checker.apply(&first_apply);
            prop_assert_eq!(
                first_apply, second_apply,
                "applying substitution twice must give the same result (idempotence)"
            );
        }
    }
}

// ============================================================================
// Property: well-formed ground types never panic in the full pipeline
// ============================================================================

proptest! {
    #[test]
    fn prop_ground_types_never_panic_in_pipeline(
        ty in arb_simple_type(),
        discipline in arb_discipline(),
    ) {
        // Only exercise ground (variable-free) types to avoid spurious
        // unification failures unrelated to the "no panic" invariant.
        if !ty.has_vars() {
            let mut checker = TypeChecker::new(discipline);
            checker.register_binding("v", UnifiedType::simple(ty.clone()));

            // infer_var may still return Err for discipline-specific reasons,
            // but it must NEVER panic.
            let result = checker.infer_var("v", Span::synthetic());
            if let Ok(inferred) = result {
                // finish() must also not panic.
                let check_result = checker.finish(&inferred);
                // The type signature must be a non-empty string.
                prop_assert!(
                    !check_result.type_signature.is_empty(),
                    "type_signature must not be empty"
                );
            }
        }
    }
}

// ============================================================================
// Property: UsageQuantifier::Zero is compatible with everything (semiring 0)
// ============================================================================

proptest! {
    #[test]
    fn prop_zero_usage_compatible_with_all(other in arb_usage()) {
        prop_assert!(
            UsageQuantifier::Zero.compatible_with(&other),
            "Zero usage must be compatible with {:?}", other
        );
    }
}

// ============================================================================
// Property: Omega is compatible only with Omega (semiring top)
// ============================================================================

proptest! {
    #[test]
    fn prop_omega_compatible_only_with_omega(other in arb_usage()) {
        let is_omega = matches!(other, UsageQuantifier::Omega);
        let compatible = UsageQuantifier::Omega.compatible_with(&other);
        prop_assert_eq!(
            compatible, is_omega,
            "Omega must be compatible with {:?} only if it is also Omega", other
        );
    }
}

// ============================================================================
// Property: usage addition with Zero is identity (semiring additive identity)
// ============================================================================

proptest! {
    #[test]
    fn prop_usage_add_zero_is_identity(q in arb_usage()) {
        let added = q.add(&UsageQuantifier::Zero);
        prop_assert_eq!(
            added, q,
            "adding Zero to {:?} must return {:?}", q, q
        );
    }
}

// ============================================================================
// Property: type variables have_vars() returns true
// ============================================================================

proptest! {
    #[test]
    fn prop_type_var_has_vars(id in 0u32..100) {
        let ty = Type::Var(TypeVar(id));
        prop_assert!(ty.has_vars(), "Type::Var must report has_vars() == true");
    }
}

// ============================================================================
// Property: primitive types have no type variables
// ============================================================================

proptest! {
    #[test]
    fn prop_primitive_type_no_vars(prim in arb_primitive()) {
        let ty = Type::Primitive(prim);
        prop_assert!(!ty.has_vars(), "Primitive types must not have type variables");
    }
}

// ============================================================================
// Property: fresh type variables are distinct
// ============================================================================

proptest! {
    #[test]
    fn prop_fresh_vars_are_distinct(count in 2usize..=20) {
        let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
        let vars: Vec<Type> = (0..count).map(|_| checker.fresh_var()).collect();

        // All pairs must be distinct.
        for i in 0..vars.len() {
            for j in (i + 1)..vars.len() {
                prop_assert_ne!(
                    &vars[i], &vars[j],
                    "fresh vars at index {} and {} must be distinct", i, j
                );
            }
        }
    }
}
