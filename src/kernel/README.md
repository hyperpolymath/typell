# SPDX-License-Identifier: PMPL-1.0-or-later

# src/kernel/ — Rust Verification Kernel

This is Typell's core implementation in Rust. It implements the type checker,
proof engine, effect tracker, session manager, and verification protocol server.

## Structure

```
kernel/
├── checker/      # Bidirectional type checker
│   │             Implements: dependent, linear, affine, session, QTT,
│   │             effect, and modal type checking.
│   │             Port of VQL-dt's VQLBidir.res logic, extended.
│   └── ...
├── proof/        # Proof engine
│   │             Automated proof generation for simple cases.
│   │             Echidna dispatch for complex proofs (Z3, CVC5, E).
│   │             Proof verification, caching, and certificate management.
│   └── ...
├── effects/      # Effect tracker
│   │             Compositional effect inference.
│   │             Tracks: reads, writes, memory usage, modality access.
│   └── ...
├── session/      # Session protocol manager
│   │             Verifies connection lifecycles (open/query/close).
│   │             Transaction atomicity checking.
│   └── ...
└── protocol/     # Verification Protocol server (JSON-RPC)
                  The primary interface. PanLL, VS Code, CLI, and CI/CD
                  all communicate with Typell through this server.
```

## Design Principles

- **Correct by construction:** Rust's type system prevents many bugs. Idris2
  specs in `src/abi/` prove the algorithms correct.
- **Protocol-first:** The JSON-RPC protocol server is the primary interface.
  Everything else is internal implementation detail.
- **Incremental:** Each type system feature can be enabled/disabled independently.
  A consumer that only needs dependent types doesn't pay for session types.
- **No unsafe without SAFETY comment:** Per hyperpolymath policy.
- **No transmute unless FFI boundary:** Per hyperpolymath policy.

## Relationship to PanLL

This kernel IS PanLL's Pane-N reasoning engine. When compiled as a Tauri plugin,
it runs in-process with PanLL. When run as a standalone server, PanLL connects
via JSON-RPC. Both modes use the same kernel code.
