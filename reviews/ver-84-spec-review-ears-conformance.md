---
id: SR-17039
title: "EARS conformance review of VER-84 FR-020..022 and NFR-010 statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-analyze; quire-analyze#88 diff origin/main...HEAD: Description/Statement and Behavior bullets of spec/functional/FR-020-datatype-lowering.md, FR-021-datatype-witness-replay.md, FR-022-datatype-package-translation.md, spec/nonfunctional/NFR-010-datatype-bounds.md"
review_set: subset
---

## Summary

Ticket: VER-84. EARS grammar check of the four requirement statements and their Behavior
bullets. Each statement is a ubiquitous "The analyzer/runner shall …" form with one system
response. The Behavior bullets use the declarative style of FR-009, which this repo accepts.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "is refused with a stable `InvalidBinding` or `UnsupportedConstruct` diagnostic" leaves open which code applies to which unresolved case: enum, variant, record, field or type name. Two implementers could choose differently, and the AC cannot pin it. Fix: map each case to one code, or say "`InvalidBinding`" for unresolved names alone. | spec/functional/FR-020-datatype-lowering.md:26-29 |

## Verdict

PASS with one low finding. The statements are atomic enough for this repo's convention, use
"shall", and name a single responsible component. The Behavior bullets state their triggers
(unresolved name, nested text or collection, bound breach) explicitly.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-020 now has a case table that maps each refusal to one `LoweringErrorCode`, and every code it names exists in `src/smt.rs:93-100`. |
