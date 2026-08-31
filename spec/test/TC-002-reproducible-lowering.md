---
id: TC-002
title: "Verify canonical requests and deterministic lowering"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-001
    type: verifies
---
# TC-002: Verify canonical requests and deterministic lowering

## Description

Verify canonical identity and lowering are stable under repetition and irrelevant ordering.

## Test Procedure

Repeat lowering while permuting irrelevant input, map, declaration, and filesystem order. Mutate one
material request field at a time. Compare request, statement, query, assertion-map, and report bytes.

## Expected Results

Irrelevant permutations change no byte. Every material semantic mutation changes the applicable
digest. Host paths, locale, wall clock, and randomized map order do not enter canonical outputs.
