# SPDX-License-Identifier: PMPL-1.0-or-later
# Design Document: Typell Vision
# Date: 2026-03-01
# Repo: typell
# Author: Jonathan D.A. Jewell (hyperpolymath)

## Summary

Typell is PanLL's verification kernel — the type-theoretic engine that provides
the "best of the best" type system coverage for neurosymbolic query languages.
It is not a standalone IDE. It is not a pane. It is the formal verification
substrate that makes PanLL's panes intelligent.

## The Problem

PanLL (the eNASAID — Environment for NeSy-Agentic Integrated Development) needs
a type-theory backbone to deliver on its promise. The nextgen-databases trilogy
(VeriSimDB, LithoGlyph, QuandleDB) each have or plan dependently typed query
languages (VCL-dt, GQL-dt, KQL-dt). These need to be extended to the ultimate
level of type-system strictness and connected to PanLL's interface.

The risk: building this as a separate IDE creates two competing tools that both
collapse under their own weight. Building it as "just a pane" undersells the
architectural depth required. The challenge is making the verification
infrastructure core to PanLL without interfering with PanLL's own development.

## The Solution: Layered Architecture

Typell is to PanLL what LLVM is to Clang. One system, two separable layers:

```
┌─────────────────────────────────────────────────┐
│  PanLL (Layer 2: Development Environment)       │
│  ├─ Pane-L ←── constraints from Typell          │
│  ├─ Pane-N ←── reasoning from Typell            │
│  └─ Pane-W ←── validated results from Typell    │
│  ├─ Anti-Crash (validates Typell's output)       │
│  ├─ Vexometer (operator stress tracking)         │
│  ├─ OrbitalSync (cross-pane synchronisation)     │
│  └─ Binary Star co-orbit governance              │
└──────────────┬──────────────────────────────────┘
               │ Verification Protocol (JSON-RPC)
               │ (the clean API boundary)
┌──────────────┴──────────────────────────────────┐
│  Typell (Layer 1: Verification Kernel)           │
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │  Type Checker (Bidirectional)             │   │
│  │  ├─ Dependent types (Pi, Sigma)           │   │
│  │  ├─ Linear types (use exactly once)       │   │
│  │  ├─ Affine types (use at most once)       │   │
│  │  ├─ Session types (protocol safety)       │   │
│  │  ├─ QTT (resource quantity tracking)      │   │
│  │  ├─ Effect system (read/write/memory)     │   │
│  │  └─ Modal types (contextual access)       │   │
│  └──────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────┐   │
│  │  Proof Engine                             │   │
│  │  ├─ Automated generation (simple proofs)  │   │
│  │  ├─ Echidna dispatch (complex proofs)     │   │
│  │  ├─ Verification (certificate checking)   │   │
│  │  ├─ Caching (proof repository)            │   │
│  │  └─ Certificates (cryptographic PCC)      │   │
│  └──────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────┐   │
│  │  Language Backends                        │   │
│  │  ├─ VCL-dt++ (VeriSimDB)                  │   │
│  │  ├─ GQL-dt++ (LithoGlyph)                │   │
│  │  └─ KQL-dt++ (QuandleDB)                 │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  Formal Specs: Idris2 (src/abi/)                │
│  Implementation: Rust (src/kernel/)              │
│  FFI Bridge: Zig (ffi/zig/)                      │
└──────────────────────────────────────────────────┘
       ↑               ↑              ↑
    VS Code          CLI/CI        Databases
   extension        pipelines    (direct query
                                  validation)
```

### Why This Works

1. **PanLL can't be hijacked** — Typell IS PanLL's backend, not a competitor
2. **No duplication** — one verification engine, many consumers
3. **Independent development** — PanLL UI can progress without waiting for Typell
4. **Graceful degradation** — PanLL works without Typell (string-based constraints)
5. **Incremental value** — each type system feature is independently useful
6. **External value** — VS Code/CLI/CI users don't need PanLL to benefit

## Type System Coverage: "The Best of the Best"

