---
id: SR-17052
title: "Code review of FR-012 integer analysis_claim translation"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze#93; src/package_translation.rs, tests/integer_translation.rs, tests/package_runner.rs, tests/package_translation.rs, tests/common/mod.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-012", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-009", type: references }
  - { target: "ix://agent-ix/quire-analyze/AP-001", type: references }
---
# Code review of FR-012 integer analysis_claim translation

## Summary

Ticket: VER-83. This review covers `git diff origin/main...HEAD` of quire-analyze#93 (two commits),
with the rust-review lane folded in. The Rust follows rust-style: no panics on package input,
typed errors, and exhaustive matches. Clippy `--all-targets --all-features -D warnings` is clean,
and the focused tests pass: 8 passed and 6 ignored in integer_translation, 25 passed and 2 ignored
in package_runner, 17 passed in package_translation. The ignored tests were then run on HEAD and
on a throwaway local merge of PR #92 (VER-71). With #92 merged, one ignored test still fails in
translation, for a reason unrelated to VER-71. One ignored end-to-end test exits 3 instead of 0.

## Verdict

**FAIL**. One high finding: a test that is ignored as waiting for VER-71 can never pass as
written. These parts are correct. The operation-id to `ExpressionKind`/operator mapping matches
the FR-012 table row for row. The product-of-two-parametric-operands check matches FR-012 and the
FR-010 constant-term rule. div, rem, mod and other unlisted integer operations are refused at
their node. The FR-009 bounds are reused unchanged and pass over integer sums. Translation is
deterministic: it is a pure walk over a `BTreeMap`. The two edited FR-009 refusal tests are
legitimate: integer `add` is no longer a refusal under amended FR-009, and the replacements use
`quire.op.integer.min` and `div`, which FR-009-AC-23 and FR-012 still refuse. Public-repo hygiene
is clean: no copied files, no SHA or digest ceremony, no roadmap text, no build artifacts, and
`.gitignore` carries unanchored `target/`.

## Assurance Context

Profile AP-001 (`spec/assurance/AP-001-analyze-release.md`) applies through its
impact-false-conclusion scenario. The change refuses terms rather than concluding on them, so it
adds no false-conclusion path. Unavailable context: Z3 and cvc5 are not installed, so real-solver
evidence was not available. Active exceptions: none.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `each_integer_operation_translates_to_its_expression_kind` is ignored as "needs VER-71 lowering" but it stops in `check_expression` ("integer result may exceed its named bounds") on HEAD and also with PR #92 merged. Under the reject policy, `a + 1` and `a - 1` over `[0,100]` can never be admitted, so the test can never pass and the Add/Subtract rows of FR-012-AC-4 have no evidence. | tests/integer_translation.rs:69-125 |
| FND-002 | medium | The ignore reason of `a_dose_bounded_by_a_maximum_runs_end_to_end` is incomplete. With PR #92 merged it exits 3 (inconclusive, both solvers `satisfied`), because model verification parses Boolean models only (`parse_boolean_model`). Integer witness replay (FR-011) is in neither main nor #92, so FR-012-AC-1 stays unevidenced after VER-71 lands. | tests/package_runner.rs:1405-1413; src/analysis.rs:545-563 |
| FND-003 | low | The refusal message builds "yields a integer where a Boolean is required" from `Class::name`, which gives the wrong article on stderr. | src/package_translation.rs:709 |
