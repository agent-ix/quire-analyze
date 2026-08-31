---
id: TC-007
title: "Verify evidence schemas, identities, and truthfulness"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-002
    type: verifies
---
# TC-007: Verify evidence schemas, identities, and truthfulness

## Description

Verify retained evidence detects missing, altered, extra, and contradicted records.

## Test Procedure

Validate every report and envelope against pinned schemas. Remove and mutate each required identity,
artifact, digest, status, limitation, engine field, and output. Contradict raw responses and declared
outcomes, and alter the artifact census.

## Expected Results

The valid corpus passes. Every omission, mutation, contradiction, or census change fails closed with
a stable diagnostic. Optional explanation evidence is never inferred as present.
