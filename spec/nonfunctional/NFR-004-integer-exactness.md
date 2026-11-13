---
id: NFR-004
title: "Exact and bounded integer analysis"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-010
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-011
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-012
    type: constrains
---
# NFR-004: Exact and bounded integer analysis

## Statement

For integer-typed analysis, the analyzer shall return `unsupported` for any term it cannot encode
exactly.

For integer-typed analysis, the analyzer shall publish a model as verified only when replay passes.

For integer-typed analysis, the analyzer shall parse and evaluate every integer within declared bounds,
so that no input causes a panic, a wrapped value, an approximated result or an unbounded computation.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Conclusive results from an approximated or clamped integer term | 0 | 0 | negative tests over non-linear, saturating, division and out-of-range fixtures |
| Verified witnesses whose replay did not pass | 0 | 0 | model-mutation tests over each replay failure reason |
| Panics or wrapped values on hostile integer numerals | 0 | 0 | boundary tests on literal, bound, model-value and arithmetic extremes |
| Integer query and normalized-report byte differences across repetition and order permutation | 0 | 0 | repeated and permuted-order testing |
| Integer model text within the parsed-model ceiling of NFR-001 | at most 8,388,608 bytes | 8,388,608 bytes | oversized-model test with integer symbols |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-004-AC-1 | Every unsupported integer construct of FR-010 returns `unsupported` and none yields a query, a `satisfied` result or a `refuted` result. | Test (TC-016) |
| NFR-004-AC-2 | Integer literals and bounds at the minimum and maximum of the 128-bit signed range lower and replay without panic; a unit test over an unchecked replay expression (not an admitted fixture) whose exact result exceeds the 128-bit signed range fails replay with `evaluation_overflow`, as FR-011-AC-7 states. | Test (TC-016, TC-017) |
| NFR-004-AC-3 | An integer model above the parsed-model ceiling is rejected as non-conclusive by the adapter bounds of NFR-001. | Test (TC-017) |
| NFR-004-AC-4 | Repeated and order-permuted integer analyses preserve query and normalized evidence bytes. | Test (TC-016) |

## Verification

TC-016 covers lowering, unsupported classification and determinism; TC-017 covers replay and parsing
bounds. Criteria that need real solvers run where Z3 and cvc5 are installed.

## Dependencies

FR-010, FR-011, FR-012 and NFR-001.
