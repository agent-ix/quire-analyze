---
id: REV-012
title: "Bounded solver adapter completion gap analysis"
type: Review
---
# Bounded solver adapter completion gap analysis

## Acceptance Coverage

| Issue #3 acceptance criterion | Evidence | Status |
|---|---|---|
| Timeout/cancellation kill the process tree within the declared bound | Three timeout and three cancellation executions spawn a shell plus descendant, probe both PIDs and the group, retain all durations, and enforce a 1,000 ms maximum; observed maximum 4 ms | closed on Linux |
| Malformed, excessive, contradictory, signaled, and nonzero responses cannot conclude | Exact protocol-state and every-limit integration tests; closed outcome census | closed |
| Z3 and cvc5 expose one result contract | Two independently configured executables use engine-exact argv and produce the same sealed normalized record shape | closed for adapter contract |
| Records retain execution identity and observations | Engine, normalized version, executable digest/path/size, argv, configuration/query digests, limits, exit, bounded streams/model, elapsed/cleanup, and diagnostic accessors are asserted | closed |
| Absolute independent paths require neither network nor shell resolution | Direct descriptor execution with cleared environment; metacharacter path test and missing/relative-path failures | closed |
| No solver library is linked | Manifest/source inspection and supply-chain graph gates | closed |
| Code review, tests, gap analysis, and retained record | REV-011, TC-005, REV-012, REV-013, and source-bound validation record | closed after validation capture |

## Open Downstream Gaps

| Gap | Disposition |
|---|---|
| No equivalent measured process-tree containment exists on non-Linux targets. | Native issue #20. Non-Linux fails before spawn in v0.1. |
| The suite does not run pinned real Z3 and cvc5 binaries or a differential corpus. | Existing issue #5 / Task-007; fake processes are correct for deterministic fault isolation but not engine conformance. |
| Sustained PID/PGID churn, full-ceiling pressure, executable-race campaigns, and deterministic OS-call fault injection are absent. | New defense-in-depth QA issue #21. |
| Analysis conclusions, model replay, and finding algebra do not exist yet. | Existing issue #4 / Task-006. |
| Versioned evidence reports and CLI publication do not exist yet. | Existing issue #5 / Task-007. |
| Lowering-wide mutation, fuzz, identity collision, and boundary campaigns remain incomplete. | Existing QA issue #19. |
| Installed Quire 0.31.0 disagrees between matrix validation and coverage calculation column names. | Existing issue #14; the repository's parsed matrix census fails closed. |

## Ticket Census

This review filed issue #21 for adapter containment stress and injected OS-boundary failures. Platform
work is already issue #20, lowering campaigns are issue #19, and real-engine/evidence work is issue
#5. No duplicate tickets are required.

## Verdict

No issue #3 Linux-v1 acceptance gap remains after source-bound validation capture. This does not
claim a fully tested cross-platform or real-engine analyzer.
