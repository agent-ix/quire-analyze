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
`quire-spec-language` into a checked package. `quire-contract-ir` lowers a checked package
(`CheckedPackageV2::lower` in `quire-contract-model`) into the target-neutral contract package
`quire.contract-package/v1`.

The runner input `--package FILE` is that `quire.contract-package/v1` document. The runner reads it
with `ContractPackageWireV1::read(bytes, limits, dependencies)` of `quire-contract-model`, with
`limits = CheckedPackageReadLimits::bounded()` (1,048,576 canonical bytes, 10,000 nodes) and
`dependencies` empty. The reader returns `Admitted`, `Refused`, `Incomplete` or `CatalogUnavailable`:
`Admitted` proceeds, `Refused` and `Incomplete` exit `2` naming the refusal or the exhausted limit, and
`CatalogUnavailable` exits `4` because it is a reader and catalog skew, not a verdict on the input. A
package that names dependencies is refused by the empty dependency map and exits `2`.

A contract package names no analysis request: it holds a semantic graph, a source map and per-request
lowering accounting. This requirement therefore fixes the request, and no QSL-side declaring field is
needed. The analysable clauses are the nodes of `semantic_graph.nodes` of kind `claim` with form
`analysis_claim`. If a package holds none, or an `analysis_claim` has a body that is not a
Boolean-typed term, the runner exits `2`. The runner builds ONE consistency request whose selected
statements are all analysable clauses, with no assumptions and no binding groups, and no per-clause
selection. One invocation yields exactly one request, which is what the one-request report of
[FR-005](./FR-005-evidence-cli.md) holds. Other analysis kinds, assumption selection and binding groups
are out of scope. This definition closes the selection question; the translation of a term into an
analysis statement is still open (Dependencies, gap 2).

## Producer-execute

`producer-execute` is the `engineering-assurance producer-execute` command, the bounded
producer-execution component of the `engineering-assurance` repository. Its contract is
`FR-019` (bounded producer execution) and `FR-029` (the command) of that repository, and its request and
result protocols are `engineering-assurance.producer-execution-request/v1` and
`engineering-assurance.producer-execution-result/v1`. This spec cites that contract and does not restate
it. `quire-analyze` is the runner that executes the solvers.

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
- For each invocation the runner shall pass one `producer-execution-request/v1` document to
  `producer-execute --request -` on its stdin, and read its stdout.
- The runner shall fill the request fields as follows: `protocol` is the request protocol constant;
  `procedure` is `direct`; `producer.executable` is the absolute solver path and `producer.name` is
  `z3` or `cvc5`; `caller` is kind `quire-analyze.run-package`, version `1`; `arguments` are the literal
  query arguments (`-in -smt2` for Z3, `--lang=smt2 --no-incremental` for cvc5) or version arguments
  (`-version`, `--version`) of the engine; `environment` is empty; `outputs` and `outputTrees` are
  empty; `containment` is `process-group-v1` with contract kind `quire.solver-process`, version
  `v1`; `cancellation` is `disabled`; `response` is protocol `engineering-assurance.process-evidence`
  version `1`, adapter `engineering-assurance.cli-process-evidence-adapter` version `1`, exit codes `any`.
- The runner shall run the version invocation of an engine first, with `producer.version` `unprobed`,
  stdin `null` and no inputs, and shall use the trimmed version stdout, bounded by the solver version
  limit, as `producer.version` of that engine's query invocation. When the version invocation is not
  `completed` or yields empty text, the runner shall skip the query invocation and record the engine
  as unavailable.
- The runner shall run the query invocation with stdin bound to the input artifact role `query`, whose
  file is the lowered query staged in the capability root.
- The runner shall set the budget as: `timeoutMillis` the solver wall-time limit, `maxStdoutBytes` the
  lesser of the solver stdout limit and 8,388,608, `maxStderrBytes` the lesser of the solver stderr
  limit and 8,388,608, `maxInputBytes` the solver stdin limit, `maxOutputArtifacts` `0`,
  `maxOutputBytes` `0`, `maxDescendants` `16` and `maxConcurrency` `1`. The stdout ceiling of 8,388,608
  bytes is `MAX_STREAM_BYTES` of `engineering-assurance`; it is below the 16,777,216-byte
  `AdapterLimits` default, so a solver whose stdout exceeds it is reported as truncated and
  non-conclusive. The other values are the smallest that a single sequential solver needs and lie
  inside the request schema's caps.
- The runner shall create a runner-owned capability root as a fresh directory with mode `0700`,
  created exclusively under the parent directory of `--output` with a name unique to the run, shall
  pass it as `capabilityRoot` and shall remove it, with the staged query, on every exit path, including
  failure and non-conclusive outcomes.
- When `producer-execute` returns state `completed`, the runner shall take the solver's exit code,
  stdout and stderr from the retained process evidence.
- If the result state is `unavailable`, `timed_out`, `failed`, `refused`, `malformed_response`,
  `containment_failure` or `cancelled`, the runner shall record the solver outcome as non-conclusive
  and shall not treat it as `sat` or `unsat`.
- If `producer-execute` cannot be started, exits with any non-zero status (including `2`), is ended by
  a signal, or does not print exactly one newline-terminated result line that decodes as a
  `producer-execution-result/v1` document, the runner shall exit `4`, name the cause on stderr and
  write no report.
