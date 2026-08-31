---
id: TC-006
title: "Verify engine parity and counterexamples"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-003
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-004
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-005
    type: verifies
---
# TC-006: Verify engine parity and counterexamples

## Description

Verify pinned engines agree on supported cases and every published counterexample replays.

## Test Procedure

Run pinned Z3 and cvc5 versions over the seeded supported corpus. Independently re-evaluate every
model against authoritative contract semantics. Inject a controlled disagreement fixture.

## Expected Results

Supported cases agree and decoded counterexamples satisfy the required truth condition. A
disagreement retains both raw and normalized results and cannot be classified as agreement.
