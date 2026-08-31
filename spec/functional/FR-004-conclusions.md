---
id: FR-004
title: "Classify analyses and map counterexamples"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-003
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface-001
    type: implements
---
# FR-004: Classify analyses and map counterexamples

## Description

The analyzer shall translate a normalized solver response into a typed consistency or implication
conclusion and, when applicable, a source-mapped counterexample.

## Behavior

- Consistency `sat` is `satisfied` with a witness; `unsat` is `refuted` with an unsat explanation.
- Implication `unsat` is `satisfied`; `sat` is `refuted` with an antecedent-satisfying,
  consequent-falsifying counterexample.
- Models are decoded only through the query assertion/symbol map and checked against type and bound
  constraints before publication.
- An invalid, incomplete, or undecodable model keeps the logical response but marks the requested
  counterexample evidence incomplete; it never fabricates a source value.
- Unsat cores and proofs are optional evidence and never replace the checked primary response.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-004-AC-1 | The four sat/unsat-by-analysis-kind classifications match the independent truth table. | Test (TC-001) |
| FR-004-AC-2 | Every published counterexample re-evaluates against the authoritative semantics. | Test (TC-006) |
| FR-004-AC-3 | Source mapping preserves package, clause, revision, declaration, observation, and span identity. | Test (TC-004) |
| FR-004-AC-4 | Missing optional explanation data cannot be represented as present or verified. | Test (TC-007) |

## Dependencies

FR-003 and native issue #4.
