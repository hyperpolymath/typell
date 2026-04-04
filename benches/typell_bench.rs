// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL core benchmarks — unification, substitution, and usage quantifier algebra.
//!
//! Covers the hot paths in the TypeLL verification kernel:
//! - `Substitution::apply` on increasingly deep type trees
//! - `Substitution::unify` for common type pairs (primitive, function, poly)
//! - `UsageQuantifier::add` / `compatible_with` semiring operations
//! - `UsageQuantifier::compatible_with` for all QTT lattice pairs

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use typell_core::types::{PrimitiveType, Type, TypeVar, UsageQuantifier};
use typell_core::{Span, Substitution, Unifier};

// ============================================================================
// Helpers — construct representative type trees
// ============================================================================

/// A simple function type: (Int, Bool) -> Float
fn make_function_type() -> Type {
    Type::Function {
        params: vec![
            Type::Primitive(PrimitiveType::Int),
            Type::Primitive(PrimitiveType::Bool),
        ],
        ret: Box::new(Type::Primitive(PrimitiveType::F64)),
        effects: vec![],
    }
}

/// A nested named type: List<Option<Int>>
fn make_nested_named_type() -> Type {
    Type::Named {
        name: "List".to_string(),
        args: vec![Type::Named {
            name: "Option".to_string(),
            args: vec![Type::Primitive(PrimitiveType::Int)],
        }],
    }
}

/// A type containing a free type variable.
fn make_var_type(id: u32) -> Type {
    Type::Var(TypeVar(id))
}

/// Build a substitution that maps N variables to primitive types.
fn build_substitution(n: u32) -> Substitution {
    let mut sub = Substitution::new();
    for i in 0..n {
        sub.bind(TypeVar(i), Type::Primitive(PrimitiveType::Int));
    }
    sub
}

/// Build a chain substitution: var(0) -> var(1) -> ... -> var(n-1) -> Int
fn build_chain_substitution(depth: u32) -> Substitution {
    let mut sub = Substitution::new();
    for i in 0..(depth - 1) {
        sub.bind(TypeVar(i), Type::Var(TypeVar(i + 1)));
    }
    sub.bind(TypeVar(depth - 1), Type::Primitive(PrimitiveType::Int));
    sub
}

// ============================================================================
// Substitution benchmarks
// ============================================================================

/// Benchmark applying a substitution to a primitive type (no-op fast path).
fn bench_subst_primitive(c: &mut Criterion) {
    let sub = build_substitution(10);
    let ty = Type::Primitive(PrimitiveType::Int);

    c.bench_function("subst_apply_primitive", |b| {
        b.iter(|| black_box(sub.apply(black_box(&ty))))
    });
}

/// Benchmark applying a substitution to a function type with free variables.
fn bench_subst_function(c: &mut Criterion) {
    let sub = build_substitution(16);
    // Function type with variable arguments that will be substituted.
    let ty = Type::Function {
        params: vec![
            Type::Var(TypeVar(0)),
            Type::Var(TypeVar(1)),
            Type::Var(TypeVar(2)),
        ],
        ret: Box::new(Type::Var(TypeVar(3))),
        effects: vec![],
    };

    c.bench_function("subst_apply_function_vars", |b| {
        b.iter(|| black_box(sub.apply(black_box(&ty))))
    });
}

/// Benchmark following a chain of variable bindings (worst-case path following).
fn bench_subst_chain(c: &mut Criterion) {
    let mut group = c.benchmark_group("subst_chain_depth");
    for depth in [2u32, 4, 8, 16, 32] {
        let sub = build_chain_substitution(depth);
        let ty = Type::Var(TypeVar(0));
        group.bench_with_input(BenchmarkId::from_parameter(depth), &depth, |b, _| {
            b.iter(|| black_box(sub.apply(black_box(&ty))))
        });
    }
    group.finish();
}

