<!-- SPDX-License-Identifier: PMPL-1.0-or-later -->
<!-- Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk> -->

# 2. Reversibility Design as a Case Study in Type-System Power

Date: 2026-04-12

## Status

Accepted

## Context

In April 2026, the JTV v2 reversibility system for the 007 language underwent
a comprehensive design session. Five open design decisions were closed, covering:
variable mutation model, linear handle behaviour, agent state rollback, the
reversible–reverse pairing mechanism, and branch interaction.

During that session, three occasions arose where a proposed runtime mechanism,
tooling addition, or syntactic obligation was found to be **entirely redundant**
because the type system already expressed the same constraint. Each time,
the mechanism was dropped and the type system did the work alone.

These three collapses are documented here as a TypeLL case study — concrete
evidence of the principle that a well-designed type system eliminates
mechanisms rather than multiplying them.

---

## Decision

Record the three type-system collapses from the 007 JTV v2 reversibility
design as canonical examples of type-system power, for use in TypeLL
documentation, teaching material, and type-system design rationale.

---

## The Three Collapses

### Collapse 1 — `ExternalHandle` type eliminates `unsend` primitive

**Proposed mechanism:** A new `unsend(h, v)` primitive, callable only inside
`reverse { }` blocks, to undo a `send` on a linear handle. Required a new
grammar production, a new typechecker rule for reversal-context tracking,
and a handle state machine (`Ready → Consumed → Ready`).

**The type-system observation:** The handle already carries a type. If the
handle's type is `ExternalHandle` (crossing an agent boundary), then
attempting to `send` inside a `reversible { }` block is a **static error** —
the same way any other type violation is caught. No new primitive. No new
runtime check. No reversal-context flag in the typechecker.

**What the type does:** The `ExternalHandle` annotation is the enforcement
mechanism. The type tells you at the call site whether the send is
reversible-safe. `unsend` becomes unreachable — its only purpose was to
handle the case the type system now prevents statically.

**Lesson:** Before adding a primitive to handle a dangerous operation, ask
whether the operation's danger is already expressible in the type system. If
the type of the target already encodes the relevant constraint, the primitive
is redundant.

---

### Collapse 2 — `Option<ReversalToken>` eliminates the asymmetric-branch lint

**Proposed mechanism:** A compiler lint warning: "this `branch` is asymmetric
— some arms produce a `ReversalToken`, some do not. Did you mean to handle
both cases?" Required a separate analysis pass, a warning type, and
documentation of when to suppress it.

**The type-system observation:** At a `branch` join point where some arms
produce a `ReversalToken` and some do not, path-sensitive typing automatically
promotes the join type to `Option<ReversalToken<S>>`. The programmer who
receives `Option<ReversalToken<S>>` in scope *already knows* the branch was
asymmetric — the type told them. And `match` on `Option<ReversalToken<S>>`
forces exhaustive handling of `Some(tok)` and `None`. The programmer cannot
ignore or forget the asymmetric case.

**What the type does:** The `Option`-lifting IS the warning. It is stronger
than a lint — a lint can be suppressed, ignored, or not seen in CI. An
`Option` that the programmer must `match` on cannot be silently skipped.
The type enforces the handling; the lint would merely have suggested it.

**Lesson:** When you find yourself wanting to add a lint for "this pattern
is probably wrong," ask whether a type transformation at the relevant join
point would make the pattern impossible to ignore rather than merely
flagged. Lints are hints; types are guarantees.

---

### Collapse 3 — `ReversalToken<S>` type parameter eliminates the snapshot data structure for local bindings

**Proposed mechanism:** A `ReversalLog` / snapshot data structure in the
evaluator — a `HashMap<String, RtValue>` capturing all local bindings at
`reversible` entry, restored at `reverse`. Required a new struct, insertion
on every binding within a reversible block, and a restore pass.

**The type-system observation:** In a purely functional language, local
bindings do not need snapshotting. `let x = x + 3` inside a `reversible`
block creates a *new* binding that shadows the outer `x = 5`. When the block
exits, the shadow goes out of scope and the outer binding reappears. Lexical
scope provides snapshot-and-restore for local bindings automatically and for
free. The `HashMap` snapshot would have been capturing something the language
already maintained.

The *only* things that genuinely require snapshotting are `@state` fields —
because those escape lexical scope, persisting at agent lifetime. And these
are precisely what the `ReversalToken<S>` type parameter encodes: `S` is
the record type of the captured `@state` field values. The token IS the
snapshot. No separate data structure. No general-purpose `HashMap`. The
type parameter makes it precise, typed, and statically verified.

**What the type does:** `ReversalToken<{ @balance: Int }>` carries exactly
what was captured, at the type level. The evaluator's restore path unpacks
the token's payload and rebinds those fields — a typed operation over a
known structure, not a dynamic HashMap lookup. Decisions 1 (snapshot) and 4
(linear token) collapse into a single mechanism: the token IS the snapshot,
for the only things that actually needed snapshotting.

**Lesson:** Before adding a runtime data structure to track something, ask
whether the language's existing semantics already maintain it. In a purely
functional language, immutability and lexical scoping do enormous amounts of
tracking work invisibly. The snapshot data structure was solving a problem
that didn't exist in the language as designed.

---

## The Pattern

All three collapses follow the same shape:

1. A mechanism is proposed to handle a problem.
2. On closer inspection, the type system already expresses the relevant
   constraint, handles the relevant distinction, or maintains the relevant
   invariant.
3. The mechanism is dropped. The type system does the work.

The result in each case: **fewer moving parts, stronger guarantees, smaller
implementation surface.** A lint that can be suppressed becomes a type that
cannot be ignored. A primitive with a dangerous footgun becomes a type error.
A runtime data structure becomes a type parameter.

This is the direction a well-typed language should run: each addition to the
type system should eliminate at least one mechanism elsewhere. If you are
adding to the type system and also adding runtime machinery for the same
concern, the design is not finished.

---

## Consequences

### Positive

- Three concrete, worked examples of the "type system as mechanism eliminator"
  principle, grounded in a real language design session.
- Usable directly in TypeLL documentation, teaching material on linear types,
  and type-system design rationale documents.
- Demonstrates that the principle applies across different type-system features:
  type annotations on values (`ExternalHandle`), type algebra at join points
  (`Option`-lifting), and type parameters encoding captured state
  (`ReversalToken<S>`).
- The three collapses are believed to constitute a novel contribution to the
  reversible computation and type theory literature, publishable as part of
  the 007 / JTV v2 language papers.

### Negative

- The collapses only reveal themselves during design — they require the
  designer to resist the momentum toward adding a mechanism and instead ask
  "what does the type system already know?" This is a discipline, not a
  technique that can be automated.

### Neutral

- The underlying reversibility design decisions are documented fully in:
  - `007/docs/session-2026-04-12-jtv-v2-reversibility-design.adoc`
  - `julia-the-viper/docs/language/DESIGN-JTV-V2-REVERSIBILITY.md`
  - `nextgen-languages/docs/design/jtv-007-reversibility-fork.adoc`
- TypeLL does not implement the 007 reversibility system; this ADR records
  the design patterns as transferable knowledge, not as an implementation
  commitment.