The goal is maximal strictness — every type system feature that makes formal
verification practical for database queries. Not as a theoretical exercise,
but with tooling that makes it usable.

### Tier 1: Core (Must Have)

| System | Purpose | Source |
|--------|---------|--------|
| **Dependent Types** | Types that depend on values. `Vector 5 Int` = exactly 5 integers. Schema-aware queries, precise result specs, proof obligations. | Existing in VCL-dt (Pi, Sigma types), GQL-dt (Lean 4 refinement types) |
| **Linear Types** | Resources used exactly once. No duplicate reads, no data leaks, transaction safety. `SELECT LINEAR GRAPH.* ... CONSUME AFTER 1 USE` | New for all query languages |
| **Session Types** | Protocol safety. Connections opened/closed correctly, transactions atomic. `WITH SESSION (OPEN, QUERY, CLOSE)` | New for all query languages |
| **Proof-Carrying Code** | Cryptographic proof certificates attached to queries. Zero-trust verification. `PROOF ATTACHED { theorem, proof: "sha256:..." }` | Partial in VCL-dt (proof obligations), full in GQL-dt (RATIONALE clause) |

### Tier 2: Advanced (Should Have)

| System | Purpose | Source |
|--------|---------|--------|
| **Quantitative Type Theory** | Track resource usage quantities. Rate limiting, cost analysis. `USAGE LIMIT 3` | Idris2 already has QTT natively |
| **Effect Systems** | Explicit side effects. `EFFECTS { read: [GRAPH, DOCUMENT], write: [], memory: <50MB }` | VCL-dt has partial effect tracking |
| **Modal Types** | Contextual access. Data only available within specific scopes. `IN TRANSACTION tx1` | New |
| **Affine Types** | Resources used at most once (relaxation of linear). Graceful cleanup. | Natural extension of linear types |

### Tier 3: Research (Could Have)

| System | Purpose | Source |
|--------|---------|--------|
| **HoTT** | Homotopy Type Theory for query equivalence proofs | KQL-dt++ research (knot equivalence) |
| **Equality Saturation** | E-graphs for equivalence classes (egglog) | KQL-dt++ research |
| **Category-Theoretic Types** | Schema = category, query = natural transformation, migration = functor | KQL-dt++ research (Spivak CQL) |
| **Substructural Types** | Unified framework for linear/affine/relevant disciplines | Long-term unification |

## The Verification Protocol

The primary interface. Any consumer talks to Typell via JSON-RPC.

### Core Operations

```
typell.check(query, language) → TypeResult
  Returns: types, proof obligations, effects, session protocol, errors

typell.prove(obligation, strategy) → ProofResult
  Returns: proof certificate, verification status, time taken

typell.infer(partial_query, context) → InferenceResult
  Returns: inferred types, suggestions, completions

typell.validate(query, proof_certificate) → ValidationResult
  Returns: valid/invalid, violations, counterexamples

typell.refactor(query, transformation) → RefactorResult
  Returns: rewritten query, proof of equivalence

typell.effects(query) → EffectResult
  Returns: reads, writes, memory estimate, modality access

typell.session(interaction_sequence) → SessionResult
  Returns: protocol compliance, violations, suggested fixes

typell.drift(proof_certificate, current_data_state) → DriftResult
  Returns: still valid / invalidated, repair suggestions
```

### Example Flow (PanLL Integration)

```
Operator writes in Pane-L:
  SELECT GRAPH.*, DOCUMENT.* FROM HEXAD 'entity-001'
    PROOF EXISTENCE(entity-001)
    EFFECTS { read: [GRAPH, DOCUMENT], memory: <50MB }

PanLL sends to Typell via protocol:
  typell.check(query, "vcl-dt++")

Typell responds:
  {
    "type": "ProvedResult<Linear<Hexad>, [ExistenceProof]>",
    "proof_obligations": [
      { "kind": "EXISTENCE", "target": "entity-001", "status": "auto-generated" }
    ],
    "effects": { "read": ["GRAPH", "DOCUMENT"], "write": [], "memory_estimate": "42MB" },
    "session": { "protocol": "valid", "connection_lifecycle": "single-shot" },
    "errors": [],
    "warnings": []
  }

PanLL renders in:
  Pane-N: "Proof obligation: EXISTENCE(entity-001) — auto-generated. ✅"
  Pane-N: "Memory effect: 42MB (under 50MB limit). ✅"
  Pane-W: ProvedResult with certificate sha256:abc123...
```

