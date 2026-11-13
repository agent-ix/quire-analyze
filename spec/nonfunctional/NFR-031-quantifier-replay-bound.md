---
id: NFR-031
title: "Quantifier replay is bounded by an instance limit"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-033
    type: constrains
---
# NFR-031: Quantifier replay is bounded by an instance limit

## Statement

The analyzer shall evaluate at most 1,048,576 quantifier instances per request during replay, counted
in the instance unit of [NFR-030](./NFR-030-quantifier-expansion-bound.md), and shall report a request
that needs more as `incomplete` with reason `replay_limit_exceeded`. An over-limit replay is never
reported as `verified`.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Replays that evaluate more than the limit | 0 | 0 | boundary tests at and above the limit |
| Over-limit replays reported `verified` | 0 | 0 | the same tests |

## Rationale

Replay is in-process evaluation with no solver round trip, so its limit is 1,024 times the expansion
budget (2^20 against 2^10). Unlike expansion, every quantifier is enumerated, including those the
lowering emitted as bounded solver quantifiers, and nested instances multiply. Counting uses checked
arithmetic, and an overflow counts as exceeding the limit.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-031-AC-1 | A request needing exactly 1,048,576 instances replays fully; one needing 1,048,577 yields `incomplete` with reason `replay_limit_exceeded`, has evaluated no more than 1,048,576 instances, and is never `verified`. | Test (TC-030) |
| NFR-031-AC-2 | A 2^64-wide range and nested ranges whose instance-count product overflows yield `incomplete` with reason `replay_limit_exceeded`, without wrapping or panicking. | Test (TC-030) |

## Verification

[TC-030](../test/TC-030-bounded-quantifiers.md) verifies both criteria.

## Dependencies

[NFR-030](./NFR-030-quantifier-expansion-bound.md) and
[FR-033](../functional/FR-033-quantified-assertion-replay.md).
