---
id: SR-17049
title: "EARS conformance review of VER-70 requirement statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-analyze; quire-analyze#90 diff origin/main...HEAD: spec/functional/FR-002, FR-009, FR-010, FR-011, FR-012; spec/nonfunctional/NFR-004; spec/test/TC-016..018; spec/index.md; spec/test-matrix.md"
review_set: subset
---

## Summary

Ticket: VER-70. EARS grammar of the FR-010, FR-011, FR-012 Descriptions and the NFR-004 Statement, plus the edited FR-002 and FR-009 passages. Engine: quire validate --summary reports 42/42 docs grammar-clean, 0 findings. Semantic review found two compound statements.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The FR-010 Description has two 'shall' clauses: lowering to SMT-LIB2, and having Z3 and cvc5 decide. These are two obligations in one statement. Fix: split them, or make solver decision a Behavior bullet. The engine check passes it as clean. | spec/functional/FR-010-linear-integer-arithmetic.md:19-23 |
| FND-002 | low | The NFR-004 Statement puts three obligations under one 'shall': return unsupported, publish no unverified model, and parse and evaluate within declared bounds. Fix: split it into atomic statements, or one statement per metric row. | spec/nonfunctional/NFR-004-integer-exactness.md:18-21 |

## Scope

| Unit | Role | Path |
| --- | --- | --- |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-012 | examined | spec/functional/FR-012-integer-claim-translation.md |
| NFR-004 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| FR-002 | examined | spec/functional/FR-002-smt-lowering.md |
| FR-009 | examined | spec/functional/FR-009-package-runner.md |

## Verdict

PASS with low findings. FR-011 and FR-012 are singular ubiquitous statements with a named subject; no vague-response verbs.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in f1ad0f5 |
| FND-002 | fixed | verified in f1ad0f5 |