## Language Backend Strategy

### VCL-dt++ (VeriSimDB)

**Source:** `nextgen-databases/verisim/src/vcl/` — ReScript implementation
**Status:** ~70% complete (parser, type checker, bidirectional inference, proof obligations)
**Strategy:** Port logic from ReScript to Rust kernel. Extend with linear/session/QTT/effects during port.

Key existing components to port:
- `VQLParser.res` — Full untyped AST with 8-modality awareness
- `VQLTypes.res` — Pi, Sigma, ProofType, ProvedResultType
- `VQLBidir.res` — Bidirectional type inference (250+ lines)
- `VQLProofObligation.res` — Proof obligation generation with composition
- `VQLContext.res` — Type context with contract registry
- `VQLSubtyping.res` — Subtyping relation for dependent types
- `VQLCircuit.res` — Custom circuit DSL for PROOF CUSTOM

### GQL-dt++ (LithoGlyph)

**Source:** `nextgen-databases/lithoglyph/gql-dt/` — Lean 4 implementation
**Status:** 100% production ready (LSP, VS Code extension, Zig FFI, SLSA Level 3)
**Strategy:** Do NOT port. Bridge via Verification Protocol. Lean 4's type system is already
powerful. Extend GQL-dt with linear/session types via protocol-level composition.

Key existing components to bridge:
- Lean 4 refinement types (BoundedNat, NonEmptyString)
- RATIONALE clause (proof tracking)
- LSP server (180 LOC, already protocol-based)
- Zig FFI bridge (5 core functions)
- VS Code extension (TextMate grammar, syntax highlighting)

### KQL-dt++ (QuandleDB)

**Source:** Research document only (`nextgen-databases/quandledb/docs/design/KQL-SQL-LANDSCAPE-RESEARCH-2026-02-22.md`)
**Status:** Research phase — no implementation
**Strategy:** Design from scratch within Typell. Category-theoretic foundation (Spivak CQL).
HoTT identity types for knot equivalence. Equality saturation (egglog) for optimisation.

Key design decisions needed:
- Pipeline syntax (PRQL-style) vs. declarative (SQL-style) vs. hybrid
- Category-theoretic schema model (functorial data migration)
- E-graph integration for equivalence classes
- Lean 4 mathlib quandle formalisation integration

## Tooling Ecosystem

What makes Typell practical, not just theoretically powerful:

### 1. Type-Aware Editor Experience

When connected to PanLL (or VS Code via extension):
- **Real-time type annotations** for every clause
- **Proof obligation display** ("You must prove EXISTENCE(entity-001)")
- **Resource usage tracking** ("45MB memory, reads GRAPH + DOCUMENT")
- **Linear variable colour-coding** (used-once vs. unused vs. consumed)
- **Session protocol visualisation** (sidebar showing connection lifecycle)
- **Autocompletion for proofs** (suggests available proof kinds from schema)
- **Auto-generated proof skeletons** for common queries
- **Query refactoring** (type-safe rewrites preserving correctness)

### 2. Proof Assistant Integration

- **Simple proofs:** Auto-generated by Typell kernel (existence, basic integrity)
- **Complex proofs:** Delegated to Echidna (Z3 SMT, CVC5 SMT, E ATP)
- **Interactive proofs:** Open proof assistant pane for custom proof construction
- **Proof repository:** Store and reuse proofs across teams/federated nodes
- **Counterexample debugging:** When a proof fails, suggest counterexamples
- **Proof certificates:** Compact cryptographic certificates attached to queries

