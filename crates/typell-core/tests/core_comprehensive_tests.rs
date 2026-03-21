// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Comprehensive test suite for TypeLL core.
//
// Tests cover:
// - Unification: variable binding, occurs check, function types, tuples,
//   named types, resource dimensions, refined types, Pi/Sigma, session types,
//   Top/Bottom, Error recovery
// - Inference: fresh variables, lookup, child scopes, generalization,
//   instantiation, synthesize_var, check_against
// - Check: TypeChecker pipeline, register_binding, infer_var, unify, finish,
//   linearity at scope end, effect recording, proof obligations
// - Linear: exact-once, at-most-once (affine), erased, omega, bounded
// - Effects: pure row, subrow, merge, handle, effect checking
// - QTT: semiring multiplication, context addition, scaling, compatibility
// - Session: duality, involution, are_dual, well-formedness
// - Dimensional: add/sub/mul/div/pow, resource name lookup, comparison
// - Edge cases: empty programs, recursive types (occurs check), conflicting
//   constraints, error messages, serialisation round-trips

use typell_core::check::{CheckResult, TypeChecker};
use typell_core::dimensional::{check_binary_op, check_binary_op_with_exponent, DimOp, resource_name_to_dimension};
use typell_core::effects::{check_effects, EffectRow};
use typell_core::error::{Span, TypeError};
use typell_core::infer::InferCtx;
use typell_core::linear::UsageTracker;
use typell_core::proof::{eval_predicate, ObligationCollector, ObligationStatus, PredicateResult, Proposition};
use typell_core::qtt::QttContext;
use typell_core::session::{are_dual, dual, is_well_formed};
use typell_core::types::{
    Dimension, Effect, Predicate, PrimitiveType, SessionType, Term, TermOp, Type,
    TypeDiscipline, TypeScheme, TypeVar, UnifiedType, UsageQuantifier,
};
use typell_core::unify::{eval_term_to_i64, terms_unify, Substitution, Unifier};

// ============================================================================
// Unification — additional edge cases
// ============================================================================

#[test]
fn test_unify_tuple_same_elements() {
    let mut u = Unifier::new();
    let t1 = Type::Tuple(vec![
        Type::Primitive(PrimitiveType::Int),
        Type::Primitive(PrimitiveType::Bool),
    ]);
    let t2 = Type::Tuple(vec![
        Type::Primitive(PrimitiveType::Int),
        Type::Primitive(PrimitiveType::Bool),
    ]);
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
}

#[test]
fn test_unify_tuple_different_length_fails() {
    let mut u = Unifier::new();
    let t1 = Type::Tuple(vec![Type::Primitive(PrimitiveType::Int)]);
    let t2 = Type::Tuple(vec![
        Type::Primitive(PrimitiveType::Int),
        Type::Primitive(PrimitiveType::Bool),
    ]);
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_err());
}

#[test]
fn test_unify_named_different_names_fails() {
    let mut u = Unifier::new();
    let t1 = Type::Named {
        name: "Vec".to_string(),
        args: vec![Type::Primitive(PrimitiveType::Int)],
    };
    let t2 = Type::Named {
        name: "List".to_string(),
        args: vec![Type::Primitive(PrimitiveType::Int)],
    };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_err());
}

#[test]
fn test_unify_named_same_name_different_arity_fails() {
    let mut u = Unifier::new();
    let t1 = Type::Named {
        name: "Map".to_string(),
        args: vec![Type::Primitive(PrimitiveType::String)],
    };
    let t2 = Type::Named {
        name: "Map".to_string(),
        args: vec![
            Type::Primitive(PrimitiveType::String),
            Type::Primitive(PrimitiveType::Int),
        ],
    };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_err());
}

#[test]
fn test_unify_pi_types() {
    let mut u = Unifier::new();
    let t1 = Type::Pi {
        param_name: "n".to_string(),
        param_type: Box::new(Type::Primitive(PrimitiveType::Int)),
        body: Box::new(Type::Var(TypeVar(0))),
    };
    let t2 = Type::Pi {
        param_name: "m".to_string(),
        param_type: Box::new(Type::Primitive(PrimitiveType::Int)),
        body: Box::new(Type::Primitive(PrimitiveType::Bool)),
    };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
    assert_eq!(
        u.substitution.apply(&Type::Var(TypeVar(0))),
        Type::Primitive(PrimitiveType::Bool)
    );
}

#[test]
fn test_unify_sigma_types() {
    let mut u = Unifier::new();
    let t1 = Type::Sigma {
        fst_name: "x".to_string(),
        fst_type: Box::new(Type::Var(TypeVar(0))),
        snd_type: Box::new(Type::Primitive(PrimitiveType::Bool)),
    };
    let t2 = Type::Sigma {
        fst_name: "y".to_string(),
        fst_type: Box::new(Type::Primitive(PrimitiveType::Int)),
        snd_type: Box::new(Type::Primitive(PrimitiveType::Bool)),
    };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
    assert_eq!(
        u.substitution.apply(&Type::Var(TypeVar(0))),
        Type::Primitive(PrimitiveType::Int)
    );
}

