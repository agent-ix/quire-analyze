---
id: NFR-002
title: "Evidence integrity and authority boundary"
type: NFR
quality_attribute: compliance
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: constrains
---
# NFR-002: Evidence integrity and authority boundary

## Statement

Every analysis artifact shall carry complete PGM-01 identity and preserve non-conclusive states. No
automated output shall claim solver qualification, semantic accreditation, source-release approval,
certification, or project-specific assurance sufficiency.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Missing or mismatched material identities | 0 | 0 | schema and mutation tests |
| False conclusive classifications | 0 | 0 | fault and differential tests |
| Automated authority claims | 0 | 0 | inspection and forbidden-claim scan |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-002-AC-1 | Every report binds all material input, schema, encoding, engine, configuration, output, and source identities. | Test (TC-007) |
| NFR-002-AC-2 | Unknown, unsupported, timeout, cancellation, absence, malformed output, and tool failure remain distinct. | Test (TC-005) |
| NFR-002-AC-3 | The evidence corpus detects changed bytes, omitted artifacts, and contradicted outcomes. | Test (TC-007) |
| NFR-002-AC-4 | Human release and consuming-project qualification claims remain open. | Inspection |

## Verification

TC-005 verifies failure-state separation, TC-007 verifies schema, identity, census, digest, and
outcome integrity, and review inspects every automated authority claim.

## Dependencies

PGM-01 R06 through R10 and FR-005.
