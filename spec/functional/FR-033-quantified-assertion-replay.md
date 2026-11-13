---
id: FR-033
title: "Replay quantified assertions over their bound"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-004
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-011
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-021
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-033: Replay quantified assertions over their bound

## Description

The analyzer shall check a solver model against every bounded quantified assertion by enumerating the
quantifier's bound, so that a model is `verified` only when the assertion holds for every instance.

## Behavior

- The replay evaluator is the analyzer's in-process evaluator over typed analysis expressions, the one
  that [FR-004](./FR-004-conclusions.md), [FR-011](./FR-011-integer-witness-replay.md) and
  [FR-021](./FR-021-datatype-witness-replay.md) already use for Boolean, integer and datatype values;
  this requirement adds quantifier enumeration to it. It calls no solver.
- Replay enumerates the domain in domain order and evaluates the body with the model's values and the
  element substituted (capture-avoiding, as in
  [FR-031](./FR-031-enum-and-collection-quantifier-lowering.md)).
- A universal quantifier holds when every instance holds; replay stops at, and reports, the first
  failing instance with its element and the quantifier's source identity, and enumerates the whole
  domain only when no instance fails. An existential quantifier holds when at least one instance
  holds; when none holds, replay reports that no instance held and the number enumerated.
- Replay evaluates the original quantifier, not the lowered expansion, and does not trust the
  solver's quantifier instantiation.
- For a range with declared bounds, replay takes the bound values from the model. A bound symbol that
  is missing, repeated, undecodable or outside its declared integer type yields `incomplete` with
  reason `bound_undecodable` naming the symbol. A model whose lower bound exceeds its upper bound
  gives an empty range, which follows the empty-domain rule of
  [FR-030](./FR-030-bounded-quantifier-domains.md); it is not a failure.
- Integer evaluation uses exact arithmetic that detects overflow as
  [FR-011](./FR-011-integer-witness-replay.md) states; an overflow in a body yields `incomplete`
  with reason `evaluation_overflow`.
- The replay limit and the counting unit are those of
  [NFR-031](../nonfunctional/NFR-031-quantifier-replay-bound.md). Before evaluating an instance that
  would exceed the remaining limit, replay yields `incomplete` with reason `replay_limit_exceeded`;
  the over-limit instance is not evaluated and the model is never `verified`. Checked-count overflow
  is treated as exceeding the limit.
- A failed or incomplete replay never publishes the model as verified. The logical solver response
  and the replay outcome are retained separately.
- Enumeration oracle: for a fixture with a small finite input space, the evaluator run over every
  assignment of the inputs yields the fixture's `sat`/`unsat` status independently of any solver.
  Tests use it as the expected-status check of [TC-030](../test/TC-030-bounded-quantifiers.md).

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-033-AC-1 | For a failing universal model, replay evaluates the domain elements in order up to the first failing element, reports that element and the source identity, and evaluates no later element; for a passing universal model it evaluates exactly as many instances as the domain has elements. | Test (TC-030) |
| FR-033-AC-2 | Replay of an existential quantifier succeeds when one instance holds and reports no holding instance and the count enumerated when none does. | Test (TC-030) |
| FR-033-AC-3 | A bound whose next instance would exceed the limit in NFR-031 yields `incomplete` with reason `replay_limit_exceeded`; that instance is not evaluated and the model is never `verified`. | Test (TC-030) |
| FR-033-AC-4 | For each of an enum domain, a finite collection and an integer range, and for each of universal and existential, one passing model and one failing model are replayed to the expected outcome. | Test (TC-030) |
| FR-033-AC-5 | A solver model that satisfies the lowered query but fails one instance of the original quantifier is not published as verified. | Test (TC-030) |
| FR-033-AC-6 | A declared range bound that is missing from the model, repeated, malformed or outside its declared type yields `incomplete` with reason `bound_undecodable`; a model with lower bound above upper bound replays as an empty range. | Test (TC-030) |
| FR-033-AC-7 | A body whose evaluation overflows yields `incomplete` with reason `evaluation_overflow`, never a wrapped value. | Test (TC-030) |
| FR-033-AC-8 | Where Z3 and cvc5 are installed, each satisfiable quantified fixture in TC-030 is decided `sat` by both solvers with agreement, and each returned model is published as `verified` only after replay passes. | Test (TC-030, real solvers) |

## Dependencies

[FR-030](./FR-030-bounded-quantifier-domains.md), [FR-004](./FR-004-conclusions.md),
[FR-011](./FR-011-integer-witness-replay.md) and [FR-021](./FR-021-datatype-witness-replay.md).
