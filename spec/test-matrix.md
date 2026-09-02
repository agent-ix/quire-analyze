---
id: TM-001
title: "Contract analysis v0.1 test matrix"
type: TestMatrix
---
# Contract analysis v0.1 test matrix

## Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|---|---|---|---|
| FR-001 | FR-001-AC-1, FR-001-AC-2 | TC-001, TC-009 | ✅ Boolean v1 analysis complete |
| FR-001 | FR-001-AC-3 | TC-002, TC-009, TC-010 | ✅ Boolean v1 analysis complete |
| FR-001 | FR-001-AC-4 | TC-003, TC-010 | ✅ Boolean v1 analysis complete |
| FR-001 | FR-001-AC-5 | TC-002, TC-009, TC-010 | ✅ Boolean v1 analysis complete |
| FR-002 | FR-002-AC-1 | TC-002, TC-010 | ✅ Boolean v1 complete |
| FR-002 | FR-002-AC-2, FR-002-AC-4 | TC-003, TC-010 | ✅ Boolean v1 complete |
| FR-002 | FR-002-AC-3 | TC-004, TC-010 | ✅ Boolean v1 complete |
| FR-002 | FR-002-AC-5 | TC-004, TC-010, Inspection | ✅ Boolean v1 complete |
| FR-003 | FR-003-AC-1, FR-003-AC-2 | TC-005 | ✅ Linux adapter v1 complete |
| FR-003 | FR-003-AC-3 | TC-005, TC-006 | ✅ Adapter contract complete; real-engine corpus planned |
| FR-003 | FR-003-AC-4 | TC-005, TC-007 | ✅ Adapter record complete; evidence envelope planned |
| FR-003 | FR-003-AC-5 | TC-005 | ✅ Linux adapter v1 complete |
| FR-003 | FR-003-AC-6 | TC-005, Inspection | ✅ Complete |
| FR-004 | FR-004-AC-1 | TC-001 | ✅ Boolean v1 analysis complete |
| FR-004 | FR-004-AC-2 | TC-006 | ✅ Boolean v1 replay complete |
| FR-004 | FR-004-AC-3 | TC-004 | ✅ Boolean v1 mapping complete |
| FR-004 | FR-004-AC-4 | TC-007 | ✅ Incomplete-state boundary complete |
| FR-004 | FR-004-AC-5 | TC-005, TC-006 | ✅ Boolean v1 analysis complete |
| FR-005 | FR-005-AC-1, FR-005-AC-4 | TC-008 | 🚧 Fault-injected publisher complete; abrupt pre-rename residue disposition and #24 remain open |
| FR-005 | FR-005-AC-2 | TC-007 | ⛔ Application schema complete; PGM-01 blocked on contract-ir#20 |
| FR-005 | FR-005-AC-3 | TC-006 | 🚧 Differential states and real SAT/UNSAT complete; retained full corpus #24 open |
| FR-005 | FR-005-AC-5 | TC-006, Inspection | 🚧 Stable regressions present; complete retained corpus #24 open |
| FR-006 | FR-006-AC-1 | TC-011 | ✅ Shared pin classification complete |
| FR-006 | FR-006-AC-2 | TC-011 | ✅ Producer isolation and result derivation complete |
| FR-006 | FR-006-AC-3 | TC-011 | ✅ Quire static export complete |
| FR-006 | FR-006-AC-4 | TC-012 | ✅ Read-only compatibility view complete |
| FR-006 | FR-006-AC-5 | TC-011, TC-012 | ✅ Non-conclusive state separation complete |
| FR-006 | FR-006-AC-6 | TC-012 | ✅ Superseded local verifier removed after the dual run |

## Nonfunctional and Stakeholder Coverage

| Requirement | Acceptance Criteria | Test Cases | Coverage Status |
|---|---|---|---|
| StR-001 | StR-001-VC-1 | TC-002, TC-007 | 🚧 Planned |
| StR-001 | StR-001-VC-2 | TC-006 | 🚧 Planned |
| NFR-001 | NFR-001-AC-1 | TC-002, TC-010 | 🚧 Planned; request/query slice complete, evidence report pending |
| NFR-001 | NFR-001-AC-2, NFR-001-AC-3 | TC-005, TC-010 | ✅ Lowering and Linux adapter boundaries complete |
| NFR-002 | NFR-002-AC-1, NFR-002-AC-3 | TC-007 | 🚧 Planned |
| NFR-002 | NFR-002-AC-2 | TC-005 | ✅ Adapter failure states complete |
| NFR-002 | NFR-002-AC-4 | Inspection | 🚧 Planned |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---|---|---|---|---|---|
| TC-001 | Analysis algebra and truth table | Analysis | P0 | FR-001, FR-004 | ✅ Boolean v1 complete |
| TC-002 | Canonical requests and deterministic lowering | Property | P0 | FR-001, FR-002, NFR-001 | 🚧 Planned |
| TC-003 | Capability rejection | Integration | P0 | FR-001, FR-002 | 🚧 Planned |
| TC-004 | Injective symbols and source mapping | Property | P0 | FR-002, FR-004 | 🚧 Planned |
| TC-005 | Adapter resource and failure isolation | Integration | P0 | FR-003, NFR-001, NFR-002 | ✅ Linux adapter v1 complete |
| TC-006 | Engine parity and counterexamples | Analysis | P0 | FR-003, FR-004, FR-005 | 🚧 Real SAT/UNSAT and controlled states complete; #24 open |
| TC-007 | Evidence integrity | Integration | P0 | FR-003, FR-004, FR-005, NFR-002 | ⛔ Application mutations complete; PGM-01 unavailable |
| TC-008 | Library/CLI parity and atomic output | Integration | P0 | FR-005 | 🚧 Fault boundaries, concurrency, durability, and crash states covered; #24 and crash-residue signoff open |
| TC-009 | ADR-0010 identity research | Analysis | P0 | FR-001 | ✅ Complete |
| TC-010 | Exact Boolean SMT-LIB2 v1 lowering slice | Integration | P0 | FR-001, FR-002, NFR-001 | ✅ Complete |
| TC-011 | Shared assurance intake and result derivation | Integration | P0 | FR-006, NFR-002 | ✅ Complete |
| TC-012 | Read-only legacy compatibility view | Integration | P0 | FR-006, NFR-002 | ✅ Complete |

TC-001 completes the Boolean-v1 analysis algebra and independent finite truth table. TC-002 through
TC-004 and TC-006 through TC-008 remain planned as complete campaigns; issue #4 closes their exact
request, mapping, replay, and incomplete-state slices without claiming real-engine differential or
evidence publication. TC-005 completes the Linux adapter v1 boundary. TC-009 is completed architecture
research. TC-010 completes only Boolean-v1 lowering. TC-011 completes the shared assurance intake path,
producer isolation and result derivation; TC-012 completes the read-only compatibility view and its
derived fixture corpus, and the removal of the superseded local evidence verifier. The placeholder crate tests count only as
scaffold health and satisfy no row. A row may become complete only when its entire acceptance scope
runs; ignored, skipped, unavailable, or platform-deferred cases remain visible and not complete.

## QA Coverage Obligations

The final campaign must include unit, property, model-based, mutation, fuzz, fault-injection,
integration, differential, reproducibility, MSRV, supported-platform, schema, license, unsafe-code,
and documentation gates. Line coverage is diagnostic, not a sufficiency claim. The decisive coverage
metric is completed acceptance criteria backed by named executable tests and retained outcomes.
