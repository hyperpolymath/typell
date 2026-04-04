// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! Criterion benchmarks for the TypeLL unified type system.
//!
//! Measures throughput of the core type-checking operations:
//! - Type construction and equality checking
//! - Unification of type variables with concrete types
//! - Full type-checking pipeline (register → infer → finish)
//! - Generic and nested type expressions

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use typell_core::{
    TypeChecker, TypeDiscipline, UnifiedType,
    Type, PrimitiveType,
    UsageQuantifier,
    Span,
};

// ============================================================================
// Helpers
// ============================================================================

fn span() -> Span {
    Span::synthetic()
}

/// Build a simple primitive type.
fn prim(p: PrimitiveType) -> Type {
    Type::Primitive(p)
}

/// Build a function type from `params` to `ret` with no effects.
fn fn_type(params: Vec<Type>, ret: Type) -> Type {
    Type::Function {
        params,
        ret: Box::new(ret),
        effects: vec![],
    }
}

/// Build a deeply nested tuple type of depth `depth` over `Int`.
fn nested_tuple(depth: usize) -> Type {
    let mut ty = prim(PrimitiveType::Int);
    for _ in 0..depth {
        ty = Type::Tuple(vec![ty, prim(PrimitiveType::Bool)]);
    }
    ty
}

/// Build a `Named` generic type with `arity` `Int` arguments.
fn generic_named(arity: usize) -> Type {
    Type::Named {
        name: "Container".to_string(),
        args: (0..arity).map(|_| prim(PrimitiveType::Int)).collect(),
    }
}

// ============================================================================
// Benchmark group: type construction
// ============================================================================

fn bench_type_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_construction");

    group.bench_function("primitive_int", |b| {
        b.iter(|| black_box(prim(PrimitiveType::Int)))
    });

    group.bench_function("function_int_to_bool", |b| {
        b.iter(|| {
            black_box(fn_type(
                vec![prim(PrimitiveType::Int)],
                prim(PrimitiveType::Bool),
            ))
        })
    });

    for depth in [2, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("nested_tuple_depth", depth),
            &depth,
            |b, &d| b.iter(|| black_box(nested_tuple(d))),
        );
    }

    for arity in [1, 4, 8] {
        group.bench_with_input(
            BenchmarkId::new("generic_named_arity", arity),
            &arity,
            |b, &a| b.iter(|| black_box(generic_named(a))),
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark group: type equality checking
// ============================================================================

fn bench_type_equality(c: &mut Criterion) {
    let mut group = c.benchmark_group("type_equality");

    let int  = prim(PrimitiveType::Int);
    let bool_ty = prim(PrimitiveType::Bool);
    let complex = nested_tuple(6);

    group.bench_function("primitive_equal", |b| {
        b.iter(|| black_box(int == int))
    });

    group.bench_function("primitive_unequal", |b| {
        b.iter(|| black_box(int == bool_ty))
    });

    group.bench_function("nested_tuple_equal", |b| {
        let lhs = nested_tuple(6);
        b.iter(|| black_box(lhs == complex))
    });

    group.finish();
}

// ============================================================================
// Benchmark group: unification
// ============================================================================

fn bench_unification(c: &mut Criterion) {
    let mut group = c.benchmark_group("unification");

    group.bench_function("fresh_var_with_primitive", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
            let fresh = checker.fresh_var();
            let target = prim(PrimitiveType::Int);
            checker.unify(black_box(&fresh), black_box(&target), span())
                .expect("unification must succeed in benchmark");
            black_box(checker.apply(&fresh))
        })
    });

    group.bench_function("chain_of_5_vars", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
            let vars: Vec<Type> = (0..5).map(|_| checker.fresh_var()).collect();
            // Chain: v0 ~ v1 ~ v2 ~ v3 ~ v4 ~ Int
            let target = prim(PrimitiveType::I64);
            for i in 0..4 {
                checker.unify(&vars[i], &vars[i + 1], span())
                    .expect("chain unification must succeed");
            }
            checker.unify(&vars[4], &target, span())
                .expect("terminal unification must succeed");
            black_box(checker.apply(&vars[0]))
        })
    });

    group.bench_function("generic_type_unification", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
            let fresh = checker.fresh_var();
            let named = generic_named(4);
            checker.unify(black_box(&fresh), black_box(&named), span())
                .expect("named type unification must succeed");
            black_box(checker.apply(&fresh))
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark group: full type-checking pipeline
// ============================================================================

fn bench_full_pipeline(c: &mut Criterion) {
    let mut group = c.benchmark_group("full_pipeline");

    group.bench_function("base_type_register_infer_finish", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
            checker.register_binding("x", UnifiedType::simple(prim(PrimitiveType::Int)));
            let inferred = checker.infer_var(black_box("x"), span())
                .expect("infer must succeed");
            black_box(checker.finish(&inferred))
        })
    });

    group.bench_function("function_type_pipeline", |b| {
        let fn_ty = fn_type(
            vec![prim(PrimitiveType::Int), prim(PrimitiveType::String)],
            prim(PrimitiveType::Bool),
        );
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
            checker.register_binding("f", UnifiedType::simple(fn_ty.clone()));
            let inferred = checker.infer_var(black_box("f"), span())
                .expect("infer must succeed");
            black_box(checker.finish(&inferred))
        })
    });

    group.bench_function("nested_type_pipeline_depth_4", |b| {
        let nested = nested_tuple(4);
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Unrestricted);
            checker.register_binding("t", UnifiedType::simple(nested.clone()));
            let inferred = checker.infer_var(black_box("t"), span())
                .expect("infer must succeed");
            black_box(checker.finish(&inferred))
        })
    });

    group.bench_function("linear_pipeline", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new(TypeDiscipline::Linear);
            checker.register_binding("r", UnifiedType::linear(prim(PrimitiveType::U64)));
            let inferred = checker.infer_var(black_box("r"), span())
                .expect("infer must succeed");
            black_box(checker.finish(&inferred))
        })
    });

    group.finish();
}

// ============================================================================
// Benchmark group: usage quantifier arithmetic
// ============================================================================

fn bench_usage_quantifier(c: &mut Criterion) {
    let mut group = c.benchmark_group("usage_quantifier");

    group.bench_function("compatible_with_omega", |b| {
        let one = UsageQuantifier::One;
        let omega = UsageQuantifier::Omega;
        b.iter(|| black_box(one.compatible_with(black_box(&omega))))
    });

    group.bench_function("add_bounded_values", |b| {
        let a = UsageQuantifier::Bounded(3);
        let bb = UsageQuantifier::Bounded(7);
        b.iter(|| black_box(a.add(black_box(&bb))))
    });

    group.finish();
}

// ============================================================================
// Criterion entry point
// ============================================================================

criterion_group!(
    benches,
    bench_type_construction,
    bench_type_equality,
    bench_unification,
    bench_full_pipeline,
    bench_usage_quantifier,
);
criterion_main!(benches);
