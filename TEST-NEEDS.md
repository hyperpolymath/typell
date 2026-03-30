# TEST-NEEDS: typell

## Current State

| Category | Count | Details |
|----------|-------|---------|
| **Source modules** | 53 | Rust: typell-core (12: types, error, unify, infer, check, linear, effects, qtt, dimensional, session, proof, lib), typell-eclexia (3), typell-affinescript (3), typell-ephapax (3), typell-wokelang (3), typell-tangle (3), typell-betlang (3), typell-mylang (3), typell-oblibeny (3), typell-jtv (3), typell-phronesis (3), typell-errorlang (3), typell-vql (4) + 3 Idris2 ABI |
| **Unit tests (inline)** | 398 | Distributed across all crates -- types=21, unify=31, proof=14, session=7, dimensional=10, etc. |
| **Integration tests** | 2 files | core_comprehensive_tests.rs (95), vql_bridge_tests.rs (58) |
| **E2E tests** | 0 | None |
| **Benchmarks** | 0 | benches/.gitkeep ONLY |

## What's Missing

### P2P Tests
- [ ] No tests for cross-language bridge correctness (e.g., typell-wokelang checks match wokelang semantics)
- [ ] No tests for bridge interoperability (two language backends against same type)

### E2E Tests (CRITICAL)
- [ ] No test that type-checks a real program through the full pipeline
- [ ] No test for all 10 type system levels (dependent, linear, session, QTT, effects, modal, dimensional, proof, epistemic, tropical)
- [ ] No test for language backend integration with actual language compilers

### Aspect Tests
- [ ] **Security**: Type system = trust boundary; no soundness fuzzing
- [ ] **Performance**: No benchmarks despite being performance-critical type checker
- [ ] **Concurrency**: No parallel type checking tests
- [ ] **Error handling**: No tests for unsatisfiable constraints, infinite unification, cyclic types

### Benchmarks Needed (CRITICAL)
- [ ] **benches/.gitkeep is EMPTY** -- phantom benchmarks
- [ ] Type checking throughput (expressions/second)
- [ ] Unification performance scaling with constraint count
- [ ] QTT resource tracking overhead
- [ ] Session type verification latency
- [ ] Proof checking throughput

### Self-Tests
- [ ] No soundness self-check
- [ ] No regression suite for type system properties

## FLAGGED ISSUES
- **398 inline tests across 13 crates is respectable** -- best inline coverage among non-Julia repos
- **benches/.gitkeep = phantom benchmarks** -- a type checker with no performance measurements
- **13 language backends (eclexia through vql) each have only ~10 inline tests** -- thin coverage per backend
- **0 E2E for a 10-level type system** -- can't verify the type system actually works on real programs
- **core_comprehensive_tests.rs (95 tests) is solid** for the kernel

## Priority: P1 (HIGH) -- inline tests are decent but need E2E, benchmarks, and soundness proofs

## FAKE-FUZZ ALERT

- `tests/fuzz/placeholder.txt` is a scorecard placeholder inherited from rsr-template-repo — it does NOT provide real fuzz testing
- Replace with an actual fuzz harness (see rsr-template-repo/tests/fuzz/README.adoc) or remove the file
- Priority: P2 — creates false impression of fuzz coverage
