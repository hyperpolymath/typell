# SPDX-License-Identifier: PMPL-1.0-or-later

# src/abi/ — Idris2 ABI Definitions (Formal Specifications)

This directory contains the formal specifications for Typell's type systems,
written in Idris2 with dependent type proofs. These specs are the mathematical
foundation that proves Typell's type checker is sound, complete, and decidable.

## Planned Modules

```
abi/
├── Types.idr       # Core type representations (primitives, modalities, hexad types)
├── Dependent.idr   # Dependent types (Pi, Sigma) with proofs of correctness
├── Linear.idr      # Linear types with proofs of resource safety
├── Session.idr     # Session types with proofs of protocol compliance
├── QTT.idr         # Quantitative Type Theory with usage tracking proofs
├── Effects.idr     # Effect system with composition proofs
├── Modal.idr       # Modal types with scope restriction proofs
├── Checker.idr     # Type checker correctness proofs (soundness, completeness)
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