/// Benchmark applying a substitution to a nested named type.
fn bench_subst_nested(c: &mut Criterion) {
    let mut sub = Substitution::new();
    sub.bind(TypeVar(0), Type::Primitive(PrimitiveType::I64));
    let ty = make_nested_named_type();

    c.bench_function("subst_apply_nested_named", |b| {
        b.iter(|| black_box(sub.apply(black_box(&ty))))
    });
}

// ============================================================================
// Unification benchmarks
// ============================================================================

/// Benchmark unifying two identical primitive types (trivial success).
fn bench_unify_primitives(c: &mut Criterion) {
    let span = Span { start: 0, end: 0 };
    c.bench_function("unify_primitives_same", |b| {
        b.iter(|| {
            let mut ctx = Unifier::new();
            black_box(ctx.unify(
                black_box(&Type::Primitive(PrimitiveType::Int)),
                black_box(&Type::Primitive(PrimitiveType::Int)),
                span,
            ))
        })
    });
}

/// Benchmark unifying a type variable with a concrete type.
fn bench_unify_var_concrete(c: &mut Criterion) {
    let span = Span { start: 0, end: 0 };
    let concrete = make_function_type();

    c.bench_function("unify_var_concrete", |b| {
        b.iter(|| {
            let mut ctx = Unifier::new();
            black_box(ctx.unify(
                black_box(&Type::Var(TypeVar(0))),
                black_box(&concrete),
                span,
            ))
        })
    });
}

/// Benchmark unifying two function types.
fn bench_unify_functions(c: &mut Criterion) {
    let span = Span { start: 0, end: 0 };
    let f1 = make_function_type();
    let f2 = make_function_type();

    c.bench_function("unify_function_types", |b| {
        b.iter(|| {
            let mut ctx = Unifier::new();
            black_box(ctx.unify(black_box(&f1), black_box(&f2), span))
        })
    });
}

// ============================================================================
// UsageQuantifier (QTT semiring) benchmarks
// ============================================================================

/// Benchmark QTT semiring addition for all lattice combinations.
fn bench_qtt_add(c: &mut Criterion) {
    use UsageQuantifier::*;
    let pairs = [
        (Zero, Zero),
        (Zero, One),
        (One, One),
        (One, Omega),
        (Omega, Omega),
        (Bounded(3), Bounded(5)),
        (Bounded(10), Omega),
    ];

    c.bench_function("qtt_add_all_pairs", |b| {
        b.iter(|| {
            for (a, b_val) in &pairs {
                black_box(a.add(black_box(b_val)));
            }
        })
    });
}

/// Benchmark QTT compatibility check across the lattice.
fn bench_qtt_compatible(c: &mut Criterion) {
    use UsageQuantifier::*;
    let pairs = [
        (Zero, Omega),
        (One, One),
        (One, Omega),
        (Omega, Omega),
        (Bounded(3), Bounded(10)),
        (Bounded(10), Bounded(3)),
    ];

    c.bench_function("qtt_compatible_with_all", |b| {
        b.iter(|| {
            for (a, b_val) in &pairs {
                black_box(a.compatible_with(black_box(b_val)));
            }
        })
    });
}

// ============================================================================
// Substitution lookup benchmarks
// ============================================================================

/// Benchmark lookup in substitutions of varying sizes.
fn bench_subst_lookup_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("subst_lookup_size");
    for n in [4u32, 16, 64, 256] {
        let sub = build_substitution(n);
        let var = TypeVar(n / 2); // Lookup a middle variable.
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, _| {
            b.iter(|| black_box(sub.lookup(black_box(var))))
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_subst_primitive,
    bench_subst_function,
    bench_subst_chain,
    bench_subst_nested,
    bench_unify_primitives,
    bench_unify_var_concrete,
    bench_unify_functions,
    bench_qtt_add,
    bench_qtt_compatible,
    bench_subst_lookup_scaling,
);
criterion_main!(benches);
