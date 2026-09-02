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

The analyzer shall translate a normalized solver response into a typed consistency, contradiction,
implication, redundancy, or dead-antecedent conclusion and, when applicable, a source-mapped model
or counterexample.

## Behavior

- Classification follows the five predicates in FR-001: consistency alone treats `sat` as
  `satisfied`; contradiction, implication, redundancy, and dead antecedent treat `unsat` as
  `satisfied`. The opposite recognized answer is `refuted` with the corresponding shared,
  counterexample, distinguishing, or activation model.
- Models are decoded only through the query assertion/symbol map and checked against type and bound
  constraints before publication.
- An invalid, incomplete, or undecodable model keeps the logical response but marks the requested
  counterexample evidence incomplete; it never fabricates a source value.
- The closed public statuses are `satisfied`, `refuted`, `unknown`, `unsupported`, `timeout`, and
  `tool-error`; only the first two are conclusive. Solver `unknown` maps to `unknown`; wall timeout
  and cancellation map to `timeout` with the exact adapter outcome retained; unsupported platform
  maps to `unsupported`; every other adapter failure maps to `tool-error`.
- A verified Boolean model must define every declared query symbol exactly once, define no unknown
  symbol, use only Boolean constants, and make every asserted replay expression true. Published
  values preserve every complete origin identity and explicit binding group from the query map.
- Every conclusion retains the analysis kind, canonical analysis request and query identities,
  ordered role/source maps, the sealed solver record, and either a replay-verified model or an
  explicit absent/incomplete explanation state.
- Unsat cores and proofs are optional evidence and never replace the checked primary response.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-004-AC-1 | All ten sat/unsat-by-analysis-kind classifications match the independent truth table. | Test (TC-001) |
| FR-004-AC-2 | Every published Boolean model/counterexample defines the complete symbol set and re-evaluates every asserted expression true; incomplete or invalid models are never represented as verified. | Test (TC-006) |
| FR-004-AC-3 | Source mapping preserves package, clause, revision, declaration, observation, and span identity. | Test (TC-004) |
| FR-004-AC-4 | Missing optional explanation data cannot be represented as present or verified, while the exact logical status and adapter outcome remain available. | Test (TC-007) |
| FR-004-AC-5 | Seeded contradictions and dead antecedents are detected without false success on timeout. | Test (TC-005, TC-006) |

## Dependencies

FR-003 and native issue #4.
