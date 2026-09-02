---
id: Task-007
title: "Evidence differential verification and CLI"
type: Task
status: in_progress
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

## Plan Delta

The application-owned `quire.analysis-report/v1` and `quire.differential-report/v1` formats remain
in this crate because they are runtime product outputs, not assurance collectors. Retained suite-run
transcription and audit adopt Quoin 0.23.1; no local collector, envelope builder, or verifier script
is added. Because `quire-contract-ir#20` has not selected the shared PGM-01 envelope/integrity
component, that envelope lane remains explicitly `unavailable` and cannot discharge FR-005-AC-2 or
advance this task out of draft. The other issue #5 deliverables proceed independently.
