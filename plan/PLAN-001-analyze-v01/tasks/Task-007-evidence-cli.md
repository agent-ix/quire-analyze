---
id: Task-007
title: "Evidence differential verification and CLI"
type: Task
status: not_started
track: C
priority: P0
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: references
---
# Task-007: Evidence, differential verification, and CLI

## Scope

Complete native issue #5 with versioned report schemas, PGM-01 envelopes, mutation verification,
Z3/cvc5 differential evidence, library/CLI parity, atomic output, and stable exit classes.

## Guard

Task-006 must be done. Disagreement or unavailable comparison remains non-conclusive. Task-007 must
not create another repository-local envelope builder, collector, or verifier before
agent-ix/quire-contract-ir#20 selects the shared transcription and integrity component. The task must
adopt that reviewed component or record a reviewed plan delta explaining the remaining divergence.
