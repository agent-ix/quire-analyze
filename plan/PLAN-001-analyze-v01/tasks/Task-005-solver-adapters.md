---
id: Task-005
title: "Bounded Z3 and cvc5 adapters"
type: Task
status: not_started
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
