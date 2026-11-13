---
id: NFR-032
title: "Solver exhaustion on a quantified query is inconclusive"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-032
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-003
    type: constrains
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# NFR-032: Solver exhaustion on a quantified query is inconclusive

## Statement

If a solver exhausts a resource limit or answers `unknown` on a quantified query, the analyzer shall
report `timeout` or `unknown` respectively and shall not report `satisfied` or `refuted`.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Quantified queries that exhaust a limit or answer `unknown` and yield a conclusive status | 0 | 0 | fault tests with a stalled solver and a solver answering `unknown` |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-032-AC-1 | A fault-injected solver that exceeds its wall-time limit on a bounded-quantifier query yields `timeout`, never `satisfied` or `refuted`. | Test (TC-030) |
| NFR-032-AC-2 | A fault-injected solver that answers `unknown` on a bounded-quantifier query yields `unknown`, never `satisfied` or `refuted`. | Test (TC-030) |
| NFR-032-AC-3 | Where Z3 and cvc5 are installed, a bounded-quantifier fixture that a solver cannot decide within its limits is reported `timeout` or `unknown` for that solver; where they are absent the case is reported as not run. | Test (TC-030; real solvers where installed) |

## Verification

[TC-030](../test/TC-030-bounded-quantifiers.md) verifies all three criteria.

## Dependencies

[FR-003](../functional/FR-003-bounded-adapters.md) and
[FR-004](../functional/FR-004-conclusions.md).