### 3. Compiler Pipeline

```
Query Source
    │
    ▼
Parse (AST)
    │
    ▼
Bidirectional Type Check
├─ Dependent type inference/checking
├─ Linear resource tracking
├─ Session protocol verification
├─ QTT resource accounting
├─ Effect inference
└─ Modal scope checking
    │
    ▼
Proof Obligation Generation
    │
    ▼
Proof Resolution
├─ Auto-generate simple proofs
├─ Dispatch complex to Echidna
└─ Verify provided certificates
    │
    ▼
ProvedResult + Certificate
    │
    ▼
Execute (database) or Display (PanLL/editor)
```

### 4. Monitoring and Debugging

- **Query tracer:** Step-through execution with proof/effect visualisation
- **Proof coverage:** Which parts of a query are proven, which are not
- **Drift dashboard:** Real-time detection of proof invalidation due to data changes
- **Proof health metrics:** Success/failure rates, resource usage, session compliance
- **CI/CD integration:** Automated proof checking in pipelines

### 5. Education and Onboarding

- **Interactive tutorials:** Teach linear types, session types, proofs by example
- **Query templates:** Pre-approved templates for common tasks
- **Error explanations:** User-friendly messages ("This variable is linear and cannot be copied")
- **Progressive disclosure:** Start simple, reveal complexity as needed

## VCL-dt vs VCL-dt++ Feature Comparison

| Feature | VCL-dt (current) | VCL-dt++ (Typell) | Kernel Component |
|---------|-----------------|-------------------|------------------|
| Dependent types (Pi, Sigma) | Yes | Yes | Bidirectional type checker |
| Proof obligations (EXISTENCE, etc.) | Yes | Yes | Proof engine |
| ZKP witness generation | Yes | Yes | Proof certificates |
| **Linear types** | No | `CONSUME AFTER n USE` | Linear resource tracker |
| **Session types** | No | `WITH SESSION protocol` | Session protocol manager |
| **Effect systems** | Partial | `EFFECTS { Read, Write, ... }` | Compositional effect inference |
| **Modal types** | No | `IN TRANSACTION state` | Modal scope checker |
| **Proof-carrying code** | Partial (pre-conditions) | `PROOF ATTACHED theorem` | Cryptographic PCC |
| **QTT** | No | `USAGE LIMIT n` | Quantitative type tracker |

Grammar delta: `nextgen-databases/typeql-experimental/docs/vcl-dtpp-grammar.ebnf` (199 lines)
Normative spec: `nextgen-databases/verisim/docs/VCL-SPEC.adoc` Appendix E

## Individual Feature Syntax Examples

Each dt++ clause shown in isolation. These are independently useful — not
all-or-nothing.

### Linear Types — CONSUME AFTER

```sql
-- Single-use: result consumed exactly once, then invalidated
SELECT GRAPH.*, DOCUMENT.* FROM HEXAD 'entity-001'
  PROOF EXISTENCE(entity-001)
  CONSUME AFTER 1 USE;

-- Multi-use with budget: federation retry budget of 3
SELECT * FROM FEDERATION '/universities/*'
  WITH DRIFT STRICT
  CONSUME AFTER 3 USE;
```

**What it prevents:** duplicate reads, data leaks, unbounded result sharing.
**Idris2 ABI:** `(1 conn : Connection)` for single-use, `(n conn : BoundedConn n)` for multi-use.

### Session Types — WITH SESSION

```sql
-- Read-only: can query but type system prevents mutations
SELECT GRAPH FROM HEXAD 'entity-001'
  WITH SESSION ReadOnlyProtocol;

-- Mutation: INSERT/UPDATE/DELETE allowed
INSERT HEXAD WITH DOCUMENT(title = 'New Entry')
  WITH SESSION MutationProtocol;

-- Streaming: cursor-based result batching
SELECT * FROM FEDERATION '/sensors/*'
  WITH SESSION StreamProtocol;
```

