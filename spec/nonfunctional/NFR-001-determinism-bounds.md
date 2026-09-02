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
byte-identical query and normalized semantic outcome bytes, and every memory, I/O, process, and time
boundary owned by the adapter shall be finite and enforced. Observational elapsed times are retained
in the execution record but excluded from the deterministic semantic-outcome projection.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Query or normalized-report digest differences | 0 | 0 | repeated and permuted-order testing |
| Owned execution paths without declared limits | 0 | 0 | closed limit-field census and code inspection |
| Processes surviving timeout/cancellation cleanup | 0 at 1,000 ms | 0 | three-repetition process-group fault injection; retain each cleanup duration and maximum |
| Conclusive results after truncation or limit breach | 0 | 0 | negative testing |
| Query input | at most 16,777,216 bytes | 16,777,216 bytes | exact boundary tests |
| Captured stdout / stderr | at most 16,777,216 / 1,048,576 bytes | profile ceilings | hostile stream tests |
| Parsed model within stdout | at most 8,388,608 bytes | 8,388,608 bytes | oversized-model test |
| Solver execution / graceful cleanup / total cleanup | at most 5,000 / 100 / 1,000 ms | profile ceilings | monotonic-clock measurement |
| Version output / executable bytes / canonical path | at most 65,536 / 536,870,912 / 4,096 bytes | profile ceilings | boundary tests and metadata inspection |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-001-AC-1 | Repetition and irrelevant-order permutation preserve query and normalized evidence bytes. | Test (TC-002) |
| NFR-001-AC-2 | Every adapter process, time, input, output, model, executable-identity, and path boundary has a numeric limit and exact boundary test; lowering recursion remains owned by TC-010. | Test (TC-005, TC-010) |
| NFR-001-AC-3 | A breached limit is explicit and never conclusive. | Test (TC-005) |

## Verification

TC-002 exercises repeated and permuted-order determinism. TC-005 injects every owned resource and
process failure boundary and verifies cleanup plus non-conclusive classification.

## Dependencies

FR-002 and FR-003.