- When the agreed status is `satisfied`, the runner shall exit `0` and record `satisfied` in the report.
- When the report records a conclusive agreement on `refuted`, the runner shall exit `1`.
- If the differential disposition is not agreement, the runner shall publish the report retaining both
  engine records and exit `3`.
- When an argument is unknown, repeated, missing or lacks a value, the runner shall exit `2`, print the
  usage on stderr and write no report.
- When the package file is unreadable, is refused by the reader, or contains no analysable clause or a
  non-Boolean analysis claim, the runner shall exit `2`, name the cause on stderr and write no report.
- When a solver or `producer-execute` path is relative, empty, non-UTF-8 or over the path bound, the
  runner shall exit `2`, name the cause on stderr and write no report.
- When the destination exists, its parent directory is missing or unwritable, or publication fails
  ahead of the atomic rename, the runner shall exit `4` and leave the destination unmodified.
- If publication fails with `published-durability-unknown`, the runner shall exit `4` and report that
  state on stderr.
- The runner shall produce byte-identical report bytes for identical package bytes, identical solver
  executables and identical limits.
- The runner shall write no file other than the capability root and its staged query, which it removes, and the staged and published report.
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
| FR-009-AC-5 | A `producer-execute` that cannot be started, that exits with status `2`, with status `1`, or with any other non-zero status, that is ended by a signal, that prints no result line, two lines, or a line that is not a `producer-execution-result/v1` document makes `run-package` exit `4` with the cause on stderr and no report file. | Test |
| FR-009-AC-6 | A missing package file, a file `ContractPackageWireV1::read` refuses, a package that names a dependency, a package with no `analysis_claim`, a package with a non-Boolean `analysis_claim`, and a document one byte over the 1,048,576-byte `CheckedPackageReadLimits::bounded()` limit (an `Incomplete` result) each exit `2`, write no report and name the cause on stderr. A document exactly at the limit is not refused for size. | Test |
| FR-009-AC-7 | An unknown, repeated or missing argument and an option without a value each exit `2` with the usage on stderr and no report. | Test |
| FR-009-AC-8 | Each of a relative, an empty, a non-UTF-8 and an over-long path, given in turn as `--z3`, `--cvc5` and `--producer-execute`, exits `2` and writes no report. | Test |
| FR-009-AC-9 | An existing `--output` file, a missing parent directory and an unwritable parent directory each exit `4`; an existing destination keeps its bytes. | Test |
| FR-009-AC-10 | A unit test of the runner function, injecting a directory-sync failure after rename through the publication seam of [TC-008](../test/TC-008-library-cli-atomic.md), shows the runner maps the `published-durability-unknown` publication error to exit `4` with that state named on stderr. | Test |
| FR-009-AC-11 | Two runs over the same package bytes, executables and stub results write byte-identical report files. | Test |
| FR-009-AC-12 | After a run the package file and its directory hold no added, removed or modified entry; the only entries added to the `--output` parent directory are the published report, and during the run a capability-root directory of mode `0700` that is gone after every exit path (conclusive, non-conclusive, exit `2`, exit `4`). | Test |
| FR-009-AC-13 | `run-package --help` lists the five arguments and the exit classes `0` to `4`. | Inspection |
| FR-009-AC-15 | Every request the runner logs validates against the request schema published by `producer-execute --schema request`, and the budget it carries has `maxStdoutBytes` and `maxStderrBytes` at most 8,388,608, `maxOutputArtifacts` and `maxOutputBytes` `0`, `maxDescendants` `16` and `maxConcurrency` `1`. | Test |
| FR-009-AC-16 | The version request of an engine carries `producer.version` `unprobed` and stdin `null`; its query request carries the trimmed version stdout of the version result; a version result that is not `completed` or is empty yields no query request and the engine is recorded unavailable (exit `3`). | Test |
| FR-009-AC-14 | A run with the real `engineering-assurance` executable and real Z3 and cvc5 on a lowered fixture exits `0` or `1` and its report validates; the test is skipped with a stated reason when either solver or the executable is absent. | Test |

## Dependencies

 [FR-005](./FR-005-evidence-cli.md) for report rendering, validation and publication;
`engineering-assurance` FR-019 and FR-029 for `producer-execute`; `quire-contract-ir` for
`quire.contract-package/v1` and `ContractPackageV1::read`; `quire-spec-language` for the QSL checked
package that is lowered to the contract package.

### Gaps

The following gaps were measured against the current code. They are preconditions of an implementing
change, and none is closed by invented behaviour here.

1. `quire-spec-language` has no production step that produces a contract package. The only call of
   `CheckedPackageV2::lower` in its `qsl-package` sources is the `cfg(test)` module
   `qsl-package/src/emit/extent_agreement.rs`, and no `quire.contract-package/v1` document is committed.
   QSL's `checked_handoff::write` writes a checked-package handoff directory. Until a production step
   exists, the fixture is produced at test time.
2. The analysis API takes `StatementInput` built from `ContractPackage<TypedExpression>` and a
   `DeclarationEnvironment`. `ContractPackageV1` holds a semantic graph of checked terms. Selection of
   the analysable clauses is defined above; the translation of an `analysis_claim` body term into a
   statement for the analysis API does not exist and is needed by the runner.
3. `execute_solver` in `src/solver.rs` runs a solver process directly. `run-package` runs solvers through
   `producer-execute` instead; whether the library path migrates is outside this requirement.
4. `producer-execute` is a separate executable. The runner depends on it through a process boundary
   only, with no Cargo dependency on `engineering-assurance`.
