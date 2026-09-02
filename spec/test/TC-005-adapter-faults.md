---
id: TC-005
title: "Verify adapter resource and failure isolation"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-003
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-001
    type: verifies
---
# TC-005: Verify adapter resource and failure isolation

## Description

Verify bounded process adapters preserve all hostile execution and protocol failure states.

## Test Procedure

Use controlled fake executables to emit sat, unsat, unknown, malformed, contradictory, oversized,
partial, slow, signaled, and nonzero-exit responses. Spawn descendants, cancel executions, exceed
each resource bound, and use executable paths and arguments containing shell metacharacters. Run
independently configured Z3 and cvc5 paths without network access and inspect built dependencies.

Run timeout and cancellation cleanup three times each. For every repetition, record cleanup start and
completion with a monotonic clock, probe the isolated process group after cleanup, and retain the six
durations plus their maximum. Fail if any group member survives or any duration exceeds 1,000 ms.
Exercise exact-at-limit and one-byte-over-limit inputs for query, stdout, stderr, model, version
output, executable identity input, and canonical path. Validate the closed set of limit fields against
the profile census rather than relying on line coverage.

## Expected Results

Only complete valid sat/unsat responses with successful exit are eligible for conclusions. Every
other state is distinct, bounded, non-conclusive, and leaves no live child process. No shell
expansion occurs and no solver library is linked.
The retained Linux cleanup maximum is no greater than 1,000 ms; every non-Linux execution is reported as
an explicit platform limitation, not a pass.
