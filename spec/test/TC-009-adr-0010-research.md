---
id: TC-009
title: "Verify ADR-0010 identity measurements and invalidation decision"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: verifies
---
# TC-009: Verify ADR-0010 identity measurements and invalidation decision

## Description

Reproduce the shared-variable candidate measurements and verify the accepted ADR names every
identity and invalidation input required by issue #6.

## Test Procedure

Exhaustively compare every pair in the retained 11-reference adversarial fixture under complete IR,
name-only, typed-name, and explicit-binding rules. Inject structurally incompatible binding members.
Inspect the ADR and research report for the versioned analysis model, SMT encoding, clause digest,
binding-set digest, encoding-profile identity, and supersession link.

## Expected Results

Candidate results are respectively `(0, 0, 2)`, `(2, 12, 0)`, `(2, 3, 0)`, and `(2, 0, 0)`.
Incompatible type, kind, observation, or execution-point members reject. Every version/hash input and
`agent-ix/quire-rs#164` replacement link is present.