#[test]
fn test_unify_refined_types_base_must_match() {
    let mut u = Unifier::new();
    let t1 = Type::Refined {
        base: Box::new(Type::Primitive(PrimitiveType::Int)),
        predicates: vec![Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0))],
    };
    let t2 = Type::Refined {
        base: Box::new(Type::Primitive(PrimitiveType::Int)),
        predicates: vec![Predicate::Lt(Term::Var("x".to_string()), Term::Lit(100))],
    };
    // Refined types unify if bases unify (predicates checked separately).
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
}

#[test]
fn test_unify_refined_vs_non_refined_fails() {
    let mut u = Unifier::new();
    let t1 = Type::Refined {
        base: Box::new(Type::Primitive(PrimitiveType::Int)),
        predicates: vec![],
    };
    let t2 = Type::Primitive(PrimitiveType::Int);
    // A refined type and a plain primitive are different constructors.
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_err());
}

#[test]
fn test_unify_error_with_anything() {
    let mut u = Unifier::new();
    assert!(u
        .unify(&Type::Error, &Type::Primitive(PrimitiveType::String), Span::synthetic())
        .is_ok());
    assert!(u
        .unify(
            &Type::Named {
                name: "Foo".to_string(),
                args: vec![],
            },
            &Type::Error,
            Span::synthetic()
        )
        .is_ok());
}

#[test]
fn test_unify_int_and_i64_are_equivalent() {
    let mut u = Unifier::new();
    assert!(u
        .unify(
            &Type::Primitive(PrimitiveType::Int),
            &Type::Primitive(PrimitiveType::I64),
            Span::synthetic()
        )
        .is_ok());
}

#[test]
fn test_unify_float_and_f64_are_equivalent() {
    let mut u = Unifier::new();
    assert!(u
        .unify(
            &Type::Primitive(PrimitiveType::Float),
            &Type::Primitive(PrimitiveType::F64),
            Span::synthetic()
        )
        .is_ok());
}

#[test]
fn test_unify_function_arity_mismatch() {
    let mut u = Unifier::new();
    let f1 = Type::Function {
        params: vec![Type::Primitive(PrimitiveType::Int)],
        ret: Box::new(Type::Primitive(PrimitiveType::Bool)),
        effects: vec![],
    };
    let f2 = Type::Function {
        params: vec![
            Type::Primitive(PrimitiveType::Int),
            Type::Primitive(PrimitiveType::String),
        ],
        ret: Box::new(Type::Primitive(PrimitiveType::Bool)),
        effects: vec![],
    };
    let err = u.unify(&f1, &f2, Span::synthetic()).unwrap_err();
    // The error message should mention arity.
    let msg = err.to_string();
    assert!(msg.contains("mismatch"), "error should mention mismatch: {}", msg);
}

#[test]
fn test_occurs_check_nested_function() {
    let mut u = Unifier::new();
    // t0 = (t0) -> Int  => infinite type
    let var = Type::Var(TypeVar(0));
    let func = Type::Function {
        params: vec![Type::Var(TypeVar(0))],
        ret: Box::new(Type::Primitive(PrimitiveType::Int)),
        effects: vec![],
    };
    let err = u.unify(&var, &func, Span::synthetic()).unwrap_err();
    assert!(matches!(err, TypeError::InfiniteType { .. }));
}

// ============================================================================
// Substitution — chain following
// ============================================================================

#[test]
fn test_substitution_chains() {
    let mut s = Substitution::new();
    s.bind(TypeVar(0), Type::Var(TypeVar(1)));
    s.bind(TypeVar(1), Type::Var(TypeVar(2)));
    s.bind(TypeVar(2), Type::Primitive(PrimitiveType::Bool));
    assert_eq!(
        s.apply(&Type::Var(TypeVar(0))),
        Type::Primitive(PrimitiveType::Bool)
    );
}

#[test]
fn test_substitution_apply_to_function() {
    let mut s = Substitution::new();
    s.bind(TypeVar(0), Type::Primitive(PrimitiveType::Int));
    let func = Type::Function {
        params: vec![Type::Var(TypeVar(0))],
        ret: Box::new(Type::Var(TypeVar(0))),
        effects: vec![],
    };
    let applied = s.apply(&func);
    match applied {
        Type::Function { params, ret, .. } => {
            assert_eq!(params[0], Type::Primitive(PrimitiveType::Int));
            assert_eq!(*ret, Type::Primitive(PrimitiveType::Int));
        }
        _ => panic!("expected Function type"),
    }
}

// ============================================================================
// Inference — additional tests
// ============================================================================

#[test]
fn test_infer_ctx_fresh_vars_are_sequential() {
    let mut ctx = InferCtx::new();
    for i in 0..5 {
        let var = ctx.fresh_var();
        assert_eq!(var, Type::Var(TypeVar(i)));
    }
}

