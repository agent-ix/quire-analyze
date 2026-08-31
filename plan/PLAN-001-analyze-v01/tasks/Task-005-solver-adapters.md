---
id: Task-005
title: "Bounded Z3 and cvc5 adapters"
type: Task
status: in_progress
track: B
priority: P0
relationships:
  - target: ix://agent-ix/quire-analyze/FR-003
    type: references
---
# Task-005: Bounded Z3 and cvc5 adapters

## Scope

Complete native issue #3 with exact solver pins, argv-only process invocation, all resource limits,
protocol normalization, process-tree cleanup, and hostile fake-process tests.

## Guard

Task-004 must be done. No adapter may bypass the shared query bundle or construct conclusions directly.

## Verification

TC-005 owns exact profile-limit boundaries, protocol/failure classification, absolute-path and argv
isolation, executable identity, and three-repetition timeout/cancellation process-group cleanup with
all six durations and the maximum retained. Non-Unix targets fail explicitly until equivalent
process-tree containment is implemented and measured.
