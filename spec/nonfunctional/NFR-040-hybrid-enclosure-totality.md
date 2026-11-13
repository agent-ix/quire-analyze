---
id: NFR-040
title: "Hybrid enclosure computation is total and bounded"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-040
    type: constrains
---
# NFR-040: Hybrid enclosure computation is total and bounded

## Statement

The hybrid reachability computation shall return a typed outcome for every request, without a panic,
wrapped arithmetic or allocation that grows beyond the declared step bound, and shall return the same
outcome for the same request.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Requests in the adversarial set that panic, wrap or exhaust memory | 0 | 0 | adversarial test set of FR-041 plus extreme-value requests |
| Requests whose two evaluations differ | 0 | 0 | repeat each request and compare outcomes |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-040-AC-1 | A request with a maximal horizon, extreme interval bounds or a very large number of transitions returns `Enclosure`, `Incomplete` or `Refused` within the step bound and does not panic or wrap. | Test (TC-040) |
| NFR-040-AC-2 | Two evaluations of one request return equal outcomes, including enclosure order. | Test (TC-040) |

## Verification

[TC-040](../test/TC-040-hybrid-enclosure-soundness.md) verifies NFR-040-AC-1 and NFR-040-AC-2.

## Dependencies

[FR-040](../functional/FR-040-hybrid-enclosure-soundness.md) and [NFR-001](./NFR-001-determinism-bounds.md).