#[test]
fn test_infer_ctx_child_inherits_bindings() {
    let mut parent = InferCtx::new();
    parent.insert(
        "x".to_string(),
        UnifiedType::simple(Type::Primitive(PrimitiveType::Int)),
    );
    parent.insert(
        "y".to_string(),
        UnifiedType::simple(Type::Primitive(PrimitiveType::Bool)),
    );
    let child = parent.child();
    assert!(child.lookup("x").is_some());
    assert!(child.lookup("y").is_some());
    assert!(child.lookup("z").is_none());
}

#[test]
fn test_infer_ctx_child_shadows_parent() {
    let mut parent = InferCtx::new();
    parent.insert(
        "x".to_string(),
        UnifiedType::simple(Type::Primitive(PrimitiveType::Int)),
    );
    let mut child = parent.child();
    child.insert(
        "x".to_string(),
        UnifiedType::simple(Type::Primitive(PrimitiveType::Bool)),
    );
    let scheme = child.lookup("x").unwrap();
    assert_eq!(scheme.body.base, Type::Primitive(PrimitiveType::Bool));
}

#[test]
fn test_synthesize_undefined_var_gives_error() {
    let mut ctx = InferCtx::new();
    let result = ctx.synthesize_var("nonexistent", Span::synthetic());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, TypeError::Undefined { .. }));
}

#[test]
fn test_synthesize_var_returns_correct_type() {
    let mut ctx = InferCtx::new();
    ctx.insert(
        "myVar".to_string(),
        UnifiedType::simple(Type::Primitive(PrimitiveType::String)),
    );
    let ty = ctx.synthesize_var("myVar", Span::synthetic()).unwrap();
    assert_eq!(ty, Type::Primitive(PrimitiveType::String));
}

#[test]
fn test_generalize_with_no_free_vars() {
    let ctx = InferCtx::new();
    let subst = Substitution::new();
    let scheme = ctx.generalize(&Type::Primitive(PrimitiveType::Int), &subst);
    assert!(scheme.vars.is_empty());
}

#[test]
fn test_available_names_includes_parent() {
    let mut parent = InferCtx::new();
    parent.insert("a".to_string(), UnifiedType::simple(Type::Primitive(PrimitiveType::Int)));
    let mut child = parent.child();
    child.insert("b".to_string(), UnifiedType::simple(Type::Primitive(PrimitiveType::Bool)));
    let names = child.available_names();
    assert!(names.contains(&"a".to_string()));
    assert!(names.contains(&"b".to_string()));
}

// ============================================================================
// TypeChecker — end-to-end pipeline tests
// ============================================================================

#[test]
fn test_checker_full_pipeline_basic() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.register_binding(
        "x",
        UnifiedType::simple(Type::Primitive(PrimitiveType::Int)),
    );
    let ty = checker.infer_var("x", Span::synthetic()).unwrap();
    let result = checker.finish(&ty);
    assert!(result.valid);
    assert_eq!(result.type_signature, "Int");
}

#[test]
fn test_checker_unification_through_pipeline() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    let var = checker.fresh_var();
    checker
        .unify(&var, &Type::Primitive(PrimitiveType::Float), Span::synthetic())
        .unwrap();
    let applied = checker.apply(&var);
    assert_eq!(applied, Type::Primitive(PrimitiveType::Float));
    let result = checker.finish(&applied);
    assert!(result.valid);
    assert_eq!(result.type_signature, "Float");
}

#[test]
fn test_checker_linear_discipline_detects_double_use() {
    let mut checker = TypeChecker::new(TypeDiscipline::Linear);
    checker.register_binding(
        "resource",
        UnifiedType::linear(Type::Primitive(PrimitiveType::String)),
    );
    // First use: fine.
    let _ = checker.infer_var("resource", Span::new(0, 10));
    // Second use: linearity violation.
    let _ = checker.infer_var("resource", Span::new(10, 20));
    assert!(!checker.errors.is_empty());
}

#[test]
fn test_checker_linear_discipline_detects_unused() {
    let mut checker = TypeChecker::new(TypeDiscipline::Linear);
    checker.register_binding(
        "resource",
        UnifiedType::linear(Type::Primitive(PrimitiveType::Int)),
    );
    // Never use resource.
    checker.check_linearity_at_scope_end(Span::synthetic());
    assert!(!checker.errors.is_empty());
}

#[test]
fn test_checker_records_effects() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.record_effect(Effect::IO);
    checker.record_effect(Effect::Network);
    checker.record_effect(Effect::IO); // Duplicate — should not be added again.
    assert_eq!(checker.discovered_effects.len(), 2);
}

#[test]
fn test_checker_finish_reports_effects() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.record_effect(Effect::IO);
    let ty = Type::Primitive(PrimitiveType::Int);
    let result = checker.finish(&ty);
    assert!(result.valid);
    assert!(result.effects.contains(&"IO".to_string()));
    assert!(result.features.contains(&"eff".to_string()));
}

