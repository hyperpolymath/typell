// SPDX-License-Identifier: MPL-2.0
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell
//
// Kategoria conformance suite.
//
// Kategoria (https://github.com/hyperpolymath/kategoria) is the Type
// Safety Challenge: 10 LANGUAGE-FEATURE levels, each demonstrated by a
// verified Idris 2 module (routes/alpha-extend/Level01..Level10, all
// pass `idris2 --check` with no postulates). This suite feeds those
// levels into typell as executable conformance checks: one section per
// kategoria level, exercising the typell-core mechanism that embodies
// it, with accept/reject pairs mirroring the Idris modules.
//
// NOTE the numbering collision: typell's own L1-L10 are SAFETY-PROPERTY
// levels (parse, schema, injection, temporal, linearity, ...). The
// mapping between the two hierarchies is enablement, not identity — see
// kategoria routes/delta-aggregate/TYPELL-BRIDGE.adoc and this repo's
// docs/KATEGORIA-BRIDGE.adoc.
//
// HONESTY RULE (kategoria discipline): absent capabilities are recorded
// as #[ignore]d tests that PANIC if force-run — never as silent passes.
//
//   kat-L1  BasicTypes      -> PrimitiveType                 [tested]
//   kat-L2  ADTs            -> Named/Tuple (nominal only)    [tested, partial]
//   kat-L3  Polymorphism    -> Var + generalize/instantiate  [tested]
//   kat-L4  TypeClasses     -> (absent)                      [ignored]
//   kat-L5  GADTs           -> (absent)                      [ignored]
//   kat-L6  DependentTypes  -> Pi/Sigma/Array lengths        [tested]
//   kat-L7  LinearTypes     -> QTT UsageTracker              [tested]
//   kat-L8  Refinement      -> Refined + eval_predicate      [tested]
//   kat-L9  SessionTypes    -> session duality (binary)      [tested]
//   kat-L10 CubicalTypes    -> (absent; route α's wall too)  [ignored]

use typell_core::error::Span;
use typell_core::infer::InferCtx;
use typell_core::linear::UsageTracker;
use typell_core::proof::{eval_predicate, PredicateResult};
use typell_core::session::{are_dual, dual, is_well_formed};
use typell_core::types::{
    Predicate, PrimitiveType, SessionType, Term, Type, TypeVar, UsageQuantifier,
};
use typell_core::unify::{Substitution, Unifier};

fn int() -> Type {
    Type::Primitive(PrimitiveType::Int)
}

fn boolean() -> Type {
    Type::Primitive(PrimitiveType::Bool)
}

// ============================================================================
// kat-L1: Basic Types (Level01_BasicTypes.idr)
// ============================================================================

#[test]
fn kat_l01_accept_primitive_types_unify_with_themselves() {
    let mut u = Unifier::new();
    assert!(u.unify(&int(), &int(), Span::synthetic()).is_ok());
    assert!(u.unify(&boolean(), &boolean(), Span::synthetic()).is_ok());
}

#[test]
fn kat_l01_reject_int_is_not_bool() {
    let mut u = Unifier::new();
    assert!(u.unify(&int(), &boolean(), Span::synthetic()).is_err());
}

// ============================================================================
// kat-L2: Algebraic Data Types (Level02_ADTs.idr)
//
// PARTIAL: typell-core has nominal constructors (Named) and products
// (Tuple), but no sum-type declarations and no exhaustiveness checking.
// ============================================================================

#[test]
fn kat_l02_accept_nominal_constructor_unifies() {
    let mut u = Unifier::new();
    let t1 = Type::Named { name: "Option".to_string(), args: vec![int()] };
    let t2 = Type::Named { name: "Option".to_string(), args: vec![int()] };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
}

#[test]
fn kat_l02_reject_different_constructors() {
    let mut u = Unifier::new();
    let t1 = Type::Named { name: "Option".to_string(), args: vec![int()] };
    let t2 = Type::Named { name: "Result".to_string(), args: vec![int()] };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_err());
}

#[test]
fn kat_l02_reject_product_arity_mismatch() {
    let mut u = Unifier::new();
    let t1 = Type::Tuple(vec![int(), boolean()]);
    let t2 = Type::Tuple(vec![int()]);
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_err());
}

// ============================================================================
// kat-L3: Parametric Polymorphism (Level03_Polymorphism.idr)
// ============================================================================

#[test]
fn kat_l03_accept_type_variable_binds() {
    let mut u = Unifier::new();
    let var = Type::Var(TypeVar(0));
    assert!(u.unify(&var, &boolean(), Span::synthetic()).is_ok());
    assert_eq!(u.substitution.apply(&Type::Var(TypeVar(0))), boolean());
}

