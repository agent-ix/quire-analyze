---
id: SR-001
title: "Code review of hybrid reachability and bounded synthesis providers"
type: SpecReview
analysis: code-review
scope: "src/hybrid_synthesis.rs, tests/hybrid_synthesis.rs, spec/functional/FR-007-sound-hybrid-reachability.md, spec/functional/FR-008-bounded-canonical-synthesis.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-007", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-008", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/AP-001", type: references }
---
# Code Review of Hybrid Reachability and Bounded Synthesis Providers

## Summary

Reviewed the hybrid/synthesis implementation at this PR's exact revision
against FR-007, FR-008, TC-013, TC-014, and AP-001. The review found three
blocking contract defects in bounded synthesis and independent validation.

## Verdict

**FAIL.** The implementation eagerly allocates unbounded combinations, omits
the search bound from the synthesis identity, and lets a caller forge an
"independent" validation decision. These must be corrected before task
completion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `synthesize` builds every combination for a size before inspecting `search_bound`; caller-controlled atom sets can allocate exponentially before the declared bound takes effect. | `src/hybrid_synthesis.rs:258-260`, `src/hybrid_synthesis.rs:367-380` |
| FND-002 | high | `synthesis_identity` excludes `search_bound`, so requests with distinct conclusion semantics receive the same problem identity. | `src/hybrid_synthesis.rs:270`, `src/hybrid_synthesis.rs:392-402` |
| FND-003 | high | `ValidationRequest.accepted` and `evidence_identity` are caller-supplied; no validator trust seam or attestation binds the asserted decision to the candidate/problem. | `src/hybrid_synthesis.rs:213-216`, `src/hybrid_synthesis.rs:303-335` |
| FND-004 | medium | TC-013 and TC-014 use unrecognized prose `Tracing:` comments rather than compiler-checked tracking markers, leaving FR-007 and FR-008 unbacked in mechanical coverage. | `tests/hybrid_synthesis.rs` |

## Coverage

- The test cases exercise portions of both requirements but their prose trace
  comments do not back Test Matrix rows.
- No production readiness, release input, or human decision was used as a
  development gate in this review.

Native `quoin write` is affected by the tracked module-fetch regression;
npm Quoin 0.23.1 rendered the authoring contract. Quire validation remains the
artifact authority.
