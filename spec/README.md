# SPDX-License-Identifier: PMPL-1.0-or-later

# spec/ — Formal Specifications

This directory contains the formal specifications for Typell's type systems,
verification protocol, and proof infrastructure.

## Structure

```
spec/
├── protocol/           # Verification Protocol specification
│   └── TYPELL-PROTOCOL.adoc   # JSON-RPC protocol spec
├── type-system/        # Type system formal specifications
│   ├── dependent.adoc  # Dependent types (Pi, Sigma)
│   ├── linear.adoc     # Linear types (use-exactly-once)
│   ├── session.adoc    # Session types (protocol safety)
│   ├── qtt.adoc        # Quantitative Type Theory
│   ├── effects.adoc    # Effect system spec
│   └── modal.adoc      # Modal types (contextual access)
└── proof/              # Proof system specifications
    ├── generation.adoc     # Automated proof generation
    ├── verification.adoc   # Proof verification
    ├── certificates.adoc   # Proof-carrying code / certificates
    └── composition.adoc    # Multi-proof composition
```

## Purpose

These specifications are the source of truth for Typell's behaviour. The Idris2
ABI definitions in `src/abi/` formalise these specs with machine-checked proofs.
The Rust kernel in `src/kernel/` implements them.

## Writing Specs

- Use AsciiDoc format (.adoc)
- Include formal typing rules where applicable
- Reference relevant academic literature
- Each spec should be self-contained but cross-reference related specs
- Update specs before changing implementation, not after
