---
id: NFR-020
title: "Bounded and deterministic datatype encoding"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-020
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-021
    type: constrains
---
# NFR-020: Bounded and deterministic datatype encoding

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

## Definitions and rationale

- **Nesting depth** is the longest chain of datatypes in which each is a field type or option payload
  of the previous one, counting the outermost datatype as depth 1. An enum alone is depth 1; a record
  with an option field whose payload is an enum is depth 3. Model parsing applies the same count to
  the value's constructor nesting.
- **Datatypes per query** is the number of distinct datatype declarations emitted: each enum, each
  record and each distinct option payload sort counts once.
- The bounds are powers of two sized above realistic role and classification models and chosen so the
  worst-case declaration text stays well inside the existing ceilings: 256 datatypes of 256 variants of
  about 64 bytes each is about 4 MiB, under the 16,777,216-byte query limit of NFR-001, and depth 8
  keeps the recursive parser and evaluator far from the 128-level expression depth.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-020-AC-1 | Each bound has an exact boundary test: the limit is accepted and the limit plus one is refused as an explicit limit breach before solver execution. | Test |
| NFR-020-AC-2 | Repetition and permutation of environment, variant, field and statement order preserve query bytes. | Test |
| NFR-020-AC-3 | A model nested beyond the depth bound is rejected without unbounded recursion or allocation. | Test |

## Verification

TC-020 exercises each boundary and the permutation determinism.

## Dependencies

FR-020 and FR-021.