#[test]
fn test_checker_finish_reports_linear_feature() {
    let mut checker = TypeChecker::new(TypeDiscipline::Linear);
    checker.register_binding(
        "x",
        UnifiedType::linear(Type::Primitive(PrimitiveType::Int)),
    );
    let _ = checker.infer_var("x", Span::synthetic());
    let ty = Type::Primitive(PrimitiveType::Int);
    let result = checker.finish(&ty);
    assert!(result.features.contains(&"lin".to_string()));
}

#[test]
fn test_checker_finish_reports_dependent_feature_for_pi() {
    let mut checker = TypeChecker::new(TypeDiscipline::Dependent);
    let ty = Type::Pi {
        param_name: "n".to_string(),
        param_type: Box::new(Type::Primitive(PrimitiveType::Int)),
        body: Box::new(Type::Primitive(PrimitiveType::Bool)),
    };
    let result = checker.finish(&ty);
    assert!(result.features.contains(&"dep".to_string()));
}

#[test]
fn test_checker_finish_with_errors_is_invalid() {
    let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
    checker.errors.push(TypeError::Custom {
        span: Span::synthetic(),
        message: "test error".to_string(),
        hint: None,
    });
    let ty = Type::Primitive(PrimitiveType::Int);
    let result = checker.finish(&ty);
    assert!(!result.valid);
    assert_eq!(result.type_signature, "<error>");
}

#[test]
fn test_check_result_err_concatenates_messages() {
    let errors = vec![
        TypeError::Custom {
            span: Span::synthetic(),
            message: "error one".to_string(),
            hint: None,
        },
        TypeError::Custom {
            span: Span::synthetic(),
            message: "error two".to_string(),
            hint: None,
        },
    ];
    let result = CheckResult::err(&errors);
    assert!(!result.valid);
    assert!(result.explanation.contains("error one"));
    assert!(result.explanation.contains("error two"));
}

// ============================================================================
// Linear usage tracker — additional tests
// ============================================================================

#[test]
fn test_tracker_bounded_exact_usage_ok() {
    let mut tracker = UsageTracker::new();
    tracker.declare("x".to_string(), UsageQuantifier::Bounded(3));
    assert!(tracker.record_use("x").is_none());
    assert!(tracker.record_use("x").is_none());
    assert!(tracker.record_use("x").is_none());
    // 4th use should fail.
    assert!(tracker.record_use("x").is_some());
}

#[test]
fn test_tracker_untracked_variable_no_violation() {
    let mut tracker = UsageTracker::new();
    // Using a variable that was never declared should not cause a violation.
    assert!(tracker.record_use("unknown").is_none());
}

#[test]
fn test_tracker_use_count() {
    let mut tracker = UsageTracker::new();
    tracker.declare("x".to_string(), UsageQuantifier::Omega);
    assert_eq!(tracker.use_count("x"), Some(0));
    tracker.record_use("x");
    assert_eq!(tracker.use_count("x"), Some(1));
    tracker.record_use("x");
    assert_eq!(tracker.use_count("x"), Some(2));
}

#[test]
fn test_tracker_use_count_unknown_returns_none() {
    let tracker = UsageTracker::new();
    assert_eq!(tracker.use_count("nonexistent"), None);
}

#[test]
fn test_affine_tracker_double_use_still_fails() {
    let mut tracker = UsageTracker::affine();
    tracker.declare("x".to_string(), UsageQuantifier::One);
    assert!(tracker.record_use("x").is_none()); // First: fine.
    assert!(tracker.record_use("x").is_some()); // Second: violation.
}

// ============================================================================
// Effects — additional tests
// ============================================================================

#[test]
fn test_effect_row_merge_preserves_open() {
    let a = EffectRow::open(vec![Effect::IO]);
    let b = EffectRow::closed(vec![Effect::Alloc]);
    let merged = a.merge(&b);
    assert!(merged.open);
}

#[test]
fn test_effect_row_merge_two_closed_is_closed() {
    let a = EffectRow::closed(vec![Effect::IO]);
    let b = EffectRow::closed(vec![Effect::Alloc]);
    let merged = a.merge(&b);
    assert!(!merged.open);
}

#[test]
fn test_effect_row_handle_all_effects_gives_pure() {
    let row = EffectRow::closed(vec![Effect::IO, Effect::Alloc]);
    let handled = row.handle(&[Effect::IO, Effect::Alloc]);
    assert!(handled.is_pure());
}

#[test]
fn test_check_effects_all_declared_ok() {
    let declared = vec![Effect::IO, Effect::Network, Effect::Alloc];
    let discovered = vec![Effect::IO, Effect::Alloc];
    assert!(check_effects(&declared, &discovered, Span::synthetic()).is_ok());
}

#[test]
fn test_check_effects_empty_discovered_ok() {
    let declared = vec![Effect::IO];
    let discovered: Vec<Effect> = vec![];
    assert!(check_effects(&declared, &discovered, Span::synthetic()).is_ok());
}

