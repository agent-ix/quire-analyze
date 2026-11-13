---
id: SR-17038
title: "Integrity review of VER-89 bounded quantifiers"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; quire-analyze#87 diff origin/main...HEAD: spec/functional/FR-030..FR-034, spec/nonfunctional/NFR-030-quantifier-resource-bounds.md, spec/test/TC-030-bounded-quantifiers.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-89. Reviewer pass over PR #87 (spec-only), method spec-review/integrity. Examined units:
FR-030, FR-031, FR-032, FR-033, FR-034, NFR-030, TC-030, FR-033-AC-1, FR-030-AC-2. Context only: FR-002, FR-009, index.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new FRs contradict FR-002 and name no profile that admits them. FR-002 says `quire.smtlib2/v1` is deliberately Boolean-only (QF_UF) and lists integer, enum, collection and quantifier as explicitly unsupported, and FR-002-AC-2 requires a quantification fixture classified unsupported. FR-030..FR-033 depend_on FR-002 and lower 'under the selected profile', but no FR adds a profile (for example a v2 with QF_LIA/LIA and enum sorts) or amends FR-002. As written, every admitted domain's element type has no exact encoding, so FR-030's own `unencodable-element` rule refuses all of them. Fix: name the new profile(s), their logics and constructs, and amend FR-002 in this PR. | spec/functional/FR-030-bounded-quantifier-domains.md:15-18, 53 |
| FND-002 | high | FR-034 contradicts FR-009 without amending it. FR-009 requires `run-package` to exit `2` on every integer, enum, collection and quantifier operation and every `integer_range` typed reference, and its note 5 says that admitting an integer construct must extend FR-009 with the type mapping, operation rows and overflow policy. FR-034 says quantified packages translate and run, but adds no FR-009 operation rows, no integer/range type mapping and no overflow policy, and FR-009-AC-23 still asserts that an `integer_range` parameter exits 2. Fix: amend FR-009 (Translation rows, note 5, AC-23) in this PR, or state that FR-034 replaces those rows and say which. | spec/functional/FR-034-quantifier-package-translation.md:15-30 |
| FND-003 | medium | FR-030 lists the reason classes as a closed set of five (`unbounded-domain`, `non-finite-bound`, `text-domain`, `unfixed-collection`, `unencodable-element`), but FR-031 and NFR-030-AC-1 return `expansion-ceiling`, which is not in it. `rational` is refused with no matching class (`unbounded-domain`? `unencodable-element`?). Fix: list `expansion-ceiling` in FR-030 and assign `rational` a class. | spec/functional/FR-030-bounded-quantifier-domains.md:29-33 |
| FND-004 | medium | 'The reference evaluator' is the oracle for FR-031-AC-1/AC-2, FR-032-AC-1 and TC-030, but no requirement in the repo defines it (it appears only in these new files). What it evaluates, over which semantics, and how it is independent of the lowering are unspecified, so 'decided correctly' has no defined oracle. Fix: define it (for example a direct enumeration evaluator over the analysis model, independent of SMT lowering) in an FR or in TC-030. | spec/test/TC-030-bounded-quantifiers.md:34, 43, 51 |
| FND-005 | medium | FR-030 depends on 'the enum domain support of the analysis model' in prose only. That support does not exist on main (FR-002 lists enum unsupported); it is specified by sibling PR #88 (VER-84, FR-020..FR-022), which is unmerged. There is no relationship edge, so the dependency is invisible to `quire extract`. Fix: add a `depends_on` edge to the FR that defines enum domains once it lands, and record the ordering dependency on VER-84. | spec/functional/FR-030-bounded-quantifier-domains.md:53 |
| FND-006 | low | FR-033-AC-1 says replay of a universal quantifier 'visits every domain element', while the Behavior says replay stops at the first failing instance. On a failing model both cannot hold. Fix: 'visits elements in domain order until the first failing element, which it reports'. | spec/functional/FR-033-quantified-assertion-replay.md:22-23, 39 |
| FND-007 | low | The new FRs trace to no stakeholder requirement: the index sentence 'StR-001 is refined by FR-001 through FR-005, FR-007, FR-008 and FR-009' is unchanged and no new FR carries a refines/derives edge to an StR. Fix: add the StR parent (StR-001 or a new StR) and update the index refinement sentence. | spec/index.md:54-58 |

## Verdict

FAIL on FND-001 and FND-002 (high): the new requirements are unreachable under the existing FR-002 profile and FR-009 translation rules, and neither is amended. `quire validate` passes on all eight changed files. TC-030 verifies every new FR and NFR-030-AC-1..AC-3; NFR-030-AC-4 is Inspection, consistent with its Verification section.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | Finite-collection quantifier domains have no encoding and no package type mapping. FR-002 (as amended here) still lists `collection` as unsupported; FR-020 (PR #88) says 'Collections and text stay unsupported' and FR-022-AC-3 makes every collection term exit `2`. FR-034 maps only enum (FR-022) and range (FR-012) domains, yet FR-030/FR-031 admit finite collections and FR-034-AC-1 requires a collection-domain quantifier to translate and lower. As written, FR-034-AC-1 and FR-031-AC-2 contradict FR-022-AC-3. Fix: define the finite-collection construct (its analysis-model form, encoding and QSL type mapping) and amend FR-002, or drop collection domains from this milestone. | spec/functional/FR-034-quantifier-package-translation.md:AC table, FR-034-AC-1 |
| FND-009 | medium | The amended FR-002 unsupported list drops `enum` without citing who admits it, and still lists `record` and `option` as unsupported, while FR-020 (PR #88) admits enum, record and option datatypes and FR-032 here declares datatype sorts when a body mentions an enum, record or option. Fix: in the same bullet, state by id that FR-020 admits enum, record and option under `quire.smtlib2/v2`, and remove record and option from the unsupported list. | spec/functional/FR-002-smt-lowering.md:Behavior, unsupported-construct bullet |
| FND-010 | low | TC-030 step 1 lists the refused fixtures as unbounded integer, text, unfixed collection, non-finite bound and nested refused, but the fixed FR-030-AC-2 and new FR-030-AC-5 also require rational, unencodable-element and `expansion-ceiling` fixtures. Fix: add them to step 1. | spec/test/TC-030-bounded-quantifiers.md:Test Procedure step 1 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in eb95c03 |
| FND-002 | fixed | verified in eb95c03 |
| FND-003 | fixed | verified in eb95c03 |
| FND-004 | fixed | verified in eb95c03 |
| FND-005 | fixed | verified in eb95c03 |
| FND-006 | fixed | verified in eb95c03 |
| FND-007 | fixed | verified in eb95c03 |
| FND-008 | still-open | New in disposition pass 1 on eb95c03; not yet fixed. |
| FND-009 | still-open | New in disposition pass 1 on eb95c03; not yet fixed. |
| FND-010 | still-open | New in disposition pass 1 on eb95c03; not yet fixed. |
