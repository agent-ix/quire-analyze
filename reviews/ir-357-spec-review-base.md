---
id: SR-5481
title: IR-357 PR 56 spec-review/base review
type: SpecReview
analysis: base
scope: agent-ix/quire-analyze@f313889b19d936a78c4319b1c910dabad413b8db; spec/functional/FR-002-smt-lowering.md; spec/functional/FR-005-evidence-cli.md; spec/functional/FR-005-evidence-cli.md; spec/assurance/AD-001-analyze-architecture.md; spec/interface/interface-001-analysis-api.md; spec/nonfunctional/NFR-002-evidence-authority.md
review_set: base
---

## Summary

The planned v2 field and validation obligations are coherent with the current v1 schema and renderer. The existing acceptance criteria do not specifically verify the new closed v2 behavior.

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
| FND-001 | medium | FR-005-AC-2 only requires validation against a versioned schema and PGM-01 envelope. A future v2 schema could still require `contractIrRevision`, omit a retained digest, or accept v1 through a fallback and satisfy that criterion. Add a v2-specific criterion that rejects the revision field and v1 documents while requiring the named content digests and producer identity. | spec/functional/FR-005-evidence-cli.md:68 |

## Verdict

**CONDITIONAL** — The current v1 report and validator remain correctly described. A v2 contract can be implemented after the acceptance criterion is made falsifiable.

## New findings (disposition pass 1)

Reviewed a7a41cf297ef06e437576f08307f7efc3dd1522f. The added criterion closes the original coverage gap but creates two narrower problems.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | medium | AC-6 requires clause, binding-set, and request digests to be verified against retained content, but the specified report retains no canonical clause, binding-set, or full request content from which a document validator can re-derive them. State which checks use retained bytes and which require an authoritative QueryBundle, or specify the additional retained inputs. | spec/functional/FR-005-evidence-cli.md:72 |
| FND-003 | medium | AC-6 never names `assertions[].clause` or `assertions[].source`, so its rejection rule for a missing named identity could pass a v2 schema that omits the complete clause and source identities required by FR-005's planned v2 behavior. Name those fields in the criterion and TC-007 mutation lane. | spec/functional/FR-005-evidence-cli.md:72 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | a7a41cf297ef06e437576f08307f7efc3dd1522f adds FR-005-AC-6 with a v2-only schema/validator, required digest and producer fields, revision-field exclusion, and v1 rejection. |
| FND-002 | still-open | New finding in pass 1: digest verification exceeds the report's retained inputs. |
| FND-003 | still-open | New finding in pass 1: clause and source identity fields are omitted from AC-6. |
| FND-002 | fixed | 06b125dcb00ee758e9f821f92659132e8f92e5aa separates report-byte checks from canonical reconstruction against authoritative `QueryBundle` and differential run. |
| FND-003 | fixed | 06b125dcb00ee758e9f821f92659132e8f92e5aa requires `assertions[].clause` and `assertions[].source` and adds omission/substitution cases to TC-007. |
