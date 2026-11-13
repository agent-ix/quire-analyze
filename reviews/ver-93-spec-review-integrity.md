---
id: SR-17043
title: "Integrity review of VER-93 hybrid reachability review specs"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; quire-analyze#89 diff origin/main...HEAD: spec/functional/FR-040-hybrid-enclosure-soundness.md, spec/functional/FR-041-hybrid-reachability-review.md, spec/functional/FR-042-hybrid-reachability-disposition.md, spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md, spec/test/TC-040-hybrid-enclosure-soundness.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-93. Integrity lens over FR-040, FR-041, FR-042, NFR-040 and TC-040: cross-reference consistency, coverage of FR-040 by the FR-041 test set and by TC-040, and completeness of the delegate-and-remove removal set. Relationships resolve (FR-007, FR-009, StR-001, FR-040, FR-041, NFR-001) and `quire validate` exits 0.

## Scope examined

- FR-041-AC-1 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-041-AC-2 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-042-AC-1 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- FR-042-AC-2 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- FR-040-AC-6 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- TC-040 (examined): spec/test/TC-040-hybrid-enclosure-soundness.md
- FR-041-AC-3 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-041-AC-7 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- NFR-040-AC-2 (examined): spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md
- FR-040 (context_only): spec/functional/FR-040-hybrid-enclosure-soundness.md

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-041-AC-1 requires tests for only five categories (guard rounding, overlapping guards, reset leaving an invariant, horizon exhaustion, malformed mode), while TC-040 says 'for each FR-040 criterion'. FR-042-AC-1 gates keep-and-harden on the FR-041 set alone, so saturation (AC-2), multi-transition join (AC-5), final-step-only enclosure (AC-8) and the error bound (AC-10) can stay untested under keep-and-harden, although saturating arithmetic and final-step-only enclosure are exactly what `reach` does (src/hybrid_synthesis.rs:190-226). Fix: require at least one test per FR-040 criterion in FR-041-AC-1. | spec/functional/FR-041-hybrid-reachability-review.md:26, spec/functional/FR-042-hybrid-reachability-disposition.md:25, spec/test/TC-040-hybrid-enclosure-soundness.md:22 | wrong-requirement |
| FND-002 | medium | FR-040-AC-6 says the target enclosure 'is that image, not the image of the whole flowed interval'. Mapping the whole flowed interval (what `reach` does, src/hybrid_synthesis.rs:204-212) over-approximates and is sound, so AC-6 is a precision obligation inside a soundness FR. FR-041-AC-2 records only unsound enclosures as findings, so a failing AC-6 test is not recorded, while FR-042-AC-1 still requires it to pass. Fix: either restate AC-6 as 'contains the image of the guard-satisfying part' (soundness), or move the precision obligation out of FR-040 and say FR-041 records precision failures too. | spec/functional/FR-040-hybrid-enclosure-soundness.md:32, spec/functional/FR-041-hybrid-reachability-review.md:27 | wrong-requirement |
| FND-003 | medium | FR-042-AC-2 removes `reach`, `replay_hybrid` and FR-007 under delegate-and-remove, but says nothing of FR-040, NFR-040, TC-013, `hybrid_method_outcome`, `bind_hybrid_result` or the index lines that cite FR-007. It also leaves open whether FR-040's soundness obligations then bind the delegated result (FR-040 depends_on FR-007, which is removed). Fix: list the full removal set and state that FR-040 applies to the delegated enclosure, or that it is removed with FR-007. | spec/functional/FR-042-hybrid-reachability-disposition.md:26, spec/functional/FR-040-hybrid-enclosure-soundness.md:6-7 | missing-requirement |
| FND-004 | low | TC-040 covers 'a duplicate ... mode', but no FR-040 AC states what a duplicate mode must yield. Either add it to FR-040-AC-11 or drop it from TC-040. | spec/test/TC-040-hybrid-enclosure-soundness.md:22-28, spec/functional/FR-040-hybrid-enclosure-soundness.md:37 | missing-requirement |

## Verdict

FAIL on three medium findings: the adversarial set FR-042-AC-1 gates on does not cover every FR-040 criterion; FR-040-AC-6 is a precision obligation that FR-041's finding rule cannot record; and the delegate-and-remove removal set is incomplete.

## Dispositions

Round 1, checked against fix commit 83fcd4e on ver-93-hybrid-spec.

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
