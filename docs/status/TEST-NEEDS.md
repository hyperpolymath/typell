# TEST-NEEDS: typell

## CRG Grade: C — ACHIEVED 2026-04-04

## Current State (verified 2026-04-04)

| Category | Count | Details |
|----------|-------|---------|
| **Source modules** | 53 | Rust: typell-core (12: types, error, unify, infer, check, linear, effects, qtt, dimensional, session, proof, lib), typell-eclexia (3), typell-affinescript (3), typell-ephapax (3), typell-wokelang (3), typell-tangle (3), typell-betlang (3), typell-mylang (3), typell-oblibeny (3), typell-jtv (3), typell-phronesis (3), typell-errorlang (3), typell-vcl (4) + 3 Idris2 ABI |
| **Unit tests (inline)** | 106 | In typell-core inline module tests |
| **Integration tests** | 95 | core_comprehensive_tests.rs |
| **E2E tests** | 15 | e2e_test.rs — full pipeline, all type disciplines |
| **Property tests** | 9 | property_test.rs |
| **Benchmarks** | 262 lines | typell_bench.rs — Criterion benchmarks for unification, inference, pipeline |

**Total verified: 225 tests passing, 0 failing.**

## What's Missing

### P2P Tests
- [ ] No tests for cross-language bridge correctness (e.g., typell-wokelang checks match wokelang semantics)
- [ ] No tests for bridge interoperability (two language backends against same type)

### E2E Tests
- [ ] No test for language backend integration with actual language compilers (requires those compilers installed)

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
- **13 language backends (eclexia through vcl) each have only ~10 inline tests** -- thin coverage per backend
- **0 E2E for a 10-level type system** -- can't verify the type system actually works on real programs
- **core_comprehensive_tests.rs (95 tests) is solid** for the kernel

## Priority: P1 (HIGH) -- inline tests are decent but need E2E, benchmarks, and soundness proofs

## FAKE-FUZZ ALERT

- `tests/fuzz/placeholder.txt` is a scorecard placeholder inherited from rsr-template-repo — it does NOT provide real fuzz testing
- Replace with an actual fuzz harness (see rsr-template-repo/tests/fuzz/README.adoc) or remove the file
- Priority: P2 — creates false impression of fuzz coverage
