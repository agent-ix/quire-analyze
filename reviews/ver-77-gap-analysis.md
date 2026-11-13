---
id: SR-17034
title: Gap analysis of VER-77 PR 83 against FR-009 AC-19 to AC-27
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze (quire-analyze#83, diff origin/main...HEAD); spec/functional/FR-009-package-runner.md, spec/test/TC-015-package-runner.md, src/package_translation.rs, tests/package_translation.rs"
review_set: subset
---

## Summary

Ticket: VER-77. Planless gap analysis scoped to the PR diff. Plan completion: not
assessed. Each `#[trace]` tag in `tests/package_translation.rs` was read against the
AC text, and each AC-19 to AC-27 refusal path in `src/package_translation.rs` was
checked for a test. `quoin matrix --repo .` (quoin 0.30.1) reports every criterion in
the repository as `untagged`, including ones from earlier merged PRs, so the matrix
could not be used. The check was done by hand and is recorded here.

Reachability was measured from a scratch copy of the branch, by byte-level mutation of
a built contract package (which FR-009-AC-27 allows): re-serialising the canonical bytes
through `serde_json` round-trips them byte for byte, and `ContractPackageWireV1::read`
admits the mutated document.

## Verdict

**PASS WITH FINDINGS.** AC-20, AC-21, AC-22, AC-26 and AC-27 are covered. The
depth, node-count, shared-subterm, cycle, claim-reference and `integer.lt` cases of
AC-23 are covered. The PR leaves two refusal branches untested that are reachable.

On the coder's reported deviations:
- AC-24, empty occurrences: the claim that this cannot be tested is wrong. It is reachable (FND-001).
- AC-23, integer-typed reference: the claim that this cannot be tested is also wrong. It is reachable by mutation (FND-002).
- AC-25: the claim is right. No admitted package reaches it (FND-003).
- The TC-015 `integer_range` note is right. An `integer_range`-typed parameter reached as the claim body is a non-Boolean claim, named at the claim node, as the test asserts. As an operand of a Boolean operation, `CheckedPackageV2::read` refuses it as `IllTyped` (measured with an integer parameter).

## Scope

- examined: FR-009-AC-19: "A package whose Boolean `analysis_claim` term the translation cannot lower exits `2` naming the node as `semantic_graph.nodes[N]`, with no report."
- examined: FR-009-AC-20.
- examined: FR-009-AC-21.
- examined: FR-009-AC-22.
- examined: FR-009-AC-23.
- examined: FR-009-AC-24: "A claim node whose `source_map.nodes[N].occurrences` is empty exits `2` naming the node."
- examined: FR-009-AC-25: "An error from `check_expression`, `from_clause` or `lower_boolean_statements` for a clause exits `2` naming `claim-N`, the path and the message, with no report."
- examined: FR-009-AC-26.
- examined: FR-009-AC-27.
- examined: TC-015 case 19.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The AC-24 empty-occurrences case has no test. It is reachable: setting a node's `occurrences` and its `source_map.nodes[N].occurrences` to `[]` in the canonical bytes is admitted by `ContractPackageWireV1::read`, and the translation returns `SourceRegion` at `semantic_graph.nodes[N]` (measured). FR-009 line 90 ("a non-empty `regions` list for every occurrence") is true, but an empty occurrence list does reach the translation. | tests/package_translation.rs:567-607; src/package_translation.rs:380-395 | correct-requirement-no-evidence |
| FND-002 | medium | The AC-23 "integer-typed reference" case and the branch "the parameter is not Boolean" have no test. The case is reachable: retyping one parameter of `and(p, q)` to `scalar_type/integer` in the contract bytes is admitted, and the translation returns `CannotLower` "the parameter is not Boolean" at that parameter's node (measured). The `quire.op.integer.add` case is listed on `an_integer_operation_is_refused_at_its_node`, but the test never reaches it: `integer.lt` is refused first, at `less`. | tests/package_translation.rs:666-678; src/package_translation.rs:517-520 | correct-requirement-no-evidence |
| FND-003 | low | AC-25 cannot be reached from an admitted package. The translation enforces the 10,000-node limit of `check_expression` and the depth limit of 128, which is below its 256. It always sets the anchor that `from_clause` needs. The per-statement limits of `lower_boolean_statements` (100,000 nodes, 16 MiB) are above anything one bounded clause produces. The `Verification` of `Test` cannot be met, so the AC needs rewording. | spec/functional/FR-009-package-runner.md:161-163, 315 | wrong-requirement |

## Recommendations

- FND-001: add a test that byte-mutates a built package to give one node empty
  occurrences, and asserts `SourceRegion` naming `semantic_graph.nodes[N]`.
- FND-002: add a byte-mutation test that retypes a parameter operand to the integer scalar,
  asserting `CannotLower` at that parameter. Either add a separate `integer.add` case, or
  remove `quire.op.integer.add` from AC-23. Suggested TC-015 case 19 addition: "the
  `integer_range`-typed parameter as a claim body is refused as a non-Boolean claim
  naming the claim node; an integer-typed parameter operand is made by byte-level
  mutation of a built package".
- FND-003: change the AC-25 Verification from `Test` to `Inspection`, and reword it: "By
  inspection, an error from `check_expression`, `from_clause` or
  `lower_boolean_statements` for a clause maps to exit `2` naming `claim-N`, the path and
  the message, with no report; no admitted package reaches these errors, because the
  translation bounds are tighter." Drop AC-25 from TC-015 cases 19 and 20.
