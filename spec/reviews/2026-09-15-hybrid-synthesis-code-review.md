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

Reviewed the bounded hybrid/synthesis implementation added for Analyze #33
against FR-007, FR-008, TC-013, TC-014, and the required AP-001 code-review
operation. Focused tests, formatting, and strict Clippy pass; two type-model
findings remain before this review can be a clean pass.

## Verdict

**CONDITIONAL.** The implementation correctly separates enclosures,
non-conclusions, candidates, and validation. It must replace identity-bearing
raw strings and either produce or remove the terminal `Failed` state before a
final task-completion claim.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Hybrid modes, witness identities, provider evidence identities, and synthesis atoms are raw `String` fields. A caller can swap a mode/identity at construction with no compiler signal, contrary to the Rust-review newtype guidance. | `src/hybrid_synthesis.rs:28`, `src/hybrid_synthesis.rs:59`, `src/hybrid_synthesis.rs:198` |
| FND-002 | medium | `HybridOutcome::Failed` is public but no reachable path produces it. A numerical-enclosure failure cannot be distinguished from refusal/incomplete despite the closed outcome contract. | `src/hybrid_synthesis.rs:91`, `src/hybrid_synthesis.rs:106` |
| FND-003 | low | No source or test stub, unsafe block, debug output, request-path panic, unchecked integer cast, test-only production branch, clock dependency, or unbounded provider loop was found in the reviewed subset. | `src/hybrid_synthesis.rs`, `tests/hybrid_synthesis.rs` |

## Coverage

- FR-007 AC-1/2/3 are exercised by TC-013: mode/guard/reset enclosure,
  malformed/bound exhaustion, and exact replay.
- FR-008 AC-1/2/3 are exercised by TC-014: canonical ordering/identity,
  independent validation/tamper refusal, and exhaustive no-candidate versus
  incomplete bounded search.
- `make fmt-check`, `make lint`, and `cargo test --test hybrid_synthesis`
  passed after the #33 remediation (4 tests).
- This review makes no source-release, production-readiness, or consuming
  qualification claim.

Native `quoin write` is affected by the tracked module-fetch regression;
npm Quoin 0.23.1 rendered the authoring contract. Quire validation remains the
artifact authority.
