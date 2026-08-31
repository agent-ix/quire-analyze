---
id: TC-003
title: "Reject invalid, unsupported, and approximate requests"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-002
    type: verifies
---
# TC-003: Reject invalid, unsupported, and approximate requests

## Description

Verify every invalid, unsupported, or approximate request fails before solver execution.

## Test Procedure

Generate positive and negative fixtures for every public IR construct and boundary. Inject stale
identities, non-Boolean roots, incompatible execution points, missing bounds, unsupported constructs,
unknown profiles, and hypothetical approximation requests.

## Expected Results

Exact supported constructs lower successfully. Every other case returns a stable ordered diagnostic
before execution and cannot carry a conclusive result.
