---
id: SR-17032
title: Gap analysis of VER-78 PR 81 against FR-009 AC-4, 5, 15, 16, 17
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze (quire-analyze#81, diff origin/main...HEAD); spec/functional/FR-009-package-runner.md, src/producer_evidence.rs, tests/producer_evidence.rs"
review_set: subset
---

## Summary

Ticket: VER-78. Planless gap analysis scoped to the PR diff. Plan completion: not
assessed. The PR delivers FR-009 Dependencies gap 3 (a library constructor of
`SolverRecord` from `producer-execute` evidence); `run-package` itself is not in
this PR. The new tests carry `#[trace]` tags for FR-009-AC-4, AC-5, AC-15, AC-16
and AC-17. Each tag was read against the AC text and the assertions in
`tests/producer_evidence.rs`. All five tests pass (Rust 1.98.1).

## Verdict

**PASS for the library slice, with coverage gaps.** Every result-state row maps
to the stated outcome and disposition, and the completed path matches a direct
run of the same bytes for nine stdout/stderr/exit shapes. The gaps: AC-15's
schema-validation clause has no test (FND-001); AC-4, AC-5 and AC-17 are stated
against `run-package` exit status and report contents, which these library tests
cannot show, yet the tags mark them covered (FND-002); and AC-16 contradicts the
result-state table (FND-003, a spec defect). Two table rows' dispositions are not
asserted (FND-004).

## Scope

- examined: FR-009-AC-4: "For each of the states `unavailable`, `timed_out`, `failed`, `refused`, `malformed_response`, `containment_failure` and `cancelled` returned for one engine, and for `sat` against `unsat` across engines, `run-package` exits `3` and the report retains both engine records."
- examined: FR-009-AC-5: "A `producer-execute` that cannot be started, that exits with status `2`, ... that prints no result line, two lines, or a line that is not a `producer-execution-result/v1` document makes `run-package` exit `4` with the cause on stderr and no report file."
- examined: FR-009-AC-15: "Every request the runner logs validates against the request schema published by `producer-execute --schema request`, and the budget it carries has `maxStdoutBytes` and `maxStderrBytes` at most 8,388,608, `maxOutputArtifacts` and `maxOutputBytes` `0`, `maxDescendants` `16` and `maxConcurrency` `1`."
- examined: FR-009-AC-16: "The version request of an engine carries `producer.version` `unprobed` and stdin `null`; its query request carries the trimmed version stdout of the version result; a version result that is not `completed` or is empty yields no query request and the engine is recorded unavailable (exit `3`)."
- examined: FR-009-AC-17: "For each row of the result-state table, a stub result of that state for one engine yields the stated engine outcome in the published report, the stated disposition, and exit `3` (for `completed`, the classification the library gives the same bytes run directly)."
- context_only: FR-009 Dependencies, gap 3.
- context_only: `src/report.rs:182` disposition mapping (only `missing-executable` and `unsupported-platform` give `unavailable`).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | AC-15 requires every request to validate against `producer-execute --schema request`; `version_and_query_requests_follow_fr_009` asserts field values only and never validates against the schema, so a schema-invalid field (for example an extra key) would pass the tagged test. | tests/producer_evidence.rs:159-244 | correct-requirement-no-evidence |
| FND-002 | medium | AC-4, AC-5 and AC-17 state `run-package` exit status (3 or 4), report contents, stderr cause and absence of a report file. The tagged tests are library-level (outcome, disposition, parse error kind) and cannot show any of that, but the `#[trace]` tags mark the ACs covered, so the matrix will report them covered before `run-package` exists. | tests/producer_evidence.rs:247,283,390 | correct-requirement-no-evidence |
| FND-003 | medium | AC-16 says a bad version run leaves the engine "recorded unavailable (exit 3)", but the result-state table records the failed version run's own outcome (`identity-error` when empty), whose disposition is inconclusive unless it is `missing-executable` or `unsupported-platform`. AC-16 contradicts the table; the code follows the table. | spec/functional/FR-009-package-runner.md:191 | wrong-requirement |
| FND-004 | low | Two table rows lack a disposition assertion: `containment_failure` off Linux (`unsupported-platform`, disposition unavailable) is only checked as an outcome by a unit test, and the "query skipped after a bad version run" row checks outcome but not disposition. | tests/producer_evidence.rs:362-387; src/producer_evidence.rs:698 | correct-requirement-no-evidence |
