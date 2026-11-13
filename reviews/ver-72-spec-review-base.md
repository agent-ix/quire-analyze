---
id: SR-17027
title: "Base spec review of VER-72 package runner (FR-009, NFR-003, TC-015)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#80 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md, spec/nonfunctional/NFR-003-runner-invocation.md, spec/test/TC-015-package-runner.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-72. Base checklist review of the spec-only diff of quire-analyze#80 (head
d321187). `quire validate --scope . "spec/**/*.md"` exits 0 with two EARS warnings on
NFR-003 (quire 0.44.0). No hash, digest, pin, vendoring or roadmap content was added; the
QSL fixtures are referenced by checkout path, not copied. The blocking defect is that the
three fixture packages the ACs and TC-015 run over no longer exist on
`quire-spec-language` main, so FR-009-AC-1 and AC-2 cannot be executed as written. Several
criteria cannot fail, and the solver-configuration input and several error paths have no
criterion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The named fixtures `artifacts/compiled-protocol-v1`, `artifacts/compiled-protocol-v2` and `examples/protocol-handoff/` are absent from quire-spec-language main: `artifacts/` was removed by #1823 (handoffs are generated, not committed) and `examples/protocol-handoff/` by #2473 (QSL-840). FR-009-AC-1, AC-2 and TC-015 cannot run. Fix: name fixtures that exist on QSL main (for example `examples/checked-handoff/`) or say the test generates the handoff with QSL's own generator at test time. | FR-009:18-20, FR-009-AC-1, FR-009-AC-2, TC-015:20-23 |
| FND-002 | medium | `--solver-config` "when omitted the configured defaults of the library apply", but the library has no default solver executable: `SolverConfig::new` requires an executable path and only `AdapterLimits` has a `Default`. No AC covers the option, an unreadable or invalid config file, or a missing solver. Fix: make the option required, or define how the executables are resolved without it; add an AC that an invalid or unreadable config exits `2` with no report. | FR-009:25-26, src/solver.rs:86,204 |
| FND-003 | medium | FR-009-AC-2 says "the three exit statuses are asserted individually" without saying what each must equal, so it adds nothing AC-1 does not already cover and cannot fail on its own. Fix: state the expected exit class for each fixture under named stub solvers. | FR-009-AC-2 |
| FND-004 | medium | FR-009-AC-9 cannot fail in this repo: "the producer-execute definition above is stated in this spec" is self-referential, and "no Quire or Quoin code path spawns `run-package`" inspects other repositories. Fix: delete the AC, or reduce it to a check that `run-package --help` documents the arguments and exit classes. | FR-009-AC-9 |
| FND-005 | medium | Error paths have no criterion: an unknown or missing argument (exit `2`), a missing or unwritable `--output` parent directory and the FR-005 `published-durability-unknown` state (exit `4`). Fix: add criteria for each, or cite the FR-005 criteria that already cover them. | FR-009:31-34, FR-009-AC-4, FR-009-AC-5 |
| FND-006 | low | TC-015 runs a missing directory and an "empty package", but FR-009-AC-4 names three distinct cases: missing directory, unloadable package and a package declaring no request. One case is left out. Fix: list all three in the procedure. | TC-015:24-25, FR-009-AC-4 |
| FND-007 | low | NFR-003-AC-2 restates FR-009-AC-1 and AC-3, and NFR-003's hermetic statement and AC-3 restate the FR-009 producer-execute bullet and FR-009-AC-8. Fix: keep each obligation in one place. | NFR-003-AC-2, NFR-003-AC-3, FR-009-AC-8 |

## Verdict

