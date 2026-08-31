---
id: TM-001
title: "Contract analysis v0.1 test matrix"
type: TestMatrix
---
# Contract analysis v0.1 test matrix

## Functional Requirement Coverage

| Functional Req | Acceptance Criteria | Test Cases | Coverage Status |
|---|---|---|---|
| FR-001 | FR-001-AC-1, FR-001-AC-2 | TC-001 | 🚧 Planned |
| FR-001 | FR-001-AC-3 | TC-002 | 🚧 Planned |
| FR-001 | FR-001-AC-4 | TC-003 | 🚧 Planned |
| FR-002 | FR-002-AC-1 | TC-002 | 🚧 Planned |
| FR-002 | FR-002-AC-2, FR-002-AC-4 | TC-003 | 🚧 Planned |
| FR-002 | FR-002-AC-3 | TC-004 | 🚧 Planned |
| FR-003 | FR-003-AC-1, FR-003-AC-2 | TC-005 | 🚧 Planned |
| FR-003 | FR-003-AC-3 | TC-006 | 🚧 Planned |
| FR-003 | FR-003-AC-4 | TC-007 | 🚧 Planned |
| FR-004 | FR-004-AC-1 | TC-001 | 🚧 Planned |
| FR-004 | FR-004-AC-2 | TC-006 | 🚧 Planned |
| FR-004 | FR-004-AC-3 | TC-004 | 🚧 Planned |
| FR-004 | FR-004-AC-4 | TC-007 | 🚧 Planned |
| FR-005 | FR-005-AC-1, FR-005-AC-4 | TC-008 | 🚧 Planned |
| FR-005 | FR-005-AC-2 | TC-007 | 🚧 Planned |
| FR-005 | FR-005-AC-3 | TC-006 | 🚧 Planned |

## Nonfunctional and Stakeholder Coverage

| Requirement | Acceptance Criteria | Test Cases | Coverage Status |
|---|---|---|---|
| StR-001 | StR-001-VC-1 | TC-002, TC-007 | 🚧 Planned |
| StR-001 | StR-001-VC-2 | TC-006 | 🚧 Planned |
| NFR-001 | NFR-001-AC-1 | TC-002 | 🚧 Planned |
| NFR-001 | NFR-001-AC-2, NFR-001-AC-3 | TC-005 | 🚧 Planned |
| NFR-002 | NFR-002-AC-1, NFR-002-AC-3 | TC-007 | 🚧 Planned |
| NFR-002 | NFR-002-AC-2 | TC-005 | 🚧 Planned |
| NFR-002 | NFR-002-AC-4 | Inspection | 🚧 Planned |

## Test Case Summary

| Test ID | Title | Type | Priority | Traces To | Status |
|---|---|---|---|---|---|
| TC-001 | Analysis algebra and truth table | Analysis | P0 | FR-001, FR-004 | 🚧 Planned |
| TC-002 | Canonical requests and deterministic lowering | Property | P0 | FR-001, FR-002, NFR-001 | 🚧 Planned |
| TC-003 | Capability rejection | Integration | P0 | FR-001, FR-002 | 🚧 Planned |
| TC-004 | Injective symbols and source mapping | Property | P0 | FR-002, FR-004 | 🚧 Planned |
| TC-005 | Adapter resource and failure isolation | Integration | P0 | FR-003, NFR-001, NFR-002 | 🚧 Planned |
| TC-006 | Engine parity and counterexamples | Analysis | P0 | FR-003, FR-004, FR-005 | 🚧 Planned |
| TC-007 | Evidence integrity | Integration | P0 | FR-003, FR-004, FR-005, NFR-002 | 🚧 Planned |
| TC-008 | Library/CLI parity and atomic output | Integration | P0 | FR-005 | 🚧 Planned |

All semantic rows remain planned until the corresponding native implementation issue has executable,
requirement-tagged tests and retained evidence. The placeholder crate tests count only as scaffold
health and satisfy no row. A row may become complete only when its entire acceptance scope runs;
ignored, skipped, unavailable, or platform-deferred cases remain visible and not complete.

## QA Coverage Obligations

The final campaign must include unit, property, model-based, mutation, fuzz, fault-injection,
integration, differential, reproducibility, MSRV, supported-platform, schema, license, unsafe-code,
and documentation gates. Line coverage is diagnostic, not a sufficiency claim. The decisive coverage
metric is completed acceptance criteria backed by named executable tests and retained outcomes.
