---
id: SR-5482
title: IR-357 PR 56 spec-review/integrity review
type: SpecReview
analysis: integrity
scope: agent-ix/quire-analyze@f313889b19d936a78c4319b1c910dabad413b8db; spec/functional/FR-002-smt-lowering.md; spec/functional/FR-005-evidence-cli.md; spec/functional/FR-005-evidence-cli.md; spec/assurance/AD-001-analyze-architecture.md; spec/interface/interface-001-analysis-api.md; spec/nonfunctional/NFR-002-evidence-authority.md
review_set: subset
---

## Summary

The changed documents consistently locate the dependency revision in Cargo metadata, keep v1 as current runtime behavior, and describe v2 as planned. No additional integrity defect beyond the base review finding was found.

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-002 | examined | The profile declares supported and unsupported constructs, logic and SMT-LIB version,   statement/expression/query byte limits, and named-assertion/model features. `Cargo.toml` declares   the `quire-contract-ir` dependency and `Cargo.lock` records its resolved source; the public   capability profile does not repeat that source revision. |
| FR-005 | examined | The planned `quire.differential-report/v2` closed contract removes the duplicated   `contractIrRevision` field. It retains the producer application name and version, complete clause   and source identities with canonical clause digests, request and query digests, exact query bytes   and their digest, raw solver-stream bytes and their digests, and the canonical report digest. |
| FR-005-AC-2 | examined | Every report validates against its versioned schema and PGM-01 evidence envelope. |
| AD-001 | examined | The current differential report is v1 and includes that repeated revision. A planned v2 closed report removes it while retaining canonical proof-to-content digests and producer application identity. No v2 renderer or validator exists yet; v2 has no v1 fallback. |
| interface_001 | examined | The current runtime report is `quire.differential-report/v1`; `quire.differential-report/v2` is a planned closed contract without `contractIrRevision`, retaining producer application name/version and canonical clause, request, query, raw-stream, and report content digests. The v2 renderer and validator are not implemented, and no v1 fallback is planned. |
| NFR-002-AC-1 | examined | Every report binds all material input, schema, encoding, engine, configuration, output, and source identities. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS** — The proposed v2 boundary and existing v1 behavior are not conflated. The base review separately records the missing criterion.
