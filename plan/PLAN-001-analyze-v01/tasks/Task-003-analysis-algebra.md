---
id: Task-003
title: "Analysis algebra identity and ADR-0010"
type: Task
status: done
track: B
priority: P0
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: references
---
# Task-003: Analysis algebra, identity, and ADR-0010

## Scope

Complete native issue #6: reconcile the exact IR dependency, decide shared-variable and encoding
semantics, implement canonical request/statement identities, verify TC-001 through TC-003 scope, and
link the accepted ADR as the explicit replacement that supersedes `quire-rs#164`.

## Guard

Tasks 001 and 002 must be done. The exact IR revision, schemas, corpus, and lockfile must agree.

## Current Evidence

ADR-0010 accepts the minimal analysis algebra, explicit cross-requirement binding groups, and
versioned invalidation rules. REV-004 retains the reproduced dual encoding and corpus/fixture
measurements. TC-009 executes the identity comparison and incompatible-binding controls.

The source-bound local outcomes and eight closed review findings are retained at
`evidence/adr-0010-2690c25/validation-summary.md`.