// ============================================================================
// QTT context — additional tests
// ============================================================================

#[test]
fn test_qtt_context_lookup_missing() {
    let ctx = QttContext::new();
    assert!(ctx.lookup("x").is_none());
}

#[test]
fn test_qtt_context_scale_by_one_preserves() {
    let mut ctx = QttContext::new();
    ctx.declare("x".to_string(), UsageQuantifier::Omega);
    let scaled = ctx.scale(&UsageQuantifier::One);
    assert_eq!(scaled.lookup("x"), Some(&UsageQuantifier::Omega));
}

#[test]
fn test_qtt_context_add_with_disjoint_vars() {
    let mut c1 = QttContext::new();
    c1.declare("x".to_string(), UsageQuantifier::One);
    let mut c2 = QttContext::new();
    c2.declare("y".to_string(), UsageQuantifier::One);
    let combined = c1.add(&c2);
    assert_eq!(combined.lookup("x"), Some(&UsageQuantifier::One));
    assert_eq!(combined.lookup("y"), Some(&UsageQuantifier::One));
}

#[test]
fn test_qtt_context_check_against_compatible() {
    let mut actual = QttContext::new();
    actual.declare("x".to_string(), UsageQuantifier::One);
    let mut declared = QttContext::new();
    declared.declare("x".to_string(), UsageQuantifier::Omega);
    let violations = actual.check_against(&declared);
    assert!(violations.is_empty());
}

#[test]
fn test_qtt_context_check_against_violation() {
    let mut actual = QttContext::new();
    actual.declare("x".to_string(), UsageQuantifier::Omega);
    let mut declared = QttContext::new();
    declared.declare("x".to_string(), UsageQuantifier::One);
    let violations = actual.check_against(&declared);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].0, "x");
}

// ============================================================================
// Session types — additional tests
// ============================================================================

#[test]
fn test_dual_of_end_is_end() {
    assert_eq!(dual(&SessionType::End), SessionType::End);
}

#[test]
fn test_dual_offer_becomes_select() {
    let offer = SessionType::Offer(vec![
        ("a".to_string(), SessionType::End),
        ("b".to_string(), SessionType::End),
    ]);
    let d = dual(&offer);
    assert!(matches!(d, SessionType::Select(_)));
}

#[test]
fn test_dual_select_becomes_offer() {
    let select = SessionType::Select(vec![("a".to_string(), SessionType::End)]);
    let d = dual(&select);
    assert!(matches!(d, SessionType::Offer(_)));
}

#[test]
fn test_dual_recursive_preserves_structure() {
    let rec = SessionType::Rec(
        "loop".to_string(),
        Box::new(SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::RecVar("loop".to_string())),
        )),
    );
    let d = dual(&rec);
    match d {
        SessionType::Rec(name, body) => {
            assert_eq!(name, "loop");
            assert!(matches!(*body, SessionType::Recv(_, _)));
        }
        other => panic!("expected Rec, got {:?}", other),
    }
}

#[test]
fn test_are_dual_complex_protocol() {
    let buyer = SessionType::Send(
        Box::new(Type::Named {
            name: "Item".to_string(),
            args: vec![],
        }),
        Box::new(SessionType::Recv(
            Box::new(Type::Named {
                name: "Price".to_string(),
                args: vec![],
            }),
            Box::new(SessionType::Select(vec![
                (
                    "accept".to_string(),
                    SessionType::Send(
                        Box::new(Type::Named {
                            name: "Payment".to_string(),
                            args: vec![],
                        }),
                        Box::new(SessionType::End),
                    ),
                ),
                ("reject".to_string(), SessionType::End),
            ])),
        )),
    );
    let seller = dual(&buyer);
    assert!(are_dual(&buyer, &seller));
    // Double dual should give back the original.
    assert_eq!(dual(&dual(&buyer)), buyer);
}

#[test]
fn test_well_formed_nested_rec() {
    let s = SessionType::Rec(
        "outer".to_string(),
        Box::new(SessionType::Send(
            Box::new(Type::Primitive(PrimitiveType::Int)),
            Box::new(SessionType::Rec(
                "inner".to_string(),
                Box::new(SessionType::Recv(
                    Box::new(Type::Primitive(PrimitiveType::Bool)),
                    Box::new(SessionType::RecVar("outer".to_string())),
                )),
            )),
        )),
    );
    assert!(is_well_formed(&s));
}

#[test]
fn test_well_formed_select_with_distinct_labels() {
    let s = SessionType::Select(vec![
        ("opt_a".to_string(), SessionType::End),
        ("opt_b".to_string(), SessionType::End),
    ]);
    assert!(is_well_formed(&s));
}

#[test]
fn test_not_well_formed_select_duplicate_labels() {
    let s = SessionType::Select(vec![
        ("dup".to_string(), SessionType::End),
        ("dup".to_string(), SessionType::End),
    ]);
    assert!(!is_well_formed(&s));
}

