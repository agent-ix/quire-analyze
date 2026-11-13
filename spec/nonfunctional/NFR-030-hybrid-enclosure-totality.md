---
id: NFR-030
title: "Hybrid enclosure computation is total and bounded"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: constrains
---
# NFR-030: Hybrid enclosure computation is total and bounded

## Statement

The hybrid reachability computation shall return a typed outcome for every request, without a panic,
wrapped arithmetic or allocation that grows beyond the declared step bound, and shall return the same
outcome for the same request.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Requests in the adversarial set that panic, wrap or exhaust memory | 0 | 0 | adversarial test set of FR-031 plus extreme-value requests |
| Requests whose two evaluations differ | 0 | 0 | repeat each request and compare outcomes |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-030-AC-1 | A request with a maximal horizon, extreme interval bounds or a very large number of transitions returns `Enclosure`, `Incomplete` or `Refused` within the step bound and does not panic or wrap. | Test (TC-030) |
| NFR-030-AC-2 | Two evaluations of one request return equal outcomes, including enclosure order. | Test (TC-030) |

## Verification

[TC-030](../test/TC-030-hybrid-enclosure-soundness.md) verifies NFR-030-AC-1 and NFR-030-AC-2.

## Dependencies

[FR-030](../functional/FR-030-hybrid-enclosure-soundness.md) and [NFR-001](./NFR-001-determinism-bounds.md).
