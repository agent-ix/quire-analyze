---
id: SR-17041
title: "Criterion strength review of VER-89 bounded quantifier ACs (manual; Jev unavailable)"
type: SpecReview
analysis: criterion-strength
scope: "agent-ix/quire-analyze; quire-analyze#87 diff origin/main...HEAD: spec/functional/FR-030..FR-034, spec/nonfunctional/NFR-030-quantifier-resource-bounds.md, spec/test/TC-030-bounded-quantifiers.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-89. Reviewer pass over PR #87 (spec-only), method spec-review/criterion-strength. Examined units:
FR-030-AC-1, FR-030-AC-2, FR-030-AC-3, FR-030-AC-4, FR-031-AC-1, FR-031-AC-2, FR-031-AC-3, FR-031-AC-4, FR-031-AC-5, FR-032-AC-1, FR-032-AC-2, FR-032-AC-3, FR-032-AC-4, FR-032-AC-5, FR-033-AC-1, FR-033-AC-2, FR-033-AC-3, FR-033-AC-4, FR-033-AC-5, FR-034-AC-1, FR-034-AC-2, FR-034-AC-3, FR-034-AC-4, NFR-030-AC-1, NFR-030-AC-2, NFR-030-AC-3, NFR-030-AC-4. Context only: none.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-030-AC-2 covers four of the refused domains but not `rational` or the `unencodable-element` case that the Behavior also refuses, so either could be admitted and the AC still passes. Fix: add both to the AC's refused list with their reason classes. | spec/functional/FR-030-bounded-quantifier-domains.md:48 |
| FND-002 | medium | FR-031-AC-1, FR-031-AC-2 and FR-032-AC-1 pass when fixtures are 'decided correctly', with the undefined reference evaluator as the oracle and Z3/cvc5 only 'where installed'. With no solver installed and no defined oracle, the AC cannot fail. Fix: name the expected status per fixture (for example a universal fixture whose body fails on one member is `refuted` and names it) and define the evaluator. | spec/functional/FR-031-enum-and-collection-quantifier-lowering.md:39-40 |
| FND-003 | low | 'are admitted' names no observable: admission produces no named output, so the AC passes whenever no refusal is seen. Fix: state the observable (a query is emitted, no `unsupported` diagnostic). | spec/functional/FR-030-bounded-quantifier-domains.md:47 |

## Verdict

Jev client unavailable in this environment (the quoin skill states no client exists), so this pass is a manual judgement over the AC text, not a calibrated Jev run. FR-033-AC-5 (lowered-satisfying model that fails the original quantifier), NFR-030-AC-1/AC-2 (at and one above each limit) and NFR-030-AC-3 are strong adverse-case ACs.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in eb95c03 |
| FND-002 | fixed | verified in eb95c03 |
| FND-003 | fixed | verified in eb95c03 |
