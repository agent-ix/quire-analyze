---
id: NFR-030
title: "Quantifier expansion is bounded by an instance budget"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-031
    type: constrains
  - target: ix://agent-ix/quire-analyze/FR-032
    type: constrains
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# NFR-030: Quantifier expansion is bounded by an instance budget

## Statement

The analyzer shall expand quantifier instances up to a budget of 1,024 instances per request and shall
count instances in the single unit defined here.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Expanded instances in one request above the budget | 0 | 0 | boundary tests at and above the budget |
| Counting results that wrap on overflow | 0 | 0 | tests with a 2^64-wide range and nested products |

## Definitions

- **Instance.** One substitution of a domain element for the bound variable of one expanded
  quantifier.
- **Instance count of a quantifier.** Its domain size multiplied by the instance counts of every
  enclosing expanded quantifier (the number of copies of its body that expansion produces).
- **Budget.** The sum of the instance counts of all expanded quantifiers in one request, over all
  clauses. A bounded solver quantifier ([FR-032](../functional/FR-032-bounded-integer-range-quantifier-lowering.md))
  is not expanded and consumes none. Quantifiers are considered outermost first in canonical order: statements in ascending assertion-symbol order and, within a statement, depth-first pre-order (a quantifier before the quantifiers in its body, siblings left to right).
- Example: an outer literal range of width 100 has instance count 100 and is expanded. A literal
  range of width 100 inside it would have instance count 100 x 100 = 10,000, above the 924 left, so
  it is emitted as a bounded quantifier; an enum or collection quantifier in that place is refused
  as `unsupported` with reason `expansion-ceiling`.

## Rationale

The 1,024 budget keeps the average expanded instance within 16,384 bytes of the 16,777,216-byte query
ceiling of [NFR-001](./NFR-001-determinism-bounds.md); a query that still exceeds that byte ceiling or
the lowering's `MAX_EXPRESSION_NODES` is refused by those existing limits, not by this budget. The
budget does not interact with the 10,000 and 100,000 `Expression` node bounds of
[FR-009](../functional/FR-009-package-runner.md), which apply to the translated tree where a quantifier
body appears once, before expansion.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-030-AC-1 | A request whose expanded instance counts sum to 1,024 lowers, and one summing to 1,025 refuses its enum or collection expansion as `unsupported` with reason `expansion-ceiling` and emits a literal range as a bounded quantifier. | Test (TC-030) |
| NFR-030-AC-2 | The nested fixture of the example (outer literal range of width 100, inner literal range of width 100) expands the outer range, emits the inner range as a bounded quantifier, and records 100 instances in the quantifier map. | Test (TC-030) |
| NFR-030-AC-3 | A literal range of width 2^64 and nested ranges whose instance-count product exceeds 2^128 are treated as exceeding the budget; the computation does not wrap or panic. | Test (TC-030) |
| NFR-030-AC-4 | Quantified lowering allocates in proportion to the checked instance count and never from an unchecked bound. | Inspection |

## Verification

[TC-030](../test/TC-030-bounded-quantifiers.md) verifies NFR-030-AC-1 through NFR-030-AC-3;
NFR-030-AC-4 is verified by inspection.

## Dependencies

[NFR-001](./NFR-001-determinism-bounds.md) and
[FR-032](../functional/FR-032-bounded-integer-range-quantifier-lowering.md).
