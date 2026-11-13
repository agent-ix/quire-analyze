---
id: SR-17032
title: "Failure-domain review of VER-77 analysis_claim translation (FR-009)"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-analyze; quire-analyze#82 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md (Translation section, FR-009-AC-23)"
review_set: subset
---

## Summary

Ticket: VER-77. This lens looks for resource and edge-case failures in the new Terms
rules, which inline every `Reference` target. Checked against `src/smt.rs` (`lower_expression`:
`MAX_EXPRESSION_DEPTH` 128 counted from 1, `MAX_EXPRESSION_NODES` 100,000) and against
`quire-contract-model` `check_expression` (`preflight`: 10,000 nodes, depth 256). Graph nodes
are keyed by identity, so a subterm reached twice is one node referenced twice.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The translation inlines each referenced node and bounds only the depth of the chain, so a shared subterm can blow up exponentially. Take x1 = and(p, p) and xk = and(x(k-1), x(k-1)) for k up to 60. That is 61 graph nodes, well inside the reader's 10,000-node limit, and depth 61, under 128. It expands to about 2^61 `Expression` nodes before `check_expression` or `lower_boolean_statements` can apply its node bound. The runner would hang or run out of memory instead of exiting 2. Fix: count the `Expression` nodes the translation produces and stop at 10,000, the `check_expression` limit and the lower of the two. At that point the runner exits 2 naming `semantic_graph.nodes[N]`. Add this case to FR-009-AC-23. | spec/functional/FR-009-package-runner.md:96-128, FR-009-AC-23 |
| FND-002 | medium | "A chain deeper than `MAX_EXPRESSION_DEPTH` (128)" does not say what is counted. A `Reference` to an expression node adds no `Expression` level, but `lower_expression` counts `Expression` depth from 1 and refuses anything above 128. If the translation counts reference hops, a chain that passes it can still fail in `lower_boolean_statements`. That failure exits 2 naming `claim-N`, not `semantic_graph.nodes[N]` as FR-009-AC-23 requires. The two readings fail differently on the same package. Fix: define the bound as the nesting depth of the translated `Expression` tree, with the root at depth 1 and a refusal above 128, which matches `src/smt.rs`. | spec/functional/FR-009-package-runner.md:122-124, FR-009-AC-23 |

## Verdict

FAIL: FND-001 lets an admitted package hang the runner. It is fixed with an expanded-node bound
in the Terms rules and one more AC-23 case. The cycle refusal and the non-Boolean refusals
are well defined.