FAIL: not mergeable until FND-001 is fixed. ID formats, sequential AC and CON numbering,
the index update and the TC-to-FR/NFR links are correct. FR-009-AC-5, AC-6 and AC-7 and
NFR-003-AC-1 are falsifiable and well targeted.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | FR-009-AC-6 says a nonexistent solver path exits 2, but FR-009-AC-3 says an absent solver exits 3, and FR-005 says a missing engine is unavailable. SolverConfig::new only rejects relative, non-UTF-8 or over-long paths (src/solver.rs:203-240); a nonexistent absolute path passes and yields SolverOutcome::MissingExecutable, so the disposition is unavailable and the exit is 3. Fix: drop nonexistent from AC-6 (relative, empty and over-long paths stay exit 2) and keep absent solver as exit 3 under AC-3, or define a pre-flight existence check and remove absent from AC-3. | spec/functional/FR-009-package-runner.md AC-6 |
| FND-009 | medium | With stub solvers that only answer sat, the run cannot reach agreement: a consistency sat needs a model that replays (classify_analysis, src/analysis.rs:510-527; verified_if_required in src/report.rs), otherwise the disposition is inconclusive and the exit is 3. As written, AC-1 and TC-015 case 1 fail against correct code. Fix: the stubs answer sat and emit a model that replays against the consistent fixture. | spec/functional/FR-009-package-runner.md AC-1 |
| FND-010 | medium | FR-009-AC-8 and TC-015 case 8 inject a directory-sync failure into the spawned binary through the TC-008 seam, but that seam (the fault PublicationIo and publication_test_termination) is cfg(test) in the library (src/report.rs:617-625, 865+) and is not compiled into the built binary. Fix: verify the mapping of publication error to exit 4 and stderr state with a unit test of the runner function using the fault IO, not through the child process. | spec/functional/FR-009-package-runner.md AC-8 |
| FND-011 | low | The oversized-file case and exceeds the bounded size name no bound. Fix: cite the decoder's limit (quire-contract-model MAX_CONFORMANCE_FILE_BYTES, 16,777,216 bytes) so the test sits just over it. | spec/functional/FR-009-package-runner.md AC-4 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in 9e07114 |
| FND-002 | fixed | verified in 9e07114 |
| FND-003 | fixed | verified in 9e07114 |
| FND-004 | fixed | verified in 9e07114 |
| FND-005 | fixed | verified in 9e07114 |
| FND-006 | fixed | verified in 9e07114 |
| FND-007 | fixed | verified in 9e07114 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-012 | high | The budget is taken from the solver limits, i.e. AdapterLimits::default(), whose stdout limit is 16,777,216 bytes (src/solver.rs:37). producer-execution-request/v1 caps maxStdoutBytes at 8,388,608 (the engineering-assurance request schema; MAX_STREAM_BYTES in src/producer_execution.rs), and FR-029 refuses an over-limit bound with producer_execute_request_structure_invalid and exit 2. FR-009 maps that to runner exit 4, so every run exits 4. maxDescendants, maxConcurrency, maxOutputArtifacts and maxOutputBytes have no source in AdapterLimits. Fix: state the budget mapping explicitly, with stdout at most 8,388,608 and a value for each of the other fields. | spec/functional/FR-009-package-runner.md Behavior |
| FND-013 | high | A stdin input artifact is a declared input file that the executor opens relative to the request's capability root (FR-019; request schema stdin {kind: input_artifact, role}). So the runner has to write the lowered query to a file. That contradicts 'The runner shall write no file other than the staged and published report' and FR-009-AC-12. Fix: specify a runner-owned temporary capability root (where it lives, its permissions and its removal), and amend the no-file bullet and AC-12 to allow it. | spec/functional/FR-009-package-runner.md AC-12 / Behavior |
| FND-014 | medium | The request has required fields the runner must fill that FR-009 leaves open: capabilityRoot (also the solver's working directory), producer name and version (for the query run the version is only known after the version run), caller, the containment contract that process-group-v1 requires, and cancellation. Two implementers would fill them differently, which also bears on byte-identical reports. Fix: state the value or rule for each field. | spec/functional/FR-009-package-runner.md Behavior |
| FND-015 | medium | 'Analysable clause' is undefined, as Gap 2 itself says ('no definition says which graph nodes are analysable clauses'). Yet the request FR-009 fixes, AC-6's no-analysable-clause case and the consistent and contradictory fixtures of AC-2 and AC-3 all depend on it, and the gap names no owner or ticket. Fix: define the selection rule in FR-009, since quire-analyze is the consumer and owns it, or cite the owning ticket and state that the dependent ACs wait on it. | spec/functional/FR-009-package-runner.md AC-6 / Gaps 2 |
| FND-016 | low | The Behavior covers producer-execute exiting with status 2, but not another non-zero exit, a signal or no result line. AC-8 tests only a relative producer-execute path, though the Behavior also lists empty, non-UTF-8 and over-long paths. Fix: treat any non-zero exit or missing result as exit 4, and list all four path cases in AC-8. | spec/functional/FR-009-package-runner.md AC-5 / AC-8 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-008 | fixed | verified in 720b33c |
| FND-009 | fixed | verified in 720b33c |
| FND-010 | fixed | verified in 720b33c |
| FND-011 | still-open | FR-009-AC-6 now says 'over the reader's byte limit', but the reader's limits are caller-supplied (CheckedPackageReadLimits) and FR-009 does not choose them, so the boundary is still unnamed. Naming CheckedPackageReadLimits::bounded() (1 MiB) would close it; see integrity FND-005. |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-017 | medium | FR-009 says only that a non-completed producer-execute state is recorded 'as non-conclusive'. The report's engine outcome is the closed v2 set (sat, unsat, unknown, timed-out, cancelled, missing-executable, spawn-error, ..., unsupported-platform, io-error; schemas/differential-report-v2.schema.json), and CON-1 forbids changing it. No mapping is given from refused, failed (exit_code/signal/stdout_too_large/stderr_too_large), malformed_response, containment_failure, cancelled, unavailable, or a skipped query after a bad version run, to those values. The mapping changes the report's disposition: compare_solver_records treats only missing-executable and unsupported-platform as unavailable. The library also has no public SolverRecord constructor, since only execute_solver builds one. Gap 3 calls the library path 'outside this requirement', but run-package cannot render without a record built from process evidence. Fix: add the state-to-outcome table and state in Gap 3 that a SolverRecord-from-evidence constructor is required. | spec/functional/FR-009-package-runner.md Behavior / Gaps 3 |
| FND-018 | low | FR-019 opens the capability root and the producer executable without following symbolic links in any path component. A --output parent or solver path that passes through a symlink (for example an alternatives link for a solver) is refused by producer-execute and surfaces as a non-conclusive exit 3 with no input diagnostic. Fix: canonicalize both paths before building the request, or refuse a symlinked path with exit 2. | spec/functional/FR-009-package-runner.md Behavior |

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-011 | fixed | verified in 665567a against code |
| FND-012 | fixed | verified in 665567a against code |
| FND-013 | fixed | verified in 665567a against code |
| FND-014 | fixed | verified in 665567a against code |
| FND-015 | fixed | verified in 665567a against code |
| FND-016 | fixed | verified in 665567a against code |
