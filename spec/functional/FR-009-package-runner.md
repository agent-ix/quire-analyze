---
id: FR-009
title: "Run analysis over a QSL contract package and write a report"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
  - target: ix://agent-ix/engineering-assurance/FR-029
    type: depends_on
---
# FR-009: Run analysis over a QSL contract package and write a report

## Description

The `quire-analyze` CLI shall provide a `run-package` subcommand that runs one differential consistency
analysis over one QSL package in its lowered form and publishes one `quire.differential-report/v2`
report, so a caller obtains a report and a pass/fail exit status from a single invocation. The
subcommand runs the Z3 and cvc5 solvers only through `producer-execute`.

## Input: the QSL package as a contract package

QSL source (in fences of spec documents or in standalone files) is parsed and checked by
`quire-spec-language` into a checked package (for example `examples/checked-handoff/model.semantic-ir.json`
in that repository). `quire-contract-ir` lowers a checked package (`CheckedPackageV2`, lowering in
`quire-contract-model`) into the target-neutral contract package `quire.contract-package/v1`.

The runner input `--package FILE` is that `quire.contract-package/v1` document, admitted by the strict
reader `ContractPackageV1::read` of `quire-contract-model`. Neither the contract package nor its
reader names an analysis request: the package holds a semantic graph, a source map and per-request
lowering accounting, not a request to analyse. This requirement therefore fixes the request: one
consistency request over the conjunction of every analysable clause of the package, with no assumptions
and no binding groups. One invocation yields exactly one request, which is what the one-request report of
[FR-005](./FR-005-evidence-cli.md) holds. Other analysis kinds, assumption selection and binding groups
are out of scope. The gaps between this input and the existing analysis API are listed under
Dependencies.

## Producer-execute

`producer-execute` is the `engineering-assurance producer-execute` command, the bounded
producer-execution component of the `engineering-assurance` repository. Its contract is
`FR-019` (bounded producer execution) and `FR-029` (the command) of that repository, and its request and
result protocols are `engineering-assurance.producer-execution-request/v1` and
`engineering-assurance.producer-execution-result/v1`. This spec cites that contract and does not restate
it. `quire-analyze` is the runner that executes the solvers; the earlier boundary that Quire and Quoin
do not execute a producer concerns those components, and `run-package` is not one of them.

## Inputs

- `--package FILE`: the `quire.contract-package/v1` document.
- `--z3 PATH` and `--cvc5 PATH`: the solver executables, each an absolute path.
- `--producer-execute PATH`: the absolute path of the `engineering-assurance` executable; it is never
  searched for on `PATH`.
- `--output NEW_REPORT.json`: the report destination, which does not exist beforehand.

Solver limits are `AdapterLimits::default()` of [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md).

## Outputs

- One report in the `quire.differential-report/v2` contract of [FR-005](./FR-005-evidence-cli.md),
  rendered by the library renderer and published by the FR-005 publication discipline. The runner
  neither converts from nor accepts any other report version.
- Exit status: `0` satisfied, `1` refuted, `2` usage or input error, `3` non-conclusive, `4`
  publication or executor failure.
- Human diagnostics on stderr only.

## Behavior

- When invoked with all five arguments and an admitted package, the runner shall build one consistency
  request over the analysable clauses of the package.
- The runner shall lower the request, run each solver through one `producer-execute` invocation for the
  query and one for its version, and render the report with the library renderer.
- The runner shall not start a solver by any means other than `producer-execute`.
- For each invocation the runner shall send a `producer-execution-request/v1` document naming the
  absolute solver path as the producer executable, the engine's query or version arguments, the lowered
  query as the stdin input artifact, an explicit empty environment, a budget taken from the solver
  limits, and the built-in process-evidence response binding with exit codes `Any`.
- When `producer-execute` returns state `completed`, the runner shall take the solver's exit code,
  stdout and stderr from the retained process evidence.
- If the result state is `unavailable`, `timed_out`, `failed`, `refused`, `malformed_response`,
  `containment_failure` or `cancelled`, the runner shall record the solver outcome as non-conclusive
  and shall not treat it as `sat` or `unsat`.
- If `producer-execute` cannot be started, exits with status `2`, or prints a result the runner cannot
  decode, the runner shall exit `4`, name the cause on stderr and write no report.
- When the agreed status is `satisfied`, the runner shall exit `0` and record `satisfied` in the report.
- When the report records a conclusive agreement on `refuted`, the runner shall exit `1`.
- If the differential disposition is not agreement, the runner shall publish the report retaining both
  engine records and exit `3`.
- When an argument is unknown, repeated, missing or lacks a value, the runner shall exit `2`, print the
  usage on stderr and write no report.
- When the package file is unreadable, is refused by the reader, or contains no analysable clause, the
  runner shall exit `2`, name the cause on stderr and write no report.
- When a solver or `producer-execute` path is relative, empty, non-UTF-8 or over the path bound, the
  runner shall exit `2`, name the cause on stderr and write no report.
- When the destination exists, its parent directory is missing or unwritable, or publication fails
  ahead of the atomic rename, the runner shall exit `4` and leave the destination unmodified.