#[test]
fn kat_l03_accept_generalize_then_instantiate_is_polymorphic() {
    let mut ctx = InferCtx::new();
    let fresh = ctx.fresh_var();
    let subst = Substitution::new();
    let scheme = ctx.generalize(&fresh, &subst);
    assert!(
        !scheme.vars.is_empty(),
        "a free inference variable must be generalized (let-polymorphism)"
    );
    let inst = ctx.instantiate(&scheme);
    assert!(inst.is_var(), "instantiation must produce a fresh variable");
    assert_ne!(inst, fresh, "instantiation must not reuse the generalized variable");
}

#[test]
fn kat_l03_reject_occurs_check() {
    // A variable cannot unify with a type containing itself (infinite type).
    let mut u = Unifier::new();
    let var = Type::Var(TypeVar(0));
    let recursive = Type::Function {
        params: vec![Type::Var(TypeVar(0))],
        ret: Box::new(int()),
        effects: vec![],
    };
    assert!(u.unify(&var, &recursive, Span::synthetic()).is_err());
}

// ============================================================================
// kat-L4: Type Classes (Level04_TypeClasses.idr)
// ============================================================================

#[test]
#[ignore = "kategoria L4: typell-core has no type-class/constraint mechanism (measured 2026-07-21; see docs/KATEGORIA-BRIDGE.adoc)"]
fn kat_l04_type_classes_absent() {
    panic!("typell-core has no type-class mechanism — this test documents the gap and must fail if force-run");
}

// ============================================================================
// kat-L5: GADTs (Level05_GADTs.idr)
// ============================================================================

#[test]
#[ignore = "kategoria L5: typell-core has no indexed-constructor (GADT) refinement (measured 2026-07-21; see docs/KATEGORIA-BRIDGE.adoc)"]
fn kat_l05_gadts_absent() {
    panic!("typell-core has no GADT mechanism — this test documents the gap and must fail if force-run");
}

// ============================================================================
// kat-L6: Dependent Types (Level06_DependentTypes.idr)
// ============================================================================

#[test]
fn kat_l06_accept_pi_types_unify() {
    let mut u = Unifier::new();
    let t1 = Type::Pi {
        param_name: "n".to_string(),
        param_type: Box::new(int()),
        body: Box::new(Type::Var(TypeVar(0))),
    };
    let t2 = Type::Pi {
        param_name: "m".to_string(),
        param_type: Box::new(int()),
        body: Box::new(boolean()),
    };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
    assert_eq!(u.substitution.apply(&Type::Var(TypeVar(0))), boolean());
}

#[test]
fn kat_l06_accept_sigma_types_unify() {
    let mut u = Unifier::new();
    let sigma = |snd: Type| Type::Sigma {
        fst_name: "x".to_string(),
        fst_type: Box::new(int()),
        snd_type: Box::new(snd),
    };
    assert!(u.unify(&sigma(boolean()), &sigma(boolean()), Span::synthetic()).is_ok());
}

#[test]
fn kat_l06_accept_dependent_array_length_matches() {
    // Mirrors kategoria's Vect n a: Array Int 3 unifies with Array Int 3.
    let mut u = Unifier::new();
    let arr = |len: i64| Type::Array {
        elem: Box::new(int()),
        length: Some(Term::Lit(len)),
    };
    assert!(u.unify(&arr(3), &arr(3), Span::synthetic()).is_ok());
}

#[test]
fn kat_l06_reject_dependent_array_length_mismatch() {
    // The REJECT case of value-indexed types: length 3 is not length 4.
    let mut u = Unifier::new();
    let arr = |len: i64| Type::Array {
        elem: Box::new(int()),
        length: Some(Term::Lit(len)),
    };
    assert!(u.unify(&arr(3), &arr(4), Span::synthetic()).is_err());
}

// ============================================================================
// kat-L7: Linear Types (Level07_LinearTypes.idr — native QTT in Idris 2)
// ============================================================================

#[test]
fn kat_l07_accept_linear_used_exactly_once() {
    let mut tracker = UsageTracker::new();
    tracker.declare("resource".to_string(), UsageQuantifier::One);
    assert!(tracker.record_use("resource").is_none());
    assert!(tracker.check_all_consumed().is_empty());
}

#[test]
fn kat_l07_reject_linear_used_twice() {
    let mut tracker = UsageTracker::new();
    tracker.declare("resource".to_string(), UsageQuantifier::One);
    assert!(tracker.record_use("resource").is_none());
    assert!(
        tracker.record_use("resource").is_some(),
        "second use of a linear resource must be a violation"
    );
}

