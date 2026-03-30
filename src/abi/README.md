# SPDX-License-Identifier: PMPL-1.0-or-later

# src/abi/ — Idris2 ABI Definitions (Formal Specifications)

This directory contains the formal specifications for Typell's type systems,
written in Idris2 with dependent type proofs. These specs are the mathematical
foundation that proves Typell's type checker is sound, complete, and decidable.

## Existing Modules

```
abi/
├── Types.idr               # ABI type definitions, platform detection, memory proofs
├── Layout.idr              # Memory layout proofs, C ABI compliance, struct alignment
├── Foreign.idr             # FFI declarations for the Zig layer (libtypell)
├── Soundness.idr           # Progress + Preservation proofs for core calculus
├── InferenceSoundness.idr  # Unification + type inference soundness proofs
└── LevelMonotonicity.idr   # 10-level hierarchy monotonicity proofs (L1-L10)
```

## Soundness Proofs

### Soundness.idr — Progress and Preservation
Models the core STLC+let calculus underlying `check.rs` and `infer.rs`.
- **Progress**: well-typed closed terms are values or can step (fully proved)
- **Preservation**: stepping preserves types (proved modulo standard renaming lemma)
- **Type safety**: well-typed programs never get stuck (corollary)

### InferenceSoundness.idr — Type Inference
Models unification (`unify.rs`) and inference (`infer.rs`).
- **Unification soundness**: successful unification produces a valid unifier
- **Occurs check correctness**: variables absent from a type are unaffected by substitution
- **Substitution idempotence**: MGU applied twice equals MGU applied once
- **Arrow decomposition**: unifying function types decomposes to component unification

### LevelMonotonicity.idr — 10-Level Hierarchy
Models the L1-L10 type safety hierarchy from `ROADMAP.adoc`.
- **Level total order**: any two levels are comparable
- **Strict increase**: each level strictly subsumes the previous
- **Feature monotonicity**: higher levels include all lower-level features
- **No downgrade**: programs requiring level N features cannot be checked at level M < N
- **Lattice bounds**: L1 is bottom, L10 is top

## Planned Modules

```
abi/
├── Dependent.idr   # Dependent types (Pi, Sigma) with proofs of correctness
├── Linear.idr      # Linear types with proofs of resource safety
├── Session.idr     # Session types with proofs of protocol compliance
├── QTT.idr         # Quantitative Type Theory with usage tracking proofs
├── Effects.idr     # Effect system with composition proofs
├── Modal.idr       # Modal types with scope restriction proofs
├── Protocol.idr    # Verification protocol message type proofs
└── Proof.idr       # Proof term representation and verification proofs
```

## Policy

- **ZERO believe_me** — Every proof must be genuine. No shortcuts.
- **ZERO assert_total** — All functions must be provably total.
- **ZERO assert_smaller** — Termination must be proven structurally.
- **%default total** — All modules use total-by-default.

These proofs are the ENTIRE POINT of Typell. If the verification kernel
has holes in its own proofs, nothing it verifies is trustworthy.

## Relationship to Rust Kernel

The Idris2 modules specify WHAT the type checker must do. The Rust kernel
in `src/kernel/` implements HOW. The Idris2 proofs guarantee that the Rust
implementation is correct (assuming faithful translation).

Generated C headers in `generated/abi/` bridge Idris2 and Zig FFI per
the hyperpolymath ABI/FFI standard.
