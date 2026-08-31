---
id: REV-010
title: "Bounded solver adapter specification review"
type: Review
---
# Bounded solver adapter specification review

## Scope

Pre-implementation producer review for native issue #3 and standing reviewer issue #16. This review
covers the quantitative profile, normalized outcome boundary, path/argv isolation, identity capture,
process containment, measurement procedure, and plan delta. It is not independent approval or a
release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| ADP-S01 | critical | FR-003 referenced a cleanup bound that did not exist, so cleanup could neither pass nor fail objectively. | Closed in specification: the v1 total cleanup ceiling is 1,000 ms, including a 100 ms graceful interval. TC-005 requires three timeout and three cancellation measurements, zero surviving group members, all six durations, and their maximum. Issue #16 closes only after executable measurements pass. |
| ADP-S02 | high | “Bounded” I/O and model capture had no byte values or exact boundary methods. | Closed in specification: query, stdout, stderr, model, version, executable, and path ceilings are numeric; exact-at-limit and one-over-limit tests are required. |
| ADP-S03 | high | A synchronous stdin write or blocked output reader could defeat the wall timeout. | Implementation constraint: stdin, stdout, and stderr use independent workers while a monotonic monitor owns timeout/cancellation and cleanup. Readers continue draining after their capture ceiling so a full pipe cannot turn a limit into a hang. |
| ADP-S04 | high | Killing only the direct child could leave descendants alive. | v0.1 support is explicitly POSIX/Unix and creates a fresh process group before exec. Every terminal path terminates and probes the group. Other platforms return `unsupported-platform` before spawn and remain visible in M-10. |
| ADP-S05 | high | PATH lookup, shell parsing, or caller-selected argv could make execution authority ambiguous. | The API requires an absolute UTF-8 path and fixed engine-specific argv; query bytes go only to stdin. Canonical path, executable SHA-256, exact normalized version output, argv, limits, and configuration digest are recorded. |
| ADP-S06 | medium | Including elapsed time in deterministic semantic bytes would make reproducibility impossible. | NFR-001 now separates the deterministic semantic-outcome projection from observational elapsed/cleanup measurements retained in the execution record. |
| ADP-S07 | medium | A caller could select nominally finite but operationally unbounded limits. | Callers may only reduce positive limits from the closed v1 profile ceilings; zero or larger values are invalid before identity probing or spawn. |

## Plan Delta

Task-005 advances to `in_progress` because Task-004 is locally complete. Implementation proceeds in
this order: closed public types and limit validation; executable identity probe; bounded POSIX runner;
protocol normalization; hostile fake-process and exact-boundary tests; measured cleanup record; code
review and completion gap analysis. Task-006 remains guarded.

## Pre-implementation Verdict

PASS to implement. The prior unquantified acceptance language is corrected. The numeric cleanup
claim remains unverified—not complete—until TC-005 retains successful monotonic measurements against
the implementation.
