---
id: SR-17076
title: Failure-domain review of VER-95 delegation contract
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-analyze; FR-042-AC-3–4, TC-041 steps 2–5, NFR-001 references"
review_set: subset
---

## Summary

The delegated path specifies timeout, output-limit, non-zero-exit, missing-tool, malformed-result, missing-mode, and inverted-interval outcomes, with no enclosure on failure. The malformed-input case in TC-041 is ambiguous about whether it should reach PHAVer.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Separate invalid hybrid requests from valid fit models: TC-041 step 2 includes malformed inputs among cases run through producer-execute, while steps 3–5 describe malformed output handling. The spec must say malformed requests are rejected before process launch (or define a safe external encoding) so executors do not invoke PHAVer on invalid input. | TC-041 steps 2, 5; FR-042-AC-4 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | Specify the typed reason for each output-limit, non-zero-exit, and schema-invalid/inverted-result outcome in FR-042-AC-7 and TC-041. The ticket requires named reasons for all five failure classes, but the spec names only `tool_timeout` and `tool_unavailable`, leaving the other adapter outcomes underspecified. | FR-042-AC-7, TC-041 step 5 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Verified at f6a9324. after_excerpt: `Malformed request models are invalid-input fixtures, not PHAVer fit cases: verify them separately in step 5 and do not send them to the external tool.` |
| FND-002 | still-open | FR-042-AC-7 and TC-041 step 5 still omit named reasons for output-limit breach, non-zero exit, and schema-invalid or inverted results; only timeout and missing-tool reasons are named. |
| FND-002 | fixed | Verified at 5907346. after_excerpt: ``Timeout maps to Incomplete with reason `tool_timeout`; output-limit breach maps to Incomplete with reason `tool_output_limit_exceeded`; non-zero exit maps to Incomplete with reason `tool_exit_nonzero`; missing tool maps to Refused with reason `tool_unavailable`; schema-invalid output maps to Refused with reason `result_schema_invalid`; and an inverted interval maps to Refused with reason `interval_inverted`.`` |

