---
id: SR-17031
title: Code and Rust review of VER-78 PR 81 producer-execute evidence records
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze (quire-analyze#81, diff origin/main...HEAD); src/producer_evidence.rs, src/solver.rs, src/lib.rs, tests/producer_evidence.rs, tests/foundation.rs"
review_set: subset
---

## Summary

Ticket: VER-78. Code review with the Rust review lane folded in, scoped to
`git diff origin/main...HEAD` of quire-analyze#81, checked against
`spec/functional/FR-009-package-runner.md` (request fields, budget, result-state
table, AC-4/5/15/16/17).

The PR adds `src/producer_evidence.rs` (request builders, result-line parser,
`solver_record_from_evidence`, `version_text`), extracts the post-exit
classification of `execute_solver` into `classify_exit`, widens `SolverRecord`
fields to `pub(crate)`, and drops a prose-substring assertion from
`tests/foundation.rs` (intentional; it asserted spec prose, not behaviour).

Measured with Rust 1.98.1 through `locked-build.sh`:
`cargo test --test producer_evidence --test foundation --test solver_adapters --lib`
all pass (5, 5, 7, 17), `cargo clippy --all-targets --all-features -- -D warnings`
is clean. A hand-built request equal in shape to what the builders emit (version and
query, Z3 arguments, FR-009 budget) was accepted by the installed
`engineering-assurance 0.8.1 producer-execute` against a fake solver, and its result
line uses the same field names and byte-array stream encoding the parser expects.

## Verdict

**PASS with one medium finding.** The request fields and budget match FR-009, the
result-state mapping matches every table row, `sat`/`unsat` arise only from
`completed`, and the `classify_exit` extraction is behaviour-preserving (the old
`!ExitStatus::success()` is equivalent to `code != Some(0)` over `ProcessExit`, and
the Z3 model-unavailable exception keeps both load-bearing conditions). The version
probe gate mirrors the direct path's `inspect_executable` (non-zero exit, truncation,
non-UTF-8, NUL, empty after trim give `identity-error`). FND-001 is a contract gap in
the result-line parser.

## Scope

- examined: FR-009 request fields: "`protocol` is the request protocol constant; `procedure` is `direct`; `producer.executable` is the absolute solver path ... `cancellation` is `disabled`; `response` is protocol `engineering-assurance.process-evidence` version `1`, adapter `engineering-assurance.cli-process-evidence-adapter` version `1`, exit codes `any`."
- examined: FR-009 budget: "`timeoutMillis` the solver wall-time limit, `maxStdoutBytes` the lesser of the solver stdout limit and 8,388,608, ... `maxDescendants` `16` and `maxConcurrency` `1`."
- examined: FR-009 result-state table, every row.
- examined: FR-009 "does not print exactly one newline-terminated result line that decodes as a `producer-execution-result/v1` document, the runner shall exit `4`".
- examined: `src/solver.rs` `classify_exit` extraction against the removed inline branch of `execute_solver`.
- examined: `tests/foundation.rs` removal of `MASTER_SPEC.contains(...)`.
- context_only: `src/solver.rs` `inspect_executable` version probe.
- context_only: `engineering-assurance producer-execute --schema request` and `--schema result`.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | `parse_result_line` accepts a result line with no trailing newline (`strip_suffix(b"\n").unwrap_or(stdout)`), but FR-009 requires exactly one newline-terminated line (otherwise exit 4) and the function's own doc says "one line terminator". No test covers the unterminated case. | src/producer_evidence.rs:465 | correct-requirement-no-evidence |
| FND-002 | low | The result decoder is not as closed as the PR states ("unknown fields refused"): the internally tagged `State` and `FailureReason` enums carry no `deny_unknown_fields`, so extra keys inside `state`/`reason` are ignored, and `ProcessEvidence.terminal_status: Option<_>` accepts a missing `terminalStatus` that the result schema requires. | src/producer_evidence.rs:404-439 | correct-requirement-no-evidence |
| FND-003 | low | Wire-type naming: `RequestResponse.exit_codes` is typed `Cancellation` (a `{kind}` struct reused for an unrelated field), and `outputs` is `Vec<()>`, a type that would serialize `null` items. Both serialize correctly today but read wrongly. | src/producer_evidence.rs:144,164 | implementation-bug-despite-evidence |
