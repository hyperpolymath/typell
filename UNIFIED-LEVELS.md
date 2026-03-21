// SPDX-License-Identifier: PMPL-2.0-or-later
= Unified Level System — TypeLL / VQL-UT / VeriSimDB / TypedQLiser / PanLL
:toc:

== Level Definitions

[cols="1,3,3"]
|===
| Level | Name | Criteria

| L0 | Scaffold | Repo exists, no functional code
| L1 | Spec | Specifications/types defined, no runtime
| L2 | Partial | Some code works standalone, not connected
| L3 | Core | Main engine works, tests pass, standalone
| L4 | Integrated | Connected to adjacent systems, data flows
| L5 | End-to-end | Full pipeline works across all systems
| L6 | Tested | Point-to-point, end-to-end, aspect tests + benchmarks
| L7 | Production | Published, documented, CI green, ready for users
|===

== Current State (2026-03-21)

[cols="2,1,3"]
|===
| Component | Level | Notes

| TypeLL core engine | L1 | Specs exist, no runtime type checker
| TypedQLiser L5-10 | L1 | Levels 1-4 work, 5-10 need TypeLL
| PanLL VQL panel | L0 | Does not exist
| VQL-UT → VeriSimDB bridge | L2 | Parser works, no DB execution
| VeriSimDB octad storage | L4 | Phase 4 validated, production tested
| PanLL TypeLL panel | L2 | ReScript UI exists, not verified running
| TypeLL-VQL bridge | L2 | Crate exists, depends on typell-core
|===

== Level-Up Plan

Round 1: Bring trailing (L0/L1) → L3::
  1. TypeLL core engine: implement type checker runtime
  2. PanLL VQL panel: create the panel
  3. TypedQLiser L5-10: implement with TypeLL backing

Round 2: Bring all → L4 (integrated)::
  4. Wire VQL-UT → VeriSimDB runtime bridge
  5. Wire TypeLL → TypedQLiser for levels 5-10
  6. Wire PanLL TypeLL panel → running TypeLL server
  7. Wire PanLL VQL panel → VQL-UT

Round 3: Bring all → L5 (end-to-end)::
  8. PanLL → TypeLL → VQL-UT → VeriSimDB pipeline test
  9. TypedQLiser → TypeLL → VQL-UT type checking pipeline

Round 4: Bring all → L6 (tested + benchmarked)::
  10. Point-to-point tests for every connection
  11. End-to-end tests for full pipelines
  12. Aspect tests (error handling, edge cases, concurrency)
  13. Benchmarks (latency, throughput, memory)
