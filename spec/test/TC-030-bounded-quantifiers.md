---
id: TC-030
title: "Verify bounded quantifiers"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-031
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-032
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-033
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-034
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-030
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-031
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-032
    type: verifies
---
# TC-030: Verify bounded quantifiers

## Description

Verify that quantifiers over enum domains, finite collections and bounded integer ranges are
admitted, expanded or bounded, decided, replayed over their bound and translated from QSL packages,
and that every unbounded quantifier and every exhausted limit is non-conclusive.

## Test Procedure

1. Admission and refusal: universal and existential fixtures over each admitted domain (enum, each
   collection constructor, enum-literal collection, integer-literal collection, and an integer range at the lowering layer), an empty domain, and refused fixtures (unbounded integer,
   text, rational, unfixed collection, collection with an unencodable element, non-finite bound,
   nested refused quantifier, and an enum or collection expansion one above the budget refused as
   `expansion-ceiling`).
2. Lowering: expand enum and collection fixtures, check for no `forall` or `exists` token, lower
   range fixtures at and above the expansion ceiling, and lower each twice in separate processes.
3. Decision: each decidable fixture carries a hand-derived expected status. "Decided correctly" means
   the lowered query equals its golden query and the enumeration oracle of FR-033 yields the expected
   status; where Z3 and cvc5 are installed, both also answer the matching `sat`/`unsat`. The oracle
   and golden checks run everywhere, so the case can fail without installed solvers.
4. Replay: one passing and one failing model per domain form and quantifier form, bounds exactly at
   and one above the replay limit, and a model that satisfies the lowered query but fails one
   original instance.
5. Resources: instance budget and replay limit at and one above their values, a 2^64-wide range,
   the nested width-100 fixture, a stalled solver and a solver answering `unknown` on a quantified
   query. Distinguish replay exhaustion from solver exhaustion.
6. Package: a QSL contract package with admitted and refused quantifiers through `run-package`, including an integer-literal collection one above the budget and a range-domain operand with no upstream form (both exit `2`), and the Boolean and integer fixtures with no quantifier.

## Expected Results

Admitted fixtures match their golden queries and expected statuses; refused fixtures return
`unsupported` with their reason class and no query; repeated lowerings are byte-identical; replay
reports the first failing instance and returns `incomplete` with reason `replay_limit_exceeded` above
its instance limit, without evaluating more than the limit or publishing the model as verified.
Solver execution exhaustion yields `timeout` or `unknown` for that solver; it does not change the
replay-limit outcome.

## Limitations

Z3 and cvc5 are external solvers. Where they are not installed their cases are reported as not run,
not as passed, and the enumeration oracle, golden queries and fault-injected solver cover the remaining behavior.