// ============================================================================
// Dimensional analysis — additional tests
// ============================================================================

#[test]
fn test_sub_same_dimension_ok() {
    let energy = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::energy(),
    };
    assert!(check_binary_op(DimOp::Sub, &energy, &energy, Span::synthetic()).is_ok());
}

#[test]
fn test_sub_different_dimension_fails() {
    let energy = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::energy(),
    };
    let time = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::time(),
    };
    assert!(check_binary_op(DimOp::Sub, &energy, &time, Span::synthetic()).is_err());
}

#[test]
fn test_compare_same_dimension_gives_bool() {
    let velocity = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::velocity(),
    };
    let result =
        check_binary_op(DimOp::Compare, &velocity, &velocity, Span::synthetic()).unwrap();
    assert_eq!(result, Type::Primitive(PrimitiveType::Bool));
}

#[test]
fn test_compare_different_dimension_fails() {
    let energy = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::energy(),
    };
    let time = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::time(),
    };
    assert!(check_binary_op(DimOp::Compare, &energy, &time, Span::synthetic()).is_err());
}

#[test]
fn test_resource_div_by_scalar() {
    let energy = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::energy(),
    };
    let scalar = Type::Primitive(PrimitiveType::Float);
    let result = check_binary_op(DimOp::Div, &energy, &scalar, Span::synthetic()).unwrap();
    match result {
        Type::Resource { dimension, .. } => assert_eq!(dimension, Dimension::energy()),
        other => panic!("expected Resource, got {:?}", other),
    }
}

#[test]
fn test_scalar_times_resource() {
    let scalar = Type::Primitive(PrimitiveType::Int);
    let time = Type::Resource {
        base: Box::new(Type::Primitive(PrimitiveType::Float)),
        dimension: Dimension::time(),
    };
    let result = check_binary_op(DimOp::Mul, &scalar, &time, Span::synthetic()).unwrap();
    match result {
        Type::Resource { dimension, .. } => assert_eq!(dimension, Dimension::time()),
        other => panic!("expected Resource, got {:?}", other),
    }
}

#[test]
fn test_resource_name_to_dimension_all_known() {
    let known = vec![
        ("energy", Dimension::energy()),
        ("Energy", Dimension::energy()),
        ("time", Dimension::time()),
        ("Time", Dimension::time()),
        ("latency", Dimension::time()),
        ("memory", Dimension::memory()),
        ("Memory", Dimension::memory()),
        ("carbon", Dimension::carbon()),
        ("power", Dimension::power()),
        ("force", Dimension::force()),
        ("velocity", Dimension::velocity()),
        ("money", Dimension::money()),
        ("currency", Dimension::money()),
    ];
    for (name, expected) in known {
        assert_eq!(
            resource_name_to_dimension(name),
            Some(expected),
            "failed for name: {}",
            name
        );
    }
}

#[test]
fn test_dimension_inverse() {
    let time = Dimension::time();
    let inv = time.inverse();
    assert_eq!(inv.time, -1);
    assert_eq!(inv.mass, 0);
    // time * time^-1 = dimensionless.
    let product = time.multiply(&inv);
    assert!(product.is_dimensionless());
}

// ============================================================================
// Proof obligations — additional tests
// ============================================================================

#[test]
fn test_obligation_ids_are_sequential() {
    let mut collector = ObligationCollector::new();
    collector.add_refinement(
        Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0)),
        "Int",
    );
    collector.add_refinement(
        Predicate::Lt(Term::Var("y".to_string()), Term::Lit(10)),
        "Int",
    );
    let all = collector.all();
    assert_eq!(all[0].id, "PO-0000");
    assert_eq!(all[1].id, "PO-0001");
}

#[test]
fn test_discharge_nonexistent_returns_false() {
    let mut collector = ObligationCollector::new();
    assert!(!collector.discharge("PO-9999"));
}

#[test]
fn test_try_discharge_refutes_false_predicate() {
    let mut collector = ObligationCollector::new();
    // 3 > 10 is false.
    collector.add_refinement(Predicate::Gt(Term::Lit(3), Term::Lit(10)), "Int");
    collector.try_discharge_refinements();
    assert!(matches!(
        collector.all()[0].status,
        ObligationStatus::Refuted(_)
    ));
}

#[test]
fn test_summaries_reflect_all_obligations() {
    let mut collector = ObligationCollector::new();
    collector.add_refinement(
        Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0)),
        "Int",
    );
    collector.add_session_completeness("MyProtocol");
    collector.add_dimension_check("add", "energy", "energy");
    let summaries = collector.summaries();
    assert_eq!(summaries.len(), 3);
}

// ============================================================================
// Predicate evaluation — additional tests
// ============================================================================

