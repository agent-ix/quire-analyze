---
id: REV-011
title: "Bounded solver adapter code review"
type: Review
---
# Bounded solver adapter code review

## Scope

Producer review of native issue #3: the Linux process-containment implementation, closed resource
profile, executable identity, fixed engine protocols, result records, hostile-process tests, and the
issue #7 query boundary. This is not independent approval or a release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| ADP-F01 | critical | The original requirement made cleanup and I/O bounds impossible to verify objectively. | Closed: FR-003 and TC-005 define every v1 ceiling, exact boundary methods, three timeout and three cancellation repetitions, zero surviving group members, and a 1,000 ms maximum cleanup time. The measured local maximum is 4 ms. |
| ADP-F02 | critical | Blocking writes or reads could make the wall-time limit itself block forever. | Closed: all three pipes are nonblocking and serviced by the monotonic monitor. Output is drained after its retention ceiling, so backpressure cannot turn a size violation into a hang. A solver that never reads a query larger than a pipe is covered. |
| ADP-F03 | critical | Terminating only the direct process could leave solver descendants alive. | Closed on Linux: the child creates a fresh POSIX process group before exec; every completion, error, timeout, and cancellation path signals and probes the group, escalating from TERM to KILL within the total cleanup deadline. |
| ADP-F04 | high | PATH lookup, shell interpretation, or caller-selected argv could change execution authority. | Closed: configuration requires an absolute UTF-8 bounded path, arguments are fixed per engine, the environment is cleared, and query bytes are accepted only through stdin. Metacharacter-path tests prove no shell expansion. |
| ADP-F05 | high | Hashing a path and later executing that path would leave a replacement race. | Closed: the adapter canonicalizes and opens the executable once, hashes that descriptor, executes the same descriptor with `fexecve`, and rehashes it after version probing and query execution. Mutation in either phase fails closed. |
| ADP-F06 | high | Publicly mutable query or result fields would let callers rewrite the record after validation. | Closed: `QueryBundle` and `SolverRecord` fields are private and exposed through read-only accessors; execution recomputes and seals all adapter-owned record fields. |
| ADP-F07 | high | Protocol text, stderr, signals, and truncation could be mistaken for a solver answer. | Closed: the response parser recognizes exactly one status and, only after `sat`, one bounded balanced model expression. Solver errors, contradictory statuses, malformed bytes, diagnostic stderr, signals, nonzero exit, and every truncation state remain distinct and non-conclusive. |
| ADP-F08 | medium | Platform-neutral source could imply process-tree containment that was never measured outside Linux. | Closed for v0.1 scope: non-Linux returns `unsupported-platform` before spawn. Equivalent measured containment remains explicit in issue #20. |
| ADP-F09 | medium | An embedded solver dependency would violate the external-process boundary. | Closed: the manifest contains no Z3/cvc5 library; the only process implementation uses `std::process::Command`. Real-engine differential execution remains later issue #5. |

## Code Quality

The public state spaces are closed enums and validated configurations. Hashes are domain-separated
and length-prefixed. The unsafe surface is limited to documented Linux `pre_exec`, `fcntl`, and
process-group signal calls, and the local unsafe audit requires a `SAFETY` justification at every
block. The module retains raw bounded stdout/stderr/model and observational timings while excluding
timing from deterministic semantic bytes.

## Verdict

PASS for the bounded Linux v1 adapter slice. No unresolved soundness or acceptance finding blocks the
local issue #3 implementation. Real-engine parity, non-Linux containment, sustained stress/fault
injection, model replay, conclusion production, and release evidence remain separately tracked.
