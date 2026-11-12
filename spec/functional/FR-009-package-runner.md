---
id: FR-009
title: "Run analysis over a QSL package and write a report"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-009: Run analysis over a QSL package and write a report

## Description

The `quire-analyze` CLI shall provide a `run-package` subcommand that executes analysis over one QSL
package directory and publishes one differential report for it, so a caller obtains a report and a
pass/fail exit status from a single invocation. The package layout and contracts are owned by the
`quire-spec-language` repository (package code under `qsl-package/src/`, fixtures
`artifacts/compiled-protocol-v1`, `artifacts/compiled-protocol-v2` and `examples/protocol-handoff/`);
this spec cites them and does not copy them.

## Inputs

- `--package DIR`: a QSL package directory on the caller's filesystem.
- `--output NEW_REPORT.json`: the report destination, which must not already exist.
- `--solver-config FILE`: optional caller-supplied solver executable paths and limits; when omitted
  the configured defaults of the library apply.

## Outputs

- One report file at `--output`, published by the publication discipline of [FR-005](./FR-005-evidence-cli.md).
- Exit status using the classes of the existing `publish-report` subcommand: `0` satisfied, `1`
  refuted, `2` usage or input error, `3` non-conclusive (disagreement, unavailable or inconclusive),
  `4` publication failure.
- Human diagnostics on stderr only.

## Behavior

- When invoked with a readable package directory, the runner shall load the package, derive the
  analysis requests it declares, run them through the library, and render the authoritative report bytes
  with the library renderer; it shall not hand-build report JSON.
- When the package cannot be loaded or declares no analysable request, the runner shall exit `2`,
  name the cause on stderr, and write no report.
- The runner shall return exit status `0` only when the report is a conclusive agreement that every
  declared request is satisfied, and `1` when any request is conclusively refuted.
- If any request is non-conclusive, the runner shall still publish the report retaining both engine
  records and exit `3`; it shall not downgrade non-conclusive to `0` or `1`.
- Two runs over identical package content and solver configuration shall produce byte-identical reports.
- The runner shall never write inside the package directory or any developer-owned file.
- The runner shall invoke only the solver executables named by configuration, with the bounds of
  [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md).

## Producer-execute invocation contract

`producer-execute` is defined here as the invocation contract through which a caller-controlled step
executes this runner as a producer. It is a contract, not a separate binary or subcommand.

- The party that executes is always the caller's own step (for example `make assurance-inputs`, or any
  CI job). That step spawns `quire-analyze run-package` as a child process with explicit arguments
  and reads only its exit status, its report file and its stderr.
- The runner is a non-interactive, one-shot process: it reads no stdin, makes no network calls, and
  needs no environment variable to select behavior.
- Quire and Quoin shall not spawn the runner. They consume the published report as an artifact, which
  keeps the existing boundaries that Quire exports and never executes a producer and Quoin
  transcribes and never executes one.
- Any caller step that reports "invoked through producer-execute" shall use exactly the arguments and
  exit classes above, and shall treat exit `0` and `1` as conclusive results and `2`, `3`, `4` as
  non-passing.

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-009-CON-1 | The runner shall not define or alter the report schema; it emits the differential report contract named by [FR-005](./FR-005-evidence-cli.md). | Interface | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-009-AC-1 | Running `run-package` over the `artifacts/compiled-protocol-v1` fixture package writes a report that passes the same validation as `publish-report` and exits with the class matching its agreed status. | Test |
| FR-009-AC-2 | The same run over `artifacts/compiled-protocol-v2` and `examples/protocol-handoff/` each writes a valid report; the three exit statuses are asserted individually. | Test |
| FR-009-AC-3 | A package whose requests are conclusively satisfied exits `0`; one with a refuted request exits `1`; a fixture with an unavailable or disagreeing engine exits `3` and the report retains both engine records. | Test |
| FR-009-AC-4 | A missing directory, an unloadable package, and a package declaring no request each exit `2`, write no report file, and name the cause on stderr. | Test |
| FR-009-AC-5 | A pre-existing `--output` destination is refused with exit `4` and its bytes are unchanged. | Test |
| FR-009-AC-6 | Two runs over the same package and configuration yield byte-identical report files. | Test |
| FR-009-AC-7 | After a run the package directory tree has no added, removed or modified file. | Test |
| FR-009-AC-8 | The runner started as a child process with closed stdin, no network and an empty environment apart from path resolution produces the same report and exit status as a normal run. | Test |
| FR-009-AC-9 | The producer-execute definition above is stated in this spec and `quire-analyze --help` for `run-package`, and no Quire or Quoin code path spawns `run-package`. | Inspection |

## Dependencies

[FR-005](./FR-005-evidence-cli.md) for report rendering, validation and publication; the QSL package
contracts owned by `quire-spec-language`.
