---
id: FR-009
title: "Run analysis over a contract package and write a report"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-009: Run analysis over a contract package and write a report

## Description

The `quire-analyze` CLI shall provide a `run-package` subcommand that runs one differential consistency
analysis over one contract package document and publishes one report, so a caller obtains a report and
a pass/fail exit status from a single invocation.

The input artifact is an executable projection, format `quire.contract.executable-projection/v1`
(schema `contract-executable-projection-v1.schema.json`), owned by `quire-contract-ir` and decoded by
`quire_contract_model::BoundPackage::from_json_bytes`. The decoder yields the bound executable clauses
with their declaration environments and typed expressions. A projection declares no analysis request, so
the request is fixed by this requirement: consistency of the conjunction of every bound executable
clause, with no assumptions and no binding groups. One invocation therefore yields exactly one request,
which is what the one-request report of [FR-005](./FR-005-evidence-cli.md) holds. Other analysis kinds,
assumption selection and binding groups are out of scope.

## Inputs

- `--package FILE`: the executable projection document.
- `--z3 PATH` and `--cvc5 PATH`: both required solver executables. Their limits are
  `AdapterLimits::default()`; there is no search of `PATH` and no environment-selected solver.
- `--output NEW_REPORT.json`: the report destination, which does not exist beforehand.

## Outputs

- One report in the `quire.differential-report/v2` contract of [FR-005](./FR-005-evidence-cli.md),
  rendered by the library renderer and published by the FR-005 publication discipline.
- Exit status using the classes of `publish-report`: `0` satisfied, `1` refuted, `2` usage or input
  error, `3` non-conclusive, `4` publication failure.
- Human diagnostics on stderr only.

## Behavior

- When invoked with all four arguments and a decodable package, the runner shall build one consistency
  request over the bound clauses of the package.
- The runner shall lower the request, execute the differential run with the configured solvers, and
  render the report with the library renderer.
- The runner shall not build report JSON by any other means.
- When the agreed status is `satisfied`, the runner shall exit `0` and record `satisfied` in the report.
- When the report records a conclusive agreement on `refuted`, the runner shall exit `1`.
- If the differential disposition is not agreement, the runner shall publish the report retaining both
  engine records and exit `3`.
- When an argument is unknown, repeated, missing or lacks a value, the runner shall exit `2`, print the
  usage on stderr and write no report.
- When the package file is unreadable, exceeds the 16,777,216-byte `MAX_CONFORMANCE_FILE_BYTES` bound of `quire-contract-model`, or is refused by the decoder, or a
  solver path is invalid for `SolverConfig::new`, the runner shall exit `2`, name the cause on stderr and
  write no report.
- When the package binds no executable clause, the runner shall exit `2` and write no report.
- When the destination exists, its parent directory is missing or unwritable, or publication fails
  ahead of the atomic rename, the runner shall exit `4` and leave the destination unmodified.
- If publication fails with `published-durability-unknown`, the runner shall exit `4` and report that
  state on stderr.
- The runner shall produce byte-identical report bytes for identical package bytes, identical solver
  executables and identical limits.
- The runner shall write no file other than the staged and published report.
- The runner shall modify no developer-owned file as defined in [FR-005](./FR-005-evidence-cli.md).
- The runner shall run solvers only with the bounds of [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-009-CON-1 | The runner shall emit `quire.differential-report/v2` as defined by [FR-005](./FR-005-evidence-cli.md) and `schemas/differential-report-v2.schema.json`; this requirement leaves that schema unchanged. | Interface | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-009-AC-1 | With stub solvers that both answer `sat` and emit a model that replays against the assertions of a projection of consistent clauses, `run-package` exits `0` and writes a report that `validate_report_document` accepts with agreed status `satisfied`. | Test |
| FR-009-AC-2 | With stub solvers that both answer `unsat` over a projection of contradictory clauses, `run-package` exits `1` and the report records agreed status `refuted`. | Test |
| FR-009-AC-3 | With one stub answering `sat` and the other `unsat`, with one solver absent, and with one stalled solver, `run-package` exits `3` and the report retains both engine records. | Test |
| FR-009-AC-4 | A missing package file, a file the decoder refuses, a file over 16,777,216 bytes and a projection with no executable clause each exit `2`, write no report file and name the cause on stderr. | Test |
| FR-009-AC-5 | An unknown argument, a repeated argument, a missing required argument and an option without a value each exit `2` with the usage on stderr and no report. | Test |
| FR-009-AC-6 | A relative solver path, an empty solver path, a non-UTF-8 path and a path longer than the `AdapterLimits` path bound each exit `2` and write no report; an absolute path naming no executable is not a configuration error and takes the unavailable route of FR-009-AC-3 (exit `3`); an omitted `--z3` or `--cvc5` is an argument error under FR-009-AC-5. | Test |
| FR-009-AC-7 | An existing `--output` file, a missing parent directory and an unwritable parent directory each exit `4`; an existing destination keeps its bytes. | Test |
| FR-009-AC-8 | A unit test of the runner function, injecting a directory-sync failure after rename through the publication seam of [TC-008](../test/TC-008-library-cli-atomic.md), shows the runner maps the `published-durability-unknown` publication error to exit `4` with that state named on stderr. | Test |
| FR-009-AC-9 | Two runs over the same package bytes and solver executables write byte-identical report files. | Test |
| FR-009-AC-10 | After a run the package file and its directory hold no added, removed or modified entry, and no file outside the report destination directory is created. | Test |
| FR-009-AC-11 | `run-package --help` lists the four arguments and the exit classes `0` to `4`. | Inspection |

## Dependencies

[FR-005](./FR-005-evidence-cli.md) for report rendering, validation and publication, and
`quire-contract-ir` for the executable projection and its decoder. QSL tooling at
`quire-spec-language` does not emit this projection (its `examples/checked-handoff/` carries a checked
package, a different artifact), so this requirement owns no QSL fixture.
