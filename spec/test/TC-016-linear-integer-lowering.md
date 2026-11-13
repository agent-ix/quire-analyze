---
id: TC-016
title: "Verify linear integer lowering and solver agreement"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-010
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-004
    type: verifies
---
# TC-016: Verify linear integer lowering and solver agreement

## Description

Verify the `QF_LIA` lowering, the unsupported classification of non-linear integer terms, determinism,
and decisions of Z3 and cvc5 over integer fixtures.

## Test Procedure

Lower hand-built integer analysis models and compare against a retained golden query; permute
declaration and statement order; lower each unsupported construct; lower the Boolean fixtures of
TC-010 under the integer profile. With controlled solver responses, classify `sat` against `unsat`.
Where Z3 and cvc5 are installed, run the unsatisfiable fixture on each through the adapters and
compare for agreement, and run the satisfiable fixture to show both answer `sat` and each returned model
replay-verifies, so the comparison agrees (FR-011-AC-11, TC-017). Where either is absent the real-solver cases are skipped visibly and the matrix
row is not marked complete.

## Expected Results

Golden queries are byte-identical to production output, unsupported constructs return the stable
diagnostic and no query, and both real solvers decide the fixtures identically, with agreement only on the unsatisfiable fixture.

## Limitations

TC-016 does not cover model replay or agreement on a satisfiable integer fixture (TC-017) or package translation (TC-018).
