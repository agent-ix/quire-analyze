---
id: StR-001
title: "Reviewable contract consistency and implication analysis"
type: StR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: satisfied_by
---
# StR-001: Reviewable contract consistency and implication analysis

## Stakeholder Need

Assurance engineers require reproducible consistency and implication conclusions from one pinned
contract package while every unsupported feature, approximation, timeout, cancellation, tool
failure, and unknown solver response remains visible and non-conclusive.

## Rationale

An SMT answer without exact encoding, assumptions, resource bounds, engine identity, and source
mapping cannot be audited. Converting infrastructure failure or incomplete semantics into a proof
would create false assurance.

## Validation Criteria

| ID | Criteria | Validation |
|---|---|---|
| StR-001-VC-1 | Repeated analysis of one pinned request yields byte-identical query and normalized evidence bytes. | Demonstration |
| StR-001-VC-2 | Z3 and cvc5 agree on the supported seeded corpus, or retain a typed discrepancy without a conclusive claim. | Demonstration |

## Dependencies

PGM-01 at `ix://agent-ix/quire-contract-ir/PGM-01` governs compatibility, evidence, authority, and
qualification. Contract-IR issues #8 and #10 provide the accepted semantic and corpus substrate.
