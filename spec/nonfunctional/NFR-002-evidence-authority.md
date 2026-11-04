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

Every analysis report shall retain its canonical proof-input identities and raw solver output, and
preserve non-conclusive states. PGM-01 envelope status remains unavailable until its owner
component is adopted. No automated output shall claim solver qualification, semantic
accreditation, source-release approval, certification, or project-specific assurance sufficiency.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Missing or mismatched canonical proof-input identities | 0 | 0 | schema and mutation tests |
| False conclusive classifications | 0 | 0 | fault and differential tests |
| Automated authority claims | 0 | 0 | inspection and forbidden-claim scan |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-002-AC-1 | Every report retains its schema and encoding, canonical clause, binding, request and query identities, configured engine and executable path, observed solver version when available, raw output and normalized result. Bound validation rejects any difference from the authoritative query or run. | Test (TC-007) |
| NFR-002-AC-2 | Unknown, unsupported, timeout, cancellation, absence, malformed output, and tool failure remain distinct. | Test (TC-005) |
| NFR-002-AC-3 | Standalone validation rejects a changed query with unchanged canonical query identity, omitted required artifacts, and contradictions between retained output and declared outcome. Validation bound to an authoritative run rejects raw-output substitution even when the substitute and its declared outcome are internally consistent. | Test (TC-007) |
| NFR-002-AC-4 | Human release and consuming-project qualification claims remain open. | Inspection |

## Verification

TC-005 verifies failure-state separation, TC-007 verifies schema, canonical proof-input identity,
census, outcome consistency, and authoritative-run comparison, and review inspects every automated
authority claim.

## Dependencies

PGM-01 R06 through R10 and FR-005.
