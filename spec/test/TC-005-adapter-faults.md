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

## Expected Results

Only complete valid sat/unsat responses with successful exit are eligible for conclusions. Every
other state is distinct, bounded, non-conclusive, and leaves no live child process. No shell
expansion occurs and no solver library is linked.
