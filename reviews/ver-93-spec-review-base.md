---
id: SR-17042
title: "Base spec review of VER-93 hybrid reachability review specs"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#89 diff origin/main...HEAD: spec/functional/FR-040-hybrid-enclosure-soundness.md, spec/functional/FR-041-hybrid-reachability-review.md, spec/functional/FR-042-hybrid-reachability-disposition.md, spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md, spec/test/TC-040-hybrid-enclosure-soundness.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-93. Base checklist over the six added or changed spec files of quire-analyze#89 (FR-040, FR-041, FR-042, NFR-040, TC-040 and spec/index.md), checked against src/hybrid_synthesis.rs `reach`/`validate_hybrid` and FR-007. Id formats, AC ids and verification columns are well formed; no 03x leftovers remain in the diff; `quire validate --scope . "spec/**/*.md"` exits 0. The IR-1469 reference is by id only and public-safe. No paywalled standard text, roadmap content, hash/SHA/version-tracking record or compatibility layer is introduced; FR-041-AC-5 and FR-042-AC-5 explicitly forbid a fallback. Criterion-strength analysis was not run: the installed schema has `criterion-strength` but no Jev client exists (the skill's own status says do not run); a manual can-this-AC-fail check is folded into this base review instead.

## Scope examined

- FR-040 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-041 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-042 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- NFR-040 (examined): spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md
- TC-040 (examined): spec/test/TC-040-hybrid-enclosure-soundness.md
- spec/index.md (examined): spec/index.md
- FR-040-AC-1 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-041-AC-4 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-041-AC-5 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-042-AC-5 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- FR-041-AC-6 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-042-AC-6 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | The index edit splices the new sentence into the middle of an existing one: 'AA-001 define the FR-040 through FR-042, NFR-040 and TC-040 specify the hybrid reachability review. assurance boundary.' Both sentences now read wrongly. Fix: restore 'AP-001, AD-001, CAC-001, MP-001, and AA-001 define the assurance boundary.' and put the new sentence after it. | spec/index.md:58 | wrong-requirement |
| FND-002 | low | The Requirements Architecture sentences that enumerate StR-001's refinements, the NFR constraints and the TC verification matrix still omit FR-040 to FR-042, NFR-040 and TC-040, although the new FRs declare `implements StR-001`. | spec/index.md:55-57 | wrong-requirement |
| FND-003 | medium | FR-040-AC-1 requires directed rounding, but `Interval` is `i64` and every bound step in `reach` is an exact integer add (src/hybrid_synthesis.rs:63-70, 190-226): no rounding happens, so no test can fail the rounding clause. Its 'within one unit of a representable boundary' case is the saturation case FR-040-AC-2 already owns. Fix: either state AC-1 conditionally ('if a non-integer interval type is introduced') or delete it and keep AC-2 as the boundary obligation. | spec/functional/FR-040-hybrid-enclosure-soundness.md:27, src/hybrid_synthesis.rs:190-226 | wrong-requirement |
| FND-004 | low | FR-041-AC-4 names four criteria but no rule for combining them, so any reason that mentions all four satisfies the AC whichever disposition it picks. Acceptable for a human disposition, but a one-line decisive rule (for example, 'delegate when any high finding has no bounded fix') would make the inspection able to fail. | spec/functional/FR-041-hybrid-reachability-review.md:28 | wrong-requirement |

## Verdict

FAIL on two medium findings: the index edit corrupts a sentence, and FR-040-AC-1 is vacuous for the integer interval type the module uses. FR-041 does require exactly one disposition with stated criteria and no fallback.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | The new decision rule is fully determined by unsound findings, bounded fixes and external-tool support, so the fourth stated criterion (the cost of maintaining an unchecked algorithm) can never change the outcome although the AC says the reason is stated against all four. Fix: either let cost decide the remaining case (no unsound findings) or drop it from the criteria list. | spec/functional/FR-041-hybrid-reachability-review.md:28 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | medium | The new rule is total and its cases do not conflict, but two of its cases pick an impossible path. With no unsound finding and no established tool supporting the model class, keep-and-harden needs a differential test against an established engine on those models, which cannot exist, so the rule selects delegate-and-remove. With an unsound finding that has no bounded fix and no supporting tool, the first clause also selects delegate-and-remove. In both cases FR-041 and FR-042-AC-3 require delegating to an established tool that does not support the model class. Fix: state the outcome when no established tool supports the model class (for example, keep-and-harden with the differential test replaced by the TC-040 enumeration check, or remove the module and refuse hybrid requests without delegating). | spec/functional/FR-041-hybrid-reachability-review.md:28 |

## Dispositions

Round 1, checked against fix commit 83fcd4e on ver-93-hybrid-spec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |

Round 2, checked against fix commit cec5bd3 on ver-93-hybrid-spec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | - |
