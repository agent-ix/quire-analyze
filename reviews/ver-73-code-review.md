---
id: SR-17035
title: Code and Rust review of VER-73 PR 84 run-package runner
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze (quire-analyze#84, diff origin/main...HEAD at a0f0f3c); src/package_runner.rs, src/main.rs, src/lib.rs, src/report.rs, tests/package_runner.rs, tests/common/mod.rs, tests/package_translation.rs"
review_set: subset
---

## Summary

Ticket: VER-73. Code review with the Rust review lane folded in, scoped to the
`run-package` runner diff. Gates run on the PR head with Rust 1.98.1:
`cargo test --test package_runner` passed 22 of 22, the AC-10 unit test
`runner_maps_published_durability_unknown_to_exit_four` passed, workspace
`clippy --all-targets --all-features -D warnings` was clean, and `fmt --check` was
clean. `engineering-assurance` 0.8.1 is on PATH on this host, so the real-executor
tests (FR-009-AC-15 schema validation, NFR-003-AC-1 stall and flood) really ran.
z3 and cvc5 are not installed, so FR-009-AC-14 took its skip branch and reports `ok`.

## Verdict

**CONDITIONAL**: no high findings. The runner is sound on the points that matter.
The capability root is a drop guard, so it is removed on every `?` return and on
unwind (no `panic = "abort"` profile). Exit codes follow the result-state table:
`Agreement`+`satisfied` gives 0, `Agreement`+`refuted` gives 1, anything else gives 3.
A spawn failure, a non-zero exit, a signal, an oversized output or a bad result line
gives 4. Symlink checks cover every component of `--z3`, `--cvc5`, `--producer-execute`
and the `--output` parent. The only process the runner starts is `producer-execute`
(`Command::new(producer)` with `env_clear`). Publication goes through
`publish_report_new`. There is no Cargo dependency on `engineering-assurance`, and
nothing is vendored: the fixtures are built at test time and `tests/common` was moved
out of `package_translation.rs`, not copied.

The 80 MiB result-line bound is enough for the 8,388,608-byte stream ceiling. Two
streams times 8,388,608 bytes times 4 JSON characters per byte is 67,108,864, which
is less than 83,886,080. The findings are a test that cannot fail on its stdin
dimension, non-fatal cleanup handling, a hand-computed bound, and two test-intent
nits.

## Scope

- examined: src/package_runner.rs (argument parsing, path checks, capability root, invoke, exit mapping)
- examined: src/main.rs (run-package dispatch, help, error classes)
- examined: src/report.rs test `runner_maps_published_durability_unknown_to_exit_four`
- examined: tests/package_runner.rs, tests/common/mod.rs (stub producer-execute realism against engineering-assurance 0.8.1 `CapturedStream` serialization)
- context_only: src/producer_evidence.rs (`parse_result_line`, `solver_record_from_evidence`), engineering-assurance src/producer_execution.rs

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | NFR-003-AC-2 test cannot fail on its stdin dimension. The "normal" run pipes stdin and closes it at once, and the "hermetic" run uses /dev/null, so both see EOF and no run has open stdin. Network access is not restricted either. A runner that read stdin would still pass. | tests/package_runner.rs:1048-1071 |
| FND-002 | low | A capability-root removal failure in `Drop` is only a stderr line, and the exit stays 0, 1 or 3, although FR-009 says the root "shall" be removed on every exit path. A SIGINT or SIGTERM to the runner during a solver run leaves the root and the `producer-execute` child behind. | src/package_runner.rs:400-411 |
| FND-003 | low | `MAX_RESULT_LINE_BYTES` is a literal `80 * 1024 * 1024` that restates arithmetic over `MAX_PRODUCER_STREAM_BYTES` in a comment, instead of deriving it, so the two can drift. The oversize branch (kill, then exit 4) has no test. | src/package_runner.rs:49-53, src/package_runner.rs:467-479 |
| FND-004 | low | The match guard `Some("--help") if seen == 1 && arguments.next().is_none()` consumes the iterator from inside a guard. The behaviour is correct only because every other path falls to `_ => usage`. A side-effecting guard is hard to audit. | src/package_runner.rs:166 |
| FND-005 | low | The FR-009-AC-10 test calls the `publication_failure` helper directly, not the runner function the AC names, and it checks the message string, not stderr. | src/report.rs:1059-1072 |
