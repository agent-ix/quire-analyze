---
id: SR-5540
title: "IR-357 PR 57 code-review review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze@cce212918a4044eb4360c110e9dedfd11bd640a2; schemas/differential-report-v2.schema.json; spec/assurance/AD-001-analyze-architecture.md; spec/functional/FR-005-evidence-cli.md; spec/interface/interface-001-analysis-api.md; spec/test/TC-007-evidence-integrity.md; src/report.rs; src/smt.rs; tests/analysis_semantics.rs; tests/smt_lowering.rs"
review_set: subset
---

## Summary

Ticket: IR-357. Reviewed PR #57 at cce212918a4044eb4360c110e9dedfd11bd640a2 against fbbf42fe03b698d472ff90c791c375a8a794e58e. The v1 rejection fixture does not exercise a valid v1 report.

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-005-AC-6 | examined | When `quire.differential-report/v2` is implemented, its closed schema requires `producer.name` and `producer.version`; every `assertions[].clause`, `assertions[].source`, and `assertions[].clauseDigest`; `bindingSetDigest`, `requestDigest`, `queryDigest`, `queryHex`, `querySha256`, and `reportDigest`; and every engine's `stdoutHex`, `stdoutSha256`, `stderrHex`, `stderrSha256`, `modelHex`, and `modelSha256`. Standalone v2 validation verifies the query, raw-stream, model, and canonical report digests against retained bytes and derives `queryDigest` from retained `requestDigest` and que |
| TC-007 | examined | Add `contractIrRevision` to a v2 document and pass a valid v1 document to each v2 validation path; both paths must reject them. |
| AD-001 | examined | The differential report is a closed v2 contract without a copied dependency revision. |
| interface_001 | examined | The runtime report uses `quire.differential-report/v2`, a closed contract without `contractIrRevision`. |

## Assurance Context

AP-001 applies to this source candidate and requires code review and gap analysis. Evaluated the pinned PR head against its base, with AD-001 and TC-007 as available architecture and verification context. The supplied pre-PR full gate log reports passing Rust code gates and three baseline-invalid spec documents; independent release measurement, source qualification, and producer-reliance decisions are unavailable. No standing exception is claimed.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The v1 rejection fixture sets contractIrRevision to 40 zeroes, so it is invalid under the historical v1 validator, which required the pinned contract-IR revision. A compatibility reader that accepts genuine v1 reports but rejects this malformed fixture would leave the test green. Construct a valid historical v1 document, including its actual revision, before asserting both v2 validation paths reject it. | tests/analysis_semantics.rs:1321 |

## Verdict

**PASS after disposition** — FND-001 was fixed at 40539ad4fbdc1c2b6581fd6206e7141ffff63377. The original finding remains above as the review record.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 40539ad4fbdc1c2b6581fd6206e7141ffff63377 |

The fix adds a canonical historical v1 fixture whose original revision and identity digests check against the pre-change contract, then asserts both current v2 validation paths refuse it. Focused TC-007 test passed on the reviewed head.

After excerpt (tests/analysis_semantics.rs:1325):

```rust
    let historical = include_bytes!("fixtures/differential-report-v1.json");
    let historical_value: serde_json::Value =
        serde_json::from_slice(historical).expect("historical v1 fixture");
    assert_eq!(historical_value["schema"], "quire.differential-report/v1");
    assert!(historical_value["contractIrRevision"].is_string());
    assert_eq!(reseal_report(historical_value.clone()), historical);
    assert!(validate_report_document(historical).is_err());
    assert!(validate_differential_report(historical, &query, &run).is_err());
```
