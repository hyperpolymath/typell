<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- TOPOLOGY.md — Typell architecture map and completion dashboard -->
<!-- Last updated: 2026-03-01 -->

# Typell — Project Topology

## System Architecture

```
                    ┌──────────────────────────────────────────────────────┐
                    │          PanLL (Primary Consumer)                    │
                    │                                                      │
                    │   ┌─────────┐  ┌──────────┐  ┌──────────┐          │
                    │   │ Pane-L  │  │  Pane-N  │  │  Pane-W  │          │
                    │   │Symbolic │  │ Neural   │  │ World /  │          │
                    │   │  Mass   │  │ Stream   │  │  Task    │          │
                    │   └────┬────┘  └────┬─────┘  └────┬─────┘          │
                    │        │            │              │                 │
                    │   constraints   reasoning    validated results       │
                    └────────┼────────────┼──────────────┼────────────────┘
                             │            │              │
                    ═════════╪════════════╪══════════════╪═════════════════
                    Verification Protocol (JSON-RPC)
                    ═════════╪════════════╪══════════════╪═════════════════
                             │            │              │
                    ┌────────┴────────────┴──────────────┴────────────────┐
                    │                  TYPELL KERNEL                       │
                    │                                                      │
                    │   ┌──────────────────────────────────────────────┐  │
                    │   │         Bidirectional Type Checker            │  │
                    │   │                                              │  │
                    │   │  ┌────────────┐  ┌──────────┐  ┌─────────┐  │  │
                    │   │  │ Dependent  │  │  Linear  │  │ Session │  │  │
                    │   │  │   Types    │  │  Types   │  │  Types  │  │  │
                    │   │  │ (Pi,Sigma) │  │ (once)   │  │(proto)  │  │  │
                    │   │  └────────────┘  └──────────┘  └─────────┘  │  │
                    │   │  ┌────────────┐  ┌──────────┐  ┌─────────┐  │  │
                    │   │  │    QTT     │  │ Effects  │  │  Modal  │  │  │
                    │   │  │ (quantity) │  │(r/w/mem) │  │ (scope) │  │  │
                    │   │  └────────────┘  └──────────┘  └─────────┘  │  │
                    │   └──────────────────────────────────────────────┘  │
                    │                                                      │
                    │   ┌──────────────────────────────────────────────┐  │
                    │   │              Proof Engine                     │  │
                    │   │  ┌──────┐  ┌────────┐  ┌──────┐  ┌───────┐  │  │
                    │   │  │ Auto │  │Echidna │  │Verify│  │ Cache │  │  │
                    │   │  │ Gen  │  │Dispatch│  │ Cert │  │ Repo  │  │  │
                    │   │  └──────┘  └───┬────┘  └──────┘  └───────┘  │  │
                    │   └────────────────┼─────────────────────────────┘  │
                    │                    │                                 │
                    │   ┌────────────────┼─────────────────────────────┐  │
                    │   │         Language Backends                     │  │
                    │   │  ┌──────────┐  │  ┌──────────┐  ┌─────────┐ │  │
                    │   │  │ VQL-dt++ │  │  │ GQL-dt++ │  │KQL-dt++ │ │  │
                    │   │  │VeriSimDB │  │  │LithoGlyph│  │QuandleDB│ │  │
                    │   │  │ (Rust)   │  │  │(Lean4    │  │(Rust,   │ │  │
                    │   │  │          │  │  │ bridge)  │  │ new)    │ │  │
                    │   │  └─────┬────┘  │  └─────┬────┘  └────┬────┘ │  │
                    │   └────────┼───────┼────────┼────────────┼──────┘  │
                    └────────────┼───────┼────────┼────────────┼─────────┘
                                 │       │        │            │
                    ┌────────────┼───────┼────────┼────────────┼─────────┐
                    │     FORMAL FOUNDATION                               │
                    │  ┌─────────┴───┐  ┌┴────────┴──┐  ┌──────┴───────┐ │
                    │  │  Idris2 ABI │  │  Zig FFI   │  │ Echidna      │ │
                    │  │  (proofs)   │  │  (C ABI)   │  │ (Z3,CVC5,E) │ │
                    │  └─────────────┘  └────────────┘  └──────────────┘ │
                    └─────────────────────────────────────────────────────┘

                    ┌─────────────────────────────────────────────────────┐
                    │        SECONDARY CONSUMERS                          │
                    │  ┌──────────┐  ┌──────────┐  ┌──────────────────┐  │
                    │  │ VS Code  │  │  CLI     │  │  CI/CD Plugins   │  │
                    │  │Extension │  │  Tool    │  │  (GitHub Actions) │  │
                    │  └──────────┘  └──────────┘  └──────────────────┘  │
                    └─────────────────────────────────────────────────────┘
```

## Completion Dashboard

