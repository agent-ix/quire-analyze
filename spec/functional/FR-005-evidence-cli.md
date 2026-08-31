---
id: FR-005
title: "Provide evidence reports, differential checks, and CLI"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-004
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface-001
    type: implements
---
# FR-005: Provide evidence reports, differential checks, and CLI

## Description

The library and CLI shall produce the same normalized analysis report, derivation envelope, and
stable exit classification, and shall retain cross-engine discrepancies without hiding either result.

## Behavior

- Reports bind request, package, schema, encoding, query, assertion map, engine, configuration,
  raw-response, normalized-result, counterexample, producer, and output digests.
- Differential runs retain both engine records before computing agreement.
- Disagreement, unavailable comparison, or unverified model remains non-conclusive for a
  cross-engine assurance claim.
- CLI output is deterministic JSON; human-readable diagnostics go to stderr and do not alter it.
- Output publication is all-or-nothing and never edits developer-owned files.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-005-AC-1 | Library and CLI reports are semantically and byte equivalent for identical inputs. | Test (TC-008) |
| FR-005-AC-2 | Every report validates against its versioned schema and PGM-01 evidence envelope. | Test (TC-007) |
| FR-005-AC-3 | Differential disagreement retains both results and cannot be reported as agreement. | Test (TC-006) |
| FR-005-AC-4 | Failed publication leaves no partial result or modified developer-owned file. | Test (TC-008) |

## Dependencies

FR-004 and native issue #5.