- If publication fails with `published-durability-unknown`, the runner shall exit `4` and report that
  state on stderr.
- The runner shall produce byte-identical report bytes for identical package bytes, identical solver
  executables and identical limits.
- The runner shall write no file other than the staged and published report.
- The runner shall modify no developer-owned file as defined in [FR-005](./FR-005-evidence-cli.md).

## Constraints

| ID | Constraint | Type | Validation |
|---|---|---|---|
| FR-009-CON-1 | The runner shall emit `quire.differential-report/v2` as defined by [FR-005](./FR-005-evidence-cli.md) and `schemas/differential-report-v2.schema.json`; this requirement leaves that schema unchanged. | Interface | Inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-009-AC-1 | With a stub `producer-execute` that records every request it receives, a `run-package` run issues exactly two requests per engine (version, query), each a valid `producer-execution-request/v1` naming the configured absolute solver path, an empty environment and the built-in response binding, and no solver process is started by any other path. | Test |
| FR-009-AC-2 | With a stub `producer-execute` returning `completed` results in which both solvers answer `sat` with a model that replays against the assertions of consistent clauses, `run-package` exits `0` and writes a report that `validate_report_document` accepts with agreed status `satisfied`. | Test |
| FR-009-AC-3 | With results in which both solvers answer `unsat` for contradictory clauses, `run-package` exits `1` and the report records agreed status `refuted`. | Test |
| FR-009-AC-4 | For each of the states `unavailable`, `timed_out`, `failed`, `refused`, `malformed_response`, `containment_failure` and `cancelled` returned for one engine, and for `sat` against `unsat` across engines, `run-package` exits `3` and the report retains both engine records. | Test |
| FR-009-AC-5 | A `producer-execute` executable that cannot be started, exits `2`, or prints an undecodable result makes `run-package` exit `4` with the cause on stderr and no report file. | Test |
| FR-009-AC-6 | A missing package file, a file `ContractPackageV1::read` refuses, a package with no analysable clause, and a file over the reader's byte limit each exit `2`, write no report and name the cause on stderr. | Test |
| FR-009-AC-7 | An unknown, repeated or missing argument and an option without a value each exit `2` with the usage on stderr and no report. | Test |
| FR-009-AC-8 | A relative, empty, non-UTF-8 or over-long solver path and a relative `producer-execute` path each exit `2` and write no report. | Test |
| FR-009-AC-9 | An existing `--output` file, a missing parent directory and an unwritable parent directory each exit `4`; an existing destination keeps its bytes. | Test |
| FR-009-AC-10 | A unit test of the runner function, injecting a directory-sync failure after rename through the publication seam of [TC-008](../test/TC-008-library-cli-atomic.md), shows the runner maps the `published-durability-unknown` publication error to exit `4` with that state named on stderr. | Test |
| FR-009-AC-11 | Two runs over the same package bytes, executables and stub results write byte-identical report files. | Test |
| FR-009-AC-12 | After a run the package file and its directory hold no added, removed or modified entry, and no file outside the report destination directory is created. | Test |
| FR-009-AC-13 | `run-package --help` lists the five arguments and the exit classes `0` to `4`. | Inspection |
| FR-009-AC-14 | A run with the real `engineering-assurance` executable and real Z3 and cvc5 on a lowered fixture exits `0` or `1` and its report validates; the test is skipped with a stated reason when either solver or the executable is absent. | Test |

## Dependencies

 [FR-005](./FR-005-evidence-cli.md) for report rendering, validation and publication;
`engineering-assurance` FR-019 and FR-029 for `producer-execute`; `quire-contract-ir` for
`quire.contract-package/v1` and `ContractPackageV1::read`; `quire-spec-language` for the QSL checked
package that is lowered to the contract package.

### Gaps

The following gaps were measured against the current code. They are preconditions of an implementing
change, and none is closed by invented behaviour here.

1. No QSL-to-contract-package step exists in `quire-spec-language`: its `qsl-package` sources and tests
   do not call the `quire-contract-ir` lowering, and the repository carries a checked package
   (`examples/checked-handoff/model.semantic-ir.json`) but no `quire.contract-package/v1` document.
   Until it does, the contract-package fixture is produced at test time by lowering the checked package
   with `quire-contract-model`.
2. The analysis API takes `StatementInput` built from `ContractPackage<TypedExpression>` and a
   `DeclarationEnvironment`. `ContractPackageV1` holds a semantic graph of checked terms. No bridge
   exists from the second to the first, and no definition says which graph nodes are analysable clauses.
   The runner needs that definition and bridge.
3. A contract package declares no analysis request, so this requirement fixes the single consistency
   request. A declaring field would have to come from the contract-package owner.
4. `execute_solver` in `src/solver.rs` runs a solver process directly. `run-package` runs solvers through
   `producer-execute` instead; whether the library path is migrated is outside this requirement.
5. `producer-execute` is a separate executable. The runner depends on it through a process boundary
   only, with no Cargo dependency on `engineering-assurance`.
