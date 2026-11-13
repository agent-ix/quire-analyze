---
id: TC-017
title: "Verify integer witness model replay"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-011
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-004
    type: verifies
---
# TC-017: Verify integer witness model replay

## Description

Verify parsing and replay of integer models, one case per verified and per typed-failure outcome.

## Test Procedure

Feed recorded Z3-format and cvc5-format model texts for integer fixtures, then mutate each: drop a
symbol, add an undeclared symbol, repeat a symbol, change a sort, push a value over each bound, falsify
one assertion, supply a decimal, a real and an over-long numeral, and force an overflowing evaluation at the 128-bit extremes, echo named-assertion aliases, and combine two defects to check the reason precedence of FR-011.
Mutate the query text and show the verdict does not move. Where solvers are installed, replay the
models they actually return for the satisfiable fixtures.

## Expected Results

Only the unmutated models verify. Each mutation yields the stated reason and an incomplete witness
with the logical status retained. No case panics.

## Limitations

Real-solver cases are skipped visibly where Z3 or cvc5 is absent.