**What it prevents:** protocol violations (writing in read-only session, querying
on closed connection, committing without opening transaction).
**State machine:** Fresh → Authenticated → InTransaction → Committed → Closed.
**Built-in protocols:** `ReadOnlyProtocol`, `MutationProtocol`, `StreamProtocol`, `BatchProtocol`.

### Effect Systems — EFFECTS

```sql
-- Pure read: type checker rejects any writes
SELECT GRAPH FROM HEXAD 'entity-001'
  EFFECTS { Read };

-- Mutation with audit trail
INSERT HEXAD WITH DOCUMENT(title = 'Audited Entry')
  PROOF INTEGRITY(schema-v2)
  EFFECTS { Read, Write, Audit };

-- Federation with data transformation
SELECT VECTOR FROM FEDERATION '/cluster/*'
  EFFECTS { Read, Federate, Transform };
```

**What it prevents:** undeclared side effects. Checker verifies actual operations ⊆ declared effects.
**Available effects:** `Read`, `Write`, `Cite`, `Audit`, `Transform`, `Federate` (extensible).

### Modal Types — IN TRANSACTION

```sql
-- Only visible in committed state
SELECT GRAPH FROM HEXAD 'entity-001'
  IN TRANSACTION Committed;

-- Snapshot isolation: consistent view at query time
SELECT * FROM FEDERATION '/analytics/*'
  IN TRANSACTION ReadSnapshot;
```

**What it prevents:** data scope leaks. Data in one transaction scope cannot leak
to another without explicit marshalling.
**Transaction states:** `Fresh`, `Active`, `Committed`, `RolledBack`, `ReadSnapshot`.

### Proof-Carrying Code — PROOF ATTACHED

```sql
-- Attach post-condition theorem to result (different from PROOF pre-condition)
SELECT GRAPH FROM HEXAD 'entity-001'
  PROOF EXISTENCE(entity-001)
  PROOF ATTACHED IntegrityTheorem;

-- Freshness guarantee on federation results
SELECT * FROM FEDERATION '/realtime/*'
  WITH DRIFT STRICT
  PROOF ATTACHED FreshnessGuarantee;

-- Parameterised theorem
SELECT DOCUMENT FROM HEXAD 'entity-001'
  PROOF ATTACHED CrossModalConsistency(tolerance = 0.01);
```

**What it provides:** cryptographic proof certificates attached to query results.
Zero-trust verification — consumers can verify results without trusting the source.
**Idris2 ABI:** `ProvedResult : (result : QueryResult) -> (prf : Theorem) -> Type`.

### Quantitative Type Theory — USAGE LIMIT

```sql
-- Cap resource operations (connections, store reads, API calls)
SELECT GRAPH FROM HEXAD 'entity-001'
  USAGE LIMIT 100;

-- Federation with bounded resource budget
SELECT * FROM FEDERATION '/global/*'
  WITH DRIFT TOLERATE
  USAGE LIMIT 1000;
```

**What it provides:** bounded resource consumption across the query plan.
Different from `LIMIT` (which caps result rows).
**Idris2 ABI:** `BoundedResource : (n : Nat) -> Type`. Generalises linear types
from exact-1 to at-most-n.

## Example: A VCL-dt++ Query Through Typell (All Six Combined)

```sql
-- Maximal strictness: linear, session-typed, effect-annotated, proof-carrying
WITH SESSION (
  OPEN CONNECTION TO FEDERATION,
  QUERY LINEAR GRAPH.*, DOCUMENT.* FROM HEXAD 'entity-001'
    PROOF EXISTENCE(entity-001) AND PROVENANCE(entity-001)
    EFFECTS { read: [GRAPH, DOCUMENT], write: [], memory: <50MB }
    USAGE LIMIT 1
    IN TRANSACTION tx1,
  CLOSE CONNECTION
) AS strict_query
SELECT * FROM strict_query
  PROOF ATTACHED {
    theorem: "cross_modal_consistency",
    proof: "sha256:def456..."
  };
```

