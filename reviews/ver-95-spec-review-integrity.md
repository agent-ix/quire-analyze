---
id: SR-17073
title: Integrity review of VER-95 specification changes
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; FR-041, FR-042, TC-041, spec/index.md, spec/test-matrix.md, MP-001"
review_set: subset
---

## Summary

The changed requirements, test case, references, and matrix were checked for completeness, consistency, and atomicity. TC-041 names a versioned result contract but does not define its shape, and its per-mode result description does not carry the step/horizon data required by the comparison criteria.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Define the hybrid-tool-result/v1 members, including how requested horizon and per-step/per-mode intervals are represented; TC-041 requires a horizon check and comparison at every step, but its parser contract only specifies a per-mode list of intervals, leaving the required result identity and temporal indexing undefined. | TC-041 steps 3–4, FR-042-AC-4 |
| FND-002 | medium | Split FR-042-AC-2 or give its distinct obligations separate criteria: it combines symbol/code removal, specification-file/index/matrix deletion, and oracle containment behavior, which require different evidence and can pass or fail independently. | FR-042-AC-2 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at f6a9324. after_excerpt: `Its root has exactly \`schema\`, \`requested_horizon\`, and \`steps\`. \`schema\` is that exact version string; \`requested_horizon\` is a non-negative integer equal to the request horizon. \`steps\` is an array with one entry for each integer step from 0 through the requested horizon, inclusive, in order.` |
| FND-002 | fixed | Verified at f6a9324. after_excerpt: `| FR-042-AC-2 | Under delegate-and-remove, reach, replay_hybrid, bind_hybrid_result and hybrid_method_outcome are absent from the crate, and no code path computes a hybrid enclosure inside the crate. | Inspection |` |