#[test]
fn test_eval_predicate_gte() {
    assert_eq!(
        eval_predicate(&Predicate::Gte(Term::Lit(5), Term::Lit(5))),
        PredicateResult::True
    );
    assert_eq!(
        eval_predicate(&Predicate::Gte(Term::Lit(4), Term::Lit(5))),
        PredicateResult::False
    );
}

#[test]
fn test_eval_predicate_lte() {
    assert_eq!(
        eval_predicate(&Predicate::Lte(Term::Lit(3), Term::Lit(5))),
        PredicateResult::True
    );
    assert_eq!(
        eval_predicate(&Predicate::Lte(Term::Lit(6), Term::Lit(5))),
        PredicateResult::False
    );
}

#[test]
fn test_eval_predicate_eq() {
    assert_eq!(
        eval_predicate(&Predicate::Eq(Term::Lit(7), Term::Lit(7))),
        PredicateResult::True
    );
    assert_eq!(
        eval_predicate(&Predicate::Eq(Term::Lit(7), Term::Lit(8))),
        PredicateResult::False
    );
}

#[test]
fn test_eval_predicate_neq() {
    assert_eq!(
        eval_predicate(&Predicate::Neq(Term::Lit(7), Term::Lit(8))),
        PredicateResult::True
    );
    assert_eq!(
        eval_predicate(&Predicate::Neq(Term::Lit(7), Term::Lit(7))),
        PredicateResult::False
    );
}

#[test]
fn test_eval_predicate_or_both_false() {
    let pred = Predicate::Or(
        Box::new(Predicate::Gt(Term::Lit(1), Term::Lit(10))),
        Box::new(Predicate::Gt(Term::Lit(2), Term::Lit(10))),
    );
    assert_eq!(eval_predicate(&pred), PredicateResult::False);
}

#[test]
fn test_eval_predicate_raw_is_unknown() {
    let pred = Predicate::Raw("some constraint".to_string());
    assert_eq!(eval_predicate(&pred), PredicateResult::Unknown);
}

#[test]
fn test_eval_predicate_double_not() {
    // !!true = true
    let pred = Predicate::Not(Box::new(Predicate::Not(Box::new(Predicate::Gt(
        Term::Lit(5),
        Term::Lit(0),
    )))));
    assert_eq!(eval_predicate(&pred), PredicateResult::True);
}

// ============================================================================
// Term evaluation — additional tests
// ============================================================================

#[test]
fn test_eval_term_subtraction() {
    let term = Term::BinOp {
        op: TermOp::Sub,
        lhs: Box::new(Term::Lit(10)),
        rhs: Box::new(Term::Lit(3)),
    };
    assert_eq!(eval_term_to_i64(&term), Some(7));
}

#[test]
fn test_eval_term_modulo() {
    let term = Term::BinOp {
        op: TermOp::Mod,
        lhs: Box::new(Term::Lit(10)),
        rhs: Box::new(Term::Lit(3)),
    };
    assert_eq!(eval_term_to_i64(&term), Some(1));
}

#[test]
fn test_eval_term_modulo_by_zero() {
    let term = Term::BinOp {
        op: TermOp::Mod,
        lhs: Box::new(Term::Lit(10)),
        rhs: Box::new(Term::Lit(0)),
    };
    assert_eq!(eval_term_to_i64(&term), None);
}

#[test]
fn test_eval_term_nested_arithmetic() {
    // (2 + 3) * (4 - 1) = 5 * 3 = 15
    let term = Term::BinOp {
        op: TermOp::Mul,
        lhs: Box::new(Term::BinOp {
            op: TermOp::Add,
            lhs: Box::new(Term::Lit(2)),
            rhs: Box::new(Term::Lit(3)),
        }),
        rhs: Box::new(Term::BinOp {
            op: TermOp::Sub,
            lhs: Box::new(Term::Lit(4)),
            rhs: Box::new(Term::Lit(1)),
        }),
    };
    assert_eq!(eval_term_to_i64(&term), Some(15));
}

#[test]
fn test_eval_term_app_returns_none() {
    let term = Term::App {
        func: "length".to_string(),
        args: vec![Term::Var("xs".to_string())],
    };
    assert_eq!(eval_term_to_i64(&term), None);
}

#[test]
fn test_terms_unify_app_same_func() {
    let t1 = Term::App {
        func: "f".to_string(),
        args: vec![Term::Var("x".to_string())],
    };
    let t2 = Term::App {
        func: "f".to_string(),
        args: vec![Term::Lit(42)],
    };
    assert!(terms_unify(&t1, &t2));
}

#[test]
fn test_terms_unify_app_different_func() {
    let t1 = Term::App {
        func: "f".to_string(),
        args: vec![Term::Lit(1)],
    };
    let t2 = Term::App {
        func: "g".to_string(),
        args: vec![Term::Lit(1)],
    };
    assert!(!terms_unify(&t1, &t2));
}

#[test]
fn test_terms_unify_app_different_arity() {
    let t1 = Term::App {
        func: "f".to_string(),
        args: vec![Term::Lit(1)],
    };
    let t2 = Term::App {
        func: "f".to_string(),
        args: vec![Term::Lit(1), Term::Lit(2)],
    };
    assert!(!terms_unify(&t1, &t2));
}

