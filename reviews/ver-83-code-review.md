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

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The new real-solver test `real_solvers_agree_with_the_controlled_run_on_the_dose_fixtures` (FR-012-AC-8) is not ignored and gates only on the executables being present. Where engineering-assurance, z3 and cvc5 exist it cannot pass on this branch. Without PR #92 the integer claims are refused at lowering. With #92 the satisfiable case needs VER-82 replay: the controlled run exits 3 with both solvers satisfied. It needs the same ignore reasons as the FR-012-AC-1 test. | tests/package_runner.rs:1435-1459 |
| FND-005 | low | `integer_refusals_exit_two_naming_the_node_through_the_runner` asserts only that stderr contains `semantic_graph.nodes[`, not the node's index. The label appears only in the assertion message, so a refusal naming the wrong node passes. The existing `node_index` helper would pin the index. | tests/package_runner.rs:1461-1490 |
| FND-006 | low | The `exercised` against `TABLE` assertion in `each_integer_operation_translates_to_its_expression_kind` compares two lists that are both hard-coded in the test file. It fails only if the test text itself is edited, so it checks bookkeeping rather than behaviour and should be deleted. | tests/integer_translation.rs:66-145 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The operands now use the point range [0,0] and negate uses [-100,100]. On a local merge of PR #92 every row (add, sub, mul, negate) passes `check_expression` and the test passes (16 of 16 in integer_translation). |
| FND-002 | fixed | The ignore reason now names PR #92 and VER-82 integer witness replay. With #92 merged the test still exits 3 (both solvers satisfied, inconclusive), which matches the stated reason. |
| FND-003 | fixed | `Class::name` returns "a Boolean" or "an integer", and both messages drop their hard-coded article. |
