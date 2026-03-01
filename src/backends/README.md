# SPDX-License-Identifier: PMPL-1.0-or-later

# src/backends/ — Language-Specific Backends

Each backend adapts Typell's verification kernel for a specific query language.
Backends handle parsing, AST construction, and language-specific type rules,
then delegate to the kernel for core type checking, proof management, and
effect inference.

## Structure

```
backends/
├── vql/    # VQL-dt++ backend (VeriSimDB)
│           8-modality queries, cross-modal proofs, hexad types.
│           Ported from nextgen-databases/verisimdb/src/vql/ (ReScript).
│
├── gql/    # GQL-dt++ backend (LithoGlyph)
│           Knowledge graph queries, RATIONALE clause, refinement types.
│           Bridged from nextgen-databases/lithoglyph/gql-dt/ (Lean 4).
│           NOT ported — Lean 4's type system is kept intact.
│
└── kql/    # KQL-dt++ backend (QuandleDB)
            Knot invariant queries, category-theoretic schema model,
            equality saturation, HoTT identity types.
            Designed from scratch — no existing implementation to port.
```

## Backend Responsibilities

1. **Parse** query source into language-specific AST
2. **Transform** AST into kernel-compatible representation
3. **Provide** language-specific typing rules to the kernel
4. **Handle** language-specific proof kinds (e.g. VQL's EXISTENCE, GQL's RATIONALE)
5. **Format** kernel results back into language-specific feedback

## Adding a New Backend

A new backend needs:
- Parser for the query language
- AST → kernel representation transformer
- Language-specific typing rules
- Proof kind registry
- Result formatter
