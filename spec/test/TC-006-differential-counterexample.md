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

For the issue #4 slice, decode bounded Boolean models and independently re-evaluate every asserted
expression using the query's sealed replay map. Reject missing, duplicate, unknown, non-Boolean, and
predicate-refuting assignments as verified evidence. In Task-007, run pinned Z3 and cvc5 versions
over the seeded supported corpus, inject a controlled disagreement fixture, and verify every filed
semantic defect has a stable executable regression fixture and retained disposition.

The issue #5 corpus includes satisfied/refuted examples for all five kinds, unsupported lowering,
timeout, missing-engine, and controlled sat/unsat disagreement fixtures. Both engine records are
retained before disposition. Real-engine execution is a measured local lane and unavailability is a
result, never a skipped pass.

## Expected Results

Every published decoded counterexample satisfies the required truth condition. Later pinned engines
agree on supported cases; a disagreement retains both raw and normalized results and cannot be
classified as conclusive before human-reviewed adjudication. Every filed semantic defect remains
reproducible by its regression fixture.