#[test]
fn kat_l07_reject_linear_never_used() {
    let mut tracker = UsageTracker::new();
    tracker.declare("resource".to_string(), UsageQuantifier::One);
    assert!(
        !tracker.check_all_consumed().is_empty(),
        "an unconsumed linear resource must be reported at scope end"
    );
}

#[test]
fn kat_l07_accept_erased_witness_zero_uses() {
    // QTT quantity 0: proof-only witness, may not be used at runtime.
    let mut tracker = UsageTracker::new();
    tracker.declare("witness".to_string(), UsageQuantifier::Zero);
    assert!(
        tracker.record_use("witness").is_some(),
        "using an erased (quantity-0) witness must be a violation"
    );
}

// ============================================================================
// kat-L8: Refinement Types (Level08_Refinement.idr — dependent-pair encoding)
// ============================================================================

#[test]
fn kat_l08_accept_satisfied_predicate() {
    // Mirrors mkPositive 42: the proof 42 >= 1 is found.
    assert_eq!(
        eval_predicate(&Predicate::Gte(Term::Lit(42), Term::Lit(1))),
        PredicateResult::True
    );
}

#[test]
fn kat_l08_reject_violated_predicate() {
    // Mirrors the REJECT case: 0 is not positive.
    assert_eq!(
        eval_predicate(&Predicate::Gte(Term::Lit(0), Term::Lit(1))),
        PredicateResult::False
    );
}

#[test]
fn kat_l08_unknown_predicate_is_not_silently_true() {
    // The decision procedure must admit ignorance, not fake a pass —
    // the same honesty rule kategoria's proof gate enforces.
    assert_eq!(
        eval_predicate(&Predicate::Raw("nontrivial SMT goal".to_string())),
        PredicateResult::Unknown
    );
}

#[test]
fn kat_l08_accept_refined_types_unify_on_base() {
    let mut u = Unifier::new();
    let t1 = Type::Refined {
        base: Box::new(int()),
        predicates: vec![Predicate::Gt(Term::Var("x".to_string()), Term::Lit(0))],
    };
    let t2 = Type::Refined {
        base: Box::new(int()),
        predicates: vec![Predicate::Lt(Term::Var("x".to_string()), Term::Lit(100))],
    };
    assert!(u.unify(&t1, &t2, Span::synthetic()).is_ok());
}

// ============================================================================
// kat-L9: Session Types (Level09_SessionTypes.idr — Brady indexed monad)
// ============================================================================

#[test]
fn kat_l09_accept_duality() {
    // Send is dual to Recv — the core protocol-safety law.
    let client = SessionType::Send(Box::new(int()), Box::new(SessionType::End));
    let server = SessionType::Recv(Box::new(int()), Box::new(SessionType::End));
    assert_eq!(dual(&client), server);
    assert!(are_dual(&client, &server));
}

#[test]
fn kat_l09_accept_duality_is_an_involution() {
    let proto = SessionType::Send(
        Box::new(int()),
        Box::new(SessionType::Recv(Box::new(boolean()), Box::new(SessionType::End))),
    );
    assert_eq!(dual(&dual(&proto)), proto);
}

#[test]
fn kat_l09_reject_non_dual_protocols() {
    // Two senders cannot talk to each other.
    let s1 = SessionType::Send(Box::new(int()), Box::new(SessionType::End));
    let s2 = SessionType::Send(Box::new(int()), Box::new(SessionType::End));
    assert!(!are_dual(&s1, &s2));
}

#[test]
fn kat_l09_accept_well_formed_protocol() {
    let proto = SessionType::Rec(
        "loop".to_string(),
        Box::new(SessionType::Send(
            Box::new(int()),
            Box::new(SessionType::RecVar("loop".to_string())),
        )),
    );
    assert!(is_well_formed(&proto));
}

#[test]
fn kat_l09_reject_unbound_recursion_variable() {
    // A RecVar with no enclosing Rec binder is ill-formed.
    assert!(!is_well_formed(&SessionType::RecVar("ghost".to_string())));
}

// ============================================================================
// kat-L10: Cubical / Homotopy Types (Level10_CubicalTypes.idr)
//
// Kategoria route α documents L10 as its wall (QTT core ≠ cubical core).
// Typell has no path types, transport, or univalence either.
// ============================================================================

#[test]
#[ignore = "kategoria L10: typell-core has no cubical/path types (measured 2026-07-21) — route α hit the same wall; see Level10_CubicalTypes.idr"]
fn kat_l10_cubical_absent() {
    panic!("typell-core has no cubical mechanism — this test documents the gap and must fail if force-run");
}
