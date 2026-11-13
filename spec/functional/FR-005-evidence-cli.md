---
id: FR-005
title: "Provide evidence reports, differential checks, and CLI"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-004
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface_001
    type: implements
---
# FR-005: Provide evidence reports, differential checks, and CLI

## Description

The library and CLI shall produce the same normalized analysis report, derivation envelope, and
stable exit classification, and shall retain cross-engine discrepancies without hiding either result.

## Behavior

- Reports retain request, package, schema, encoding, canonical query identity, assertion map,
  engine, observed solver version, configured executable path and limits, raw response,
  normalized result, counterexample, and producer. The canonical proof-input identities bind
  the reported conclusion to the exact selected clauses, bindings, request, and query bytes.
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
- The analysis product contract names `quire.analysis-report/v1`; the differential runtime
  report emits `quire.differential-report/v2`. Canonical JSON is UTF-8, compact, recursively
  key-sorted, and has no insignificant whitespace. Validation re-derives canonical proof-input
  identities against retained material and the authoritative query where supplied; it does not
  add a hash of already retained raw streams or the report itself.
- The `quire.differential-report/v2` closed contract removes the duplicated
  `contractIrRevision` field. It retains the producer application name and version, complete clause
  and source identities with canonical clause digests, request and query identities, exact query
  bytes, and raw solver-stream bytes. The engine record retains its observed executable path and
  version only; it has no executable byte count or SHA, expected version or SHA, or configuration
  digest. The closed outcome set excludes executable-digest-mismatch, executable-changed, and
  version-mismatch because no pin or digest check exists.
  Dependency selection and resolution remain in `Cargo.toml` and `Cargo.lock`; their source revision
  is not copied into report metadata. The renderer and validator accept the v2 contract only, with
  no v1 fallback or compatibility reader. This change does not make the PGM-01 envelope available.
- Differential status is the closed set `agreement`, `disagreement`, `unavailable`, and
  `inconclusive`. Agreement is conclusive only when both exact engines return the same conclusive
  status and every required sat model is replay-verified. Different conclusive statuses are
  disagreement; a missing engine is unavailable; every other combination is inconclusive.
- The v1 library executes caller-prepared queries with caller-supplied solver paths and limits and
  renders the authoritative report bytes. The `publish-report` subcommand validates and publishes those exact bytes; it does not
  reconstruct authoritative contract packages or solver configurations from JSON. The `run-package`
  subcommand of [FR-009](./FR-009-package-runner.md) is the CLI surface that decodes a package and solver paths. Publication
  creates a new destination through a same-directory temporary file, sync, and atomic rename; an
  existing destination is a developer-owned file and is refused. Recoverable pre-rename failures
  close and remove their staging file and synchronize that removal. An uncatchable process
  termination can leave a private staging file but never exposes partial destination bytes; the
  file name identifies it as non-authoritative staging rather than a report.
- Until `quire-contract-ir#20` selects the shared PGM-01 envelope and integrity component, its
  status is `unavailable`; no report or local gate may translate that absence into schema
  validation success.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-005-AC-1 | CLI publication preserves the authoritative library report byte-for-byte and returns its stable exit class. | Test (TC-008) |
| FR-005-AC-2 | Every report validates against its versioned schema and PGM-01 evidence envelope. | Test (TC-007) |
| FR-005-AC-3 | Differential disagreement retains both results and cannot be reported as agreement. | Test (TC-006) |
| FR-005-AC-4 | Failed publication leaves no partial result or modified developer-owned file. | Test (TC-008) |
| FR-005-AC-5 | Every filed semantic defect maps to an executable retained regression fixture. | Inspection and test (TC-006) |
| FR-005-AC-6 | The closed `quire.differential-report/v2` schema requires `producer.name` and `producer.version`; every `assertions[].clause`, `assertions[].source`, and `assertions[].clauseDigest`; `bindingSetDigest`, `requestDigest`, `queryDigest`, and `queryHex`; and every engine's `configuredExecutable`, observed `identity.path` and `identity.version` when available, `stdoutHex`, `stderrHex`, and `modelHex`. It refuses `identity.bytes`, `identity.sha256`, `limits.executableBytes`, `expectedVersion`, `expectedExecutableSha256`, `configurationDigest`, `querySha256`, `stdoutSha256`, `stderrSha256`, `modelSha256`, and `reportDigest`, and the three removed mismatch outcomes. Standalone v2 validation derives `queryDigest` from retained `requestDigest` and query bytes. Validation bound to the authoritative `QueryBundle` and differential run rejects a report whose clause/source identities, clause/binding-set/request identities, or raw solver output differ from the authoritative reconstruction; standalone validation does not claim to derive those values from absent source content or authenticate self-consistent substituted raw output. Both validation paths reject missing named fields, `contractIrRevision`, and every `quire.differential-report/v1` document, with no v1 fallback. The closed schema requires `encodingProfile` `quire.smtlib2/v2` and a `logic` equal to the logic the query's feature set requires (`QF_LIA` for a query without datatype declarations; otherwise the logic FR-020 specifies), and validation refuses a report that names `quire.smtlib2/v1` or `QF_UF`, with no fallback. `assertions[]` lists the named assertions of the query only; integer range constraints are not named assertions ([FR-010](./FR-010-linear-integer-arithmetic.md)). | Test (TC-007) |

## Dependencies

FR-004 and native issue #5.
