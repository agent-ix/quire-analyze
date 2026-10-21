---
id: SR-5643
title: PLAT-1173 CLI kit adoption gap analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze PLAT-1173 candidate versus origin/main@3706867; FR-005-AC-1; TC-008; src/main.rs; tests/cli_streams.rs; tests/analysis_semantics.rs; Cargo.toml; Cargo.lock; Makefile"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: references
---

## Summary

The CLI kit change is owned by FR-005's CLI publication and stable exit-class requirement. The new tagged binary test checks a usage failure's exit class and streams. Existing tagged tests exercise the publication path and its exit classes, while the report producer remains unchanged. No stub, duplicated kit implementation, or weakened gate was introduced in the changed scope.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new trace or implementation gap in the changed scope (placeholder). | - |

## Coverage

`quoin matrix --repo /home/peter/dev/worktrees/plat-1173-quire-analyze --json` computed 52 criteria, 36 with static test binders. FR-005-AC-1 binds both `tests/analysis_semantics.rs:829` and the new `tests/cli_streams.rs:5`. The other 16 unbound criteria predate this change and are outside this PR's scope. Run evidence was not bound into the matrix; the local tests passed separately. Semantic review: skipped because it is optional and was not requested. Plan completion: not assessed.

## Verdict

**PASS for the changed scope.** The test exercises the real executable and checks outputs that would fail if the kit integration changed the established CLI contract. This differential result does not claim repository-wide trace completeness or a validated assurance profile.
