---
id: PLAN-001
title: "Contract analysis v0.1 implementation and release preparation"
type: Plan
status: active
relationships:
  - target: ix://agent-ix/quire-analyze/StR-001
    type: references
  - target: ix://agent-ix/quire-analyze/AP-001
    type: references
---
# PLAN-001: Contract analysis v0.1 implementation and release preparation

## Scope

Specify, implement, and verify deterministic bounded consistency and implication analysis over the
accepted contract IR, then hand an exact evidence-backed source candidate to the later human release wave.

## Dependency Graph

```text
Task-001 -> Task-002 -> Task-003 -> Task-004 -> Task-005 -> Task-006 -> Task-007 -> Task-008
                         ^
          PGM-01 + accepted IR issues #8/#10
```

## Task File Mapping

| Task | Native issue | Status |
|---|---|---|
| Task-001 | #2 foundation specification | done |
| Task-002 | #2 verification/evidence | done |
| Task-003 | #6 ADR-0010 algebra/identity | done |
| Task-004 | #7 deterministic SMT lowering | done |
| Task-005 | #3 bounded solver adapters | not_started |
| Task-006 | #4 analyses/counterexamples | not_started |
| Task-007 | #5 evidence/differential/CLI | not_started |
| Task-008 | #8 epic verification/human handoff | not_started |

## Guard

Only one semantic task advances at a time in DAG order unless a reviewed plan delta proves independence.
All implementation children remain Backlog while Task-001/002 are incomplete. Task-008 may prepare
epic evidence but cannot make the PGM-02 Wave 4 source-release decision.