Typell's response:

```json
{
  "type": "ProvedResult<Linear<Hexad<GRAPH, DOCUMENT>>, [ExistenceProof, ProvenanceProof, ConsistencyProof]>",
  "proof_obligations": [
    { "kind": "EXISTENCE", "target": "entity-001", "status": "auto-generated", "time_ms": 50 },
    { "kind": "PROVENANCE", "target": "entity-001", "status": "auto-generated", "time_ms": 200 },
    { "kind": "CROSS_MODAL_CONSISTENCY", "status": "verified", "certificate": "sha256:def456..." }
  ],
  "linear_tracking": { "strict_query": { "uses": 1, "limit": 1, "status": "compliant" } },
  "session_protocol": { "status": "valid", "sequence": ["OPEN", "QUERY", "CLOSE"], "transaction": "tx1" },
  "effects": { "read": ["GRAPH", "DOCUMENT"], "write": [], "memory_estimate_mb": 42 },
  "modal_scope": { "transaction": "tx1", "data_accessible_only_within": true },
  "errors": [],
  "warnings": [],
  "certificate": {
    "hash": "sha256:abc123...",
    "timestamp": "2026-03-01T12:00:00Z",
    "verifier": "typell-kernel-v0.1.0"
  }
}
```

## Implementation Technology

| Layer | Technology | Rationale |
|-------|-----------|-----------|
| **Formal Specs** | Idris2 (`src/abi/`) | Dependent types prove type system soundness. Zero believe_me. |
| **Kernel** | Rust (`src/kernel/`) | Performance, safety, Tauri compatibility (PanLL backend). |
| **FFI** | Zig (`ffi/zig/`) | C ABI compatibility per hyperpolymath standard. |
| **Protocol** | JSON-RPC | Language-agnostic, well-tooled, LSP-adjacent. |
| **GQL-dt bridge** | Lean 4 (existing) | Don't port — bridge. Lean's type system is already powerful. |
| **Proof dispatch** | gRPC to Echidna | Multi-solver theorem proving (Z3, CVC5, E). |

## What Typell Is NOT

- **NOT a standalone IDE** — it's a verification engine consumed by IDEs
- **NOT a pane** — it's the intelligence behind all three panes
- **NOT a database** — it validates queries, doesn't store data
- **NOT a replacement for PanLL** — it IS PanLL's backend
- **NOT a general-purpose type checker** — it's specialised for query languages
- **NOT a theorem prover** — it delegates complex proofs to Echidna

## Development Priorities

**Rule:** PanLL is the priority. Typell must never divert effort from PanLL.

1. **Phase 0 (NOW):** Capture the vision. This document. Repo scaffolding.
2. **Phase 1:** Formal type system spec in Idris2 (dependent + linear + QTT)
3. **Phase 2:** Verification Protocol specification (JSON-RPC schema)
4. **Phase 3:** Rust kernel — bidirectional type checker (port VCL-dt logic)
5. **Phase 4:** Rust kernel — proof engine
6. **Phase 5:** PanLL integration (Pane-N + Pane-L)
7. **Phase 6-8:** Language backends (VCL-dt++, GQL-dt++, KQL-dt++)
8. **Phase 9-10:** VS Code extension, CLI, CI/CD plugins

Each phase delivers independent value. No big bang.

## Open Questions

1. Should the VCL-dt ReScript code be ported to Rust, or should Typell
   call VeriSimDB's existing type checker via the protocol?
2. How tightly should Typell couple with Echidna for proof dispatch?
3. ~~What is the right syntax for linear/session annotations in each query language?~~
   **RESOLVED:** VCL-dt++ grammar delta specifies all six clauses (`vcl-dtpp-grammar.ebnf`).
   GQL-dt++ and KQL-dt++ syntax TBD but will follow the same clause pattern.
4. Should Typell define a universal query AST that all backends parse into,
   or should each backend maintain its own AST?
5. How does the PanLL v0.2.0 VeriSimDB integration timeline align with Typell?
