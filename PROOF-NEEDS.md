# PROOF-NEEDS.md
<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->

## Current State

- **LOC**: ~17,000
- **Languages**: Rust, Idris2, Zig
- **Existing ABI proofs**: `src/abi/*.idr` (template-level)
- **Dangerous patterns**: None detected in source files

## What Needs Proving

### Core Type System (crates/typell-core/)
- `check.rs` — type checking for the 10-level type system
- `infer.rs` — type inference
- `unify.rs` — unification algorithm
- `proof.rs` (495 lines) — proof generation/checking in Rust
- `linear.rs` — linear type tracking
- `qtt.rs` — quantitative type theory
- `session.rs` — session types
- `effects.rs` — effect system
- Prove: type checking is sound (well-typed programs do not get stuck)
- Prove: type inference is complete (finds principal types when they exist)
- Prove: unification terminates and is most general

### Language Bridges (14 crates)
- Each `typell-*` crate bridges TypeLL to a nextgen language
- Prove: bridge translations preserve typing judgments
- Prove: subtyping relationships are transitive across bridges

### Dimensional Types (crates/typell-core/src/dimensional.rs)
- Physical dimension tracking in types
- Prove: dimensional analysis is consistent (no unit mismatch can pass type checking)

## Recommended Prover

- **Idris2** for the type system metatheory (soundness, completeness)
- **Agda** alternative for the unification/inference proofs (strong equational reasoning)

## Priority

**HIGH** — TypeLL is PanLL's verification kernel. If the type system is unsound, every downstream language that relies on TypeLL levels has unsound guarantees. The 10-level type hierarchy is the central claim of the project and must be formally verified.
