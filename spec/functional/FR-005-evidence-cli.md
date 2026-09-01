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
- Disagreement, unavailable comparison, or unverified model makes the differential result
  non-conclusive; it cannot become conclusive until a human-reviewed adjudication is retained.
- Every filed semantic defect receives a stable regression fixture that reproduces the pre-fix
  discrepancy and passes only with its reviewed disposition.
- CLI output is deterministic JSON; human-readable diagnostics go to stderr and do not alter it.
- Output publication is all-or-nothing and never edits developer-owned files.
- Linux publication does not report success until the staged file and parent directory have both
  been synchronized. A failure before atomic rename reports `destination-unmodified`; a parent
  directory sync failure after rename reports `published-durability-unknown`, because complete
  destination bytes can already be visible. Callers do not delete or retry that state as though the
  destination were absent.
- Runtime product output uses `quire.analysis-report/v1` and
  `quire.differential-report/v1`. Canonical JSON is UTF-8, compact, recursively key-sorted, and has
  no insignificant whitespace. `reportDigest` is SHA-256 over the canonical object with that field
  omitted; validation re-derives every raw-stream and report digest.
- Differential status is the closed set `agreement`, `disagreement`, `unavailable`, and
  `inconclusive`. Agreement is conclusive only when both exact engines return the same conclusive
  status and every required sat model is replay-verified. Different conclusive statuses are
  disagreement; a missing engine is unavailable; every other combination is inconclusive.
- The Linux-x86_64 conformance pins are Z3 5.1.0 from the official
  `z3_solver-5.1.0.0-py3-none-manylinux_2_27_x86_64.whl` asset (archive SHA-256
  `dfad9e309d7010b1ff6bdb33f21570a1603ef4727373221c7117a74448f0cfef`) and cvc5 1.3.4 from
  `cvc5-Linux-x86_64-static.zip` (archive SHA-256
  `dcdbfada0ce493ee98259c0816e0daafc561c223aadb3af298c2968e73ea39c6`). Executable SHA-256 and
  complete normalized version output are recorded after extraction and remain adapter pins.
- The v1 library executes caller-prepared queries with caller-supplied exact solver pins and renders
  the authoritative report bytes. The v1 CLI validates and publishes those exact bytes; it does not
  reconstruct authoritative contract packages or solver configurations from JSON. Publication
  creates a new destination through a same-directory temporary file, sync, and atomic rename; an
  existing destination is a developer-owned file and is refused. Recoverable pre-rename failures
  close and remove their staging file and synchronize that removal. An uncatchable process
  termination can leave a private staging file but never exposes partial destination bytes; the
  file name identifies it as non-authoritative staging rather than a report.
- Assurance-run transcription/audit uses Quoin 0.22.5. Until `quire-contract-ir#20` selects the
  shared PGM-01 envelope and integrity component, its status is `unavailable`; no report or local
  gate may translate that absence into schema validation success.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-005-AC-1 | CLI publication preserves the authoritative library report byte-for-byte and returns its stable exit class. | Test (TC-008) |
| FR-005-AC-2 | Every report validates against its versioned schema and PGM-01 evidence envelope. | Test (TC-007) |
| FR-005-AC-3 | Differential disagreement retains both results and cannot be reported as agreement. | Test (TC-006) |
| FR-005-AC-4 | Failed publication leaves no partial result or modified developer-owned file. | Test (TC-008) |
| FR-005-AC-5 | Every filed semantic defect maps to an executable retained regression fixture. | Inspection and test (TC-006) |

## Dependencies

FR-004 and native issue #5.
