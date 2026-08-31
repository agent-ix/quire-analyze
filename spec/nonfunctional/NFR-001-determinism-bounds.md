---
id: NFR-001
title: "Deterministic and bounded analysis"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-003
    type: constrains
---
# NFR-001: Deterministic and bounded analysis

## Statement

For one pinned package, request, encoding, engine, and configuration, the analyzer shall produce
byte-identical query and normalized evidence bytes, and every memory, I/O, process, and time boundary
owned by the adapter shall be finite and enforced.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Query or normalized-report digest differences | 0 | 0 | repeated and permuted-order testing |
| Owned execution paths without declared limits | 0 | 0 | interface and code inspection |
| Processes surviving timeout/cancellation cleanup | 0 | 0 | process-tree fault injection |
| Conclusive results after truncation or limit breach | 0 | 0 | negative testing |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-001-AC-1 | Repetition and irrelevant-order permutation preserve query and normalized evidence bytes. | Test (TC-002) |
| NFR-001-AC-2 | Every process, time, input, output, model, and recursion boundary has a tested finite limit. | Test (TC-005) |
| NFR-001-AC-3 | A breached limit is explicit and never conclusive. | Test (TC-005) |

## Verification

TC-002 exercises repeated and permuted-order determinism. TC-005 injects every owned resource and
process failure boundary and verifies cleanup plus non-conclusive classification.

## Dependencies

FR-002 and FR-003.
