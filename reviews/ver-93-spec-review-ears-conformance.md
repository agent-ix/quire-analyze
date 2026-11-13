---
id: SR-17044
title: "EARS conformance review of VER-93 hybrid reachability review specs"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-analyze; quire-analyze#89 diff origin/main...HEAD: spec/functional/FR-040-hybrid-enclosure-soundness.md, spec/functional/FR-041-hybrid-reachability-review.md, spec/functional/FR-042-hybrid-reachability-disposition.md, spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md, spec/test/TC-040-hybrid-enclosure-soundness.md, spec/index.md"
review_set: subset
---

## Summary

Ticket: VER-93. EARS grammar check of the FR-040, FR-041 and FR-042 statements and the NFR-040 statement.

## Scope examined

- FR-040 (examined): spec/functional/FR-040-hybrid-enclosure-soundness.md
- FR-041 (examined): spec/functional/FR-041-hybrid-reachability-review.md
- FR-042 (examined): spec/functional/FR-042-hybrid-reachability-disposition.md
- NFR-040 (examined): spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | FR-040 is two requirements under one trigger. 'When the analysis provider returns a hybrid reachability enclosure' cannot govern the second SHALL ('end every request it cannot enclose that way as a typed non-conclusion'), because such a request returns no enclosure. Fix: split into an ubiquitous 'The analysis provider SHALL end every hybrid request it cannot soundly enclose as Incomplete or Refused' and the event-driven enclosure-soundness statement. | spec/functional/FR-040-hybrid-enclosure-soundness.md:15-18 | wrong-requirement |
| FND-002 | low | FR-041's trigger 'When further work is about to build on the in-house hybrid reachability module' names no observable event, and the statement joins two SHALLs (review, record). Fix: use a concrete trigger (before any requirement or consumer that depends on `reach` is accepted) and one SHALL. | spec/functional/FR-041-hybrid-reachability-review.md:15-19 | wrong-requirement |
| FND-003 | low | NFR-040 joins two shall clauses (totality, determinism), and 'allocation that grows beyond the declared step bound' compares memory with a step count, which has no common unit. Fix: split the determinism clause out, and bound allocation by a stated per-step or per-request size. | spec/nonfunctional/NFR-040-hybrid-enclosure-totality.md:14-16 | wrong-requirement |

## Verdict

FAIL on one medium finding: FR-040's event trigger excludes the case its second SHALL governs. FR-042's statement is a well-formed event-driven requirement.