```
COMPONENT                            STATUS              NOTES
──────────────────────────────────  ──────────────────  ───────────────────────────
FOUNDATION (Phase 0)
  Vision document                    ██████████ 100%    DESIGN-2026-03-01-typell-vision.md
  Repo scaffolding (RSR)             ██████████ 100%    Template customised, dirs created
  AI Manifest                        ██████████ 100%    0-AI-MANIFEST.a2ml bespoke
  SCM metadata                       ██████████ 100%    STATE/ECOSYSTEM/META/NEUROSYM
  TOPOLOGY.md                        ██████████ 100%    This file

FORMAL SPECIFICATIONS (Phase 1)
  Verification Protocol spec         ░░░░░░░░░░   0%    JSON-RPC schema TBD
  Dependent type spec                ░░░░░░░░░░   0%    Pi, Sigma formal rules
  Linear type spec                   ░░░░░░░░░░   0%    Substructural rules
  Session type spec                  ░░░░░░░░░░   0%    Protocol compliance rules
  QTT spec                           ░░░░░░░░░░   0%    Quantity tracking rules
  Effect system spec                 ░░░░░░░░░░   0%    Compositional effects
  Modal type spec                    ░░░░░░░░░░   0%    Scope restriction rules
  Proof system spec                  ░░░░░░░░░░   0%    Gen/verify/cert/compose

IDRIS2 ABI (Phase 1)
  Types.idr                          ░░░░░░░░░░   0%    Core type representations
  Dependent.idr                      ░░░░░░░░░░   0%    Pi/Sigma proofs
  Linear.idr                         ░░░░░░░░░░   0%    Resource safety proofs
  Session.idr                        ░░░░░░░░░░   0%    Protocol compliance proofs
  QTT.idr                            ░░░░░░░░░░   0%    Quantity tracking proofs
  Effects.idr                        ░░░░░░░░░░   0%    Composition proofs
  Checker.idr                        ░░░░░░░░░░   0%    Soundness/completeness proofs

RUST KERNEL (Phases 3-5)
  Bidirectional type checker         ░░░░░░░░░░   0%    Port from VQL-dt + extend
  Proof engine                       ░░░░░░░░░░   0%    Auto-gen, Echidna dispatch
  Effect tracker                     ░░░░░░░░░░   0%    Compositional inference
  Session protocol manager           ░░░░░░░░░░   0%    Connection lifecycle
  JSON-RPC protocol server           ░░░░░░░░░░   0%    Primary interface

LANGUAGE BACKENDS (Phases 6-8)
  VQL-dt++ (VeriSimDB)               ░░░░░░░░░░   0%    Port from ReScript
  GQL-dt++ (LithoGlyph)             ░░░░░░░░░░   0%    Bridge to Lean 4
  KQL-dt++ (QuandleDB)              ░░░░░░░░░░   0%    Design from scratch

INTEGRATIONS (Phases 9-10)
  PanLL (Pane-N + Pane-L)           ░░░░░░░░░░   0%    Primary consumer
  VS Code extension                  ░░░░░░░░░░   0%    Secondary consumer
  CLI tool                           ░░░░░░░░░░   0%    Secondary consumer
  CI/CD plugins                      ░░░░░░░░░░   0%    Secondary consumer

──────────────────────────────────────────────────────────────────────────────
OVERALL:                             █░░░░░░░░░  10%    Phase 0 complete. Phase 1 next.
```

## Key Dependencies

```
PanLL ──────────────► Typell Kernel ──────────► Idris2 ABI (proofs)
  │                      │                         │
  │                      │                         ▼
  │                      │                    Zig FFI (C ABI)
  │                      │
  │                      ├──► VQL-dt++ ──────► VeriSimDB
  │                      ├──► GQL-dt++ ──────► LithoGlyph (Lean 4)
  │                      └──► KQL-dt++ ──────► QuandleDB
  │                      │
  │                      └──► Echidna ──────► Z3 / CVC5 / E
  │
  ├──► VS Code ext ──► Typell Protocol (JSON-RPC)
  ├──► CLI tool ─────► Typell Protocol (JSON-RPC)
  └──► CI/CD ────────► Typell Protocol (JSON-RPC)
```

## Critical Path

```
Phase 0 (DONE) ──► Phase 1 (Specs) ──► Phase 3 (Kernel) ──► Phase 9 (PanLL)
                       │                     │
                       ▼                     ▼
                   Phase 1 (ABI)        Phase 6-8 (Backends)
                       │
                       ▼
                   Phase 2 (Protocol)
```

## Update Protocol

This file is maintained by both humans and AI agents. When updating:

1. **After completing a component**: Change its bar and percentage
2. **After adding a component**: Add a new row in the appropriate section
3. **After architectural changes**: Update the ASCII diagram
4. **Date**: Update the `Last updated` comment at the top of this file

Progress bars use: `█` (filled) and `░` (empty), 10 characters wide.
Percentages: 0%, 10%, 20%, ... 100% (in 10% increments).
