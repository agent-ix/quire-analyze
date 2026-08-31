---
id: TC-001
title: "Verify analysis algebra and truth table"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-004
    type: verifies
---
# TC-001: Verify analysis algebra and truth table

## Description

Verify the analysis algebra, shared-variable identity, and conclusion truth table independently of a solver.

## Test Procedure

Evaluate seeded finite Boolean and bounded-numeric models with an independent enumerator. Compare
consistency, contradiction, implication, redundancy, and dead-antecedent classifications across all
ten sat/unsat cases and explicit assumption groups. Exercise equal and near-collision variable
identities across packages, declarations, observations, and execution points.

## Expected Results

Every classification matches enumeration, equal complete identities share one variable, and every
unequal identity remains distinct.
