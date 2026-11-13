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

The hybrid reachability computation shall return a typed outcome for every request without a panic or wrapped arithmetic.

The hybrid reachability computation shall refuse or end `Incomplete` any request that exceeds the crate limits on step bound, transitions and distinct modes, before it allocates for them.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Requests in the adversarial set that panic or wrap | 0 | 0 | adversarial test set of FR-041 plus extreme-value requests |
| Requests above a crate limit that allocate beyond the limit before ending | 0 | 0 | requests at `u64::MAX` step bound and above the transition and mode limits, with allocation measured against the limit |
| Requests whose two evaluations differ | 0 | 0 | repeat each request and compare outcomes |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-040-AC-1 | The crate states hybrid limits for step bound, transitions and distinct modes, as the synthesis path states limits for terms and atoms. A request above any limit ends `Refused` or `Incomplete` before work proportional to the excess starts. A request at each limit returns an outcome without a panic or wrapped arithmetic. | Test (TC-040) |
| NFR-040-AC-2 | A request with a step bound of `u64::MAX`, a horizon of `u64::MAX` or extreme interval bounds returns an outcome in time proportional to the crate limits, not to the requested count. | Test (TC-040) |
| NFR-040-AC-3 | Two evaluations of one request return equal outcomes, including enclosure order. | Test (TC-040) |

## Verification

[TC-040](../test/TC-040-hybrid-enclosure-soundness.md) verifies NFR-040-AC-1, NFR-040-AC-2 and NFR-040-AC-3.

## Dependencies

[FR-040](../functional/FR-040-hybrid-enclosure-soundness.md) and [NFR-001](./NFR-001-determinism-bounds.md).