// ============================================================================
// Error types — span and hint extraction
// ============================================================================

#[test]
fn test_error_span_extraction() {
    let err = TypeError::Mismatch {
        span: Span::new(10, 20),
        expected: Type::Primitive(PrimitiveType::Int),
        found: Type::Primitive(PrimitiveType::Bool),
        hint: None,
    };
    let span = err.span();
    assert_eq!(span.start, 10);
    assert_eq!(span.end, 20);
}

#[test]
fn test_error_hint_extraction() {
    let err = TypeError::Mismatch {
        span: Span::synthetic(),
        expected: Type::Primitive(PrimitiveType::Int),
        found: Type::Primitive(PrimitiveType::Bool),
        hint: Some("try casting".to_string()),
    };
    assert_eq!(err.hint(), Some("try casting"));
}

#[test]
fn test_error_no_hint() {
    let err = TypeError::InfiniteType {
        span: Span::synthetic(),
        var: TypeVar(0),
        ty: Type::Primitive(PrimitiveType::Int),
    };
    assert!(err.hint().is_none());
}

#[test]
fn test_span_point() {
    let span = Span::point(42);
    assert_eq!(span.start, 42);
    assert_eq!(span.end, 42);
}

// ============================================================================
// UsageQuantifier — additional compatibility and addition tests
// ============================================================================

#[test]
fn test_usage_quantifier_zero_compatible_with_all() {
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::Zero));
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::One));
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::Omega));
    assert!(UsageQuantifier::Zero.compatible_with(&UsageQuantifier::Bounded(5)));
}

#[test]
fn test_usage_quantifier_bounded_addition() {
    let result = UsageQuantifier::Bounded(3).add(&UsageQuantifier::Bounded(4));
    assert_eq!(result, UsageQuantifier::Bounded(7));
}

#[test]
fn test_usage_quantifier_one_plus_bounded() {
    let result = UsageQuantifier::One.add(&UsageQuantifier::Bounded(5));
    assert_eq!(result, UsageQuantifier::Bounded(6));
}

#[test]
fn test_usage_quantifier_omega_plus_anything_is_omega() {
    assert_eq!(
        UsageQuantifier::Omega.add(&UsageQuantifier::One),
        UsageQuantifier::Omega
    );
    assert_eq!(
        UsageQuantifier::Omega.add(&UsageQuantifier::Bounded(10)),
        UsageQuantifier::Omega
    );
}

// ============================================================================
// TypeDiscipline — display
// ============================================================================

#[test]
fn test_type_discipline_display() {
    assert_eq!(TypeDiscipline::Linear.to_string(), "linear");
    assert_eq!(TypeDiscipline::Affine.to_string(), "affine");
    assert_eq!(TypeDiscipline::Dependent.to_string(), "dependent");
    assert_eq!(TypeDiscipline::Refined.to_string(), "refined");
    assert_eq!(TypeDiscipline::Unrestricted.to_string(), "unrestricted");
}

#[test]
fn test_type_discipline_default_is_affine() {
    assert_eq!(TypeDiscipline::default(), TypeDiscipline::Affine);
}

// ============================================================================
// Serialisation round-trips
// ============================================================================

#[test]
fn test_type_serialise_round_trip() {
    let types = vec![
        Type::Primitive(PrimitiveType::Int),
        Type::Var(TypeVar(42)),
        Type::Named {
            name: "Vec".to_string(),
            args: vec![Type::Primitive(PrimitiveType::String)],
        },
        Type::Tuple(vec![
            Type::Primitive(PrimitiveType::Bool),
            Type::Primitive(PrimitiveType::Float),
        ]),
        Type::Top,
        Type::Bottom,
        Type::Error,
    ];
    for ty in types {
        let json = serde_json::to_string(&ty).expect("serialise");
        let recovered: Type = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(ty, recovered);
    }
}

#[test]
fn test_unified_type_serialise_round_trip() {
    let ut = UnifiedType {
        base: Type::Primitive(PrimitiveType::Int),
        usage: UsageQuantifier::Bounded(5),
        discipline: TypeDiscipline::Linear,
        dependent_indices: vec![Term::Lit(10)],
        effects: vec![Effect::IO],
        refinements: vec![Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0))],
    };
    let json = serde_json::to_string(&ut).expect("serialise");
    let recovered: UnifiedType = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(ut, recovered);
}

#[test]
fn test_check_result_serialise_round_trip() {
    let result = CheckResult::ok(&Type::Primitive(PrimitiveType::Bool), TypeDiscipline::Affine);
    let json = serde_json::to_string(&result).expect("serialise");
    let recovered: CheckResult = serde_json::from_str(&json).expect("deserialise");
    assert_eq!(recovered.valid, result.valid);
    assert_eq!(recovered.type_signature, result.type_signature);
}
