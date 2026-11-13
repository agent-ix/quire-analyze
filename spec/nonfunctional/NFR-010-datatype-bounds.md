---
id: NFR-010
title: "Bounded and deterministic datatype encoding"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-020
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-021
    type: constrains
---
# NFR-010: Bounded and deterministic datatype encoding

## Statement

The datatype encoding of [FR-020](../functional/FR-020-datatype-lowering.md) and the model parsing of
[FR-021](../functional/FR-021-datatype-witness-replay.md) shall be finite, bounded and deterministic:
the same environment and statements yield byte-identical queries, and any datatype that exceeds a
bound below is refused and never yields a conclusive result.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Variants per enum | at most 256 | 256 | exact boundary test (256 accepted, 257 refused) |
| Fields per record | at most 64 | 64 | exact boundary test |
| Datatype nesting depth (record, option, enum) | at most 8 | 8 | exact boundary test, in lowering and in model parsing |
| Datatypes declared per query | at most 256 | 256 | exact boundary test |
| Query byte differences across repetition and order permutation | 0 | 0 | repeated and permuted-order testing |
| Conclusive results after a datatype bound breach | 0 | 0 | negative testing |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-010-AC-1 | Each bound has an exact boundary test: the limit is accepted and the limit plus one is refused as an explicit limit breach before solver execution. | Test (TC-020) |
| NFR-010-AC-2 | Repetition and permutation of environment, variant, field and statement order preserve query bytes. | Test (TC-020) |
| NFR-010-AC-3 | A model nested beyond the depth bound is rejected without unbounded recursion or allocation. | Test (TC-020) |

## Verification

TC-020 exercises each boundary and the permutation determinism.

## Dependencies

FR-020 and FR-021.
