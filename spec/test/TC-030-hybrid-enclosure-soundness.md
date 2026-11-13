---
id: TC-030
title: "Verify hybrid enclosure soundness and disposition"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-032
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-030
    type: verifies
---
# TC-030: Verify hybrid enclosure soundness and disposition

## Description

Verify that no request makes the hybrid reachability provider return an enclosure that omits a reachable
state, and that the disposition recorded by the review is carried out.

## Test Procedure

For each FR-030 criterion, build requests that attack it and compare the provider's answer with an
independent enumeration of concrete executions on a small model (explicit states over the declared
horizon). Cover bounds one unit from the representable limit, a flowed interval touching a guard at one
point, two overlapping guards from one mode, two transitions into one target mode, a reset applied to
part of an interval, a declared invariant, a horizon equal to and above the step bound, a duplicate and
an undeclared mode, and inverted intervals and flow deltas. Run each request twice. Under
delegate-and-remove, drive a stub `producer-execute` through complete, stalled, flooding, non-zero-exit,
missing-tool and unparseable cases.

## Expected Results

Each enclosure contains every state the enumeration reaches. Each case that cannot be enclosed is
`Incomplete` or `Refused`. Both runs of one request agree. Under delegate-and-remove, only a complete
result maps to an enclosure.

## Limitations

The enumeration check covers bounded integer models small enough to enumerate. It does not prove
soundness for all models. The disposition criteria that FR-031 states are verified by inspection of the
review record, not by this case.
