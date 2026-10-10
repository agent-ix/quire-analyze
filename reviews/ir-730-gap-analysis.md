---
id: SR-5581
title: IR-730 changed-scope gap analysis
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze@1c0625699abe8dfc5167c040ca5fccbf8616ec88; Cargo.toml; Cargo.lock; spec/functional/FR-008-bounded-canonical-synthesis.md; tests/hybrid_synthesis.rs; spec/assurance/MP-001-analyze-measurements.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-analyze/FR-008
    type: references
---

## Summary

Ticket: IR-730. The dependency-only change leaves requirement, source, and test text untouched. The test's property macro still comes from the resolved `proptest` crate; the Rust 1.75 focused build compiled it and the property-test target. The computed matrix links FR-008-AC-1 to four test functions, including the `proptest!` case at `tests/hybrid_synthesis.rs:46`.

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| FR-008-AC-1 | examined | Equivalent set-like inputs yield the same canonical candidate order and identity. |
| MP-001 M-09 | examined | Local quality gates include an MSRV pass. |
| Cargo.toml / Cargo.lock | examined | The dev-dependency is restricted to the 1.8 series and locked to proptest 1.8.0. |
| tests/hybrid_synthesis.rs | examined | The tagged `proptest!` case still asserts `synthesize(&request) == synthesize(&reordered)`. |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No new gap in the changed dependency scope (placeholder) | - |

## Coverage

`quoin matrix --repo /home/peter/dev/quire-analyze-ir730 --json` found 52 criterion records, 37 with static test binders and 15 without; those existing unbound records were unchanged by this two-file diff. The FR-008-AC-1 property test is one of four static binders. The focused Rust 1.75 compilation completed; execution stopped at the unchanged foundation prose assertion before this later integration binary ran. The full pre-PR gate has not run on this head. Semantic review: skipped (optional, not requested). Plan completion: not assessed.

## Verdict

**PASS for the changed dependency scope.** No requirement, source, test, trace tag, or test threshold was removed or weakened. This is a differential finding only; it does not declare the repository's pre-existing matrix gaps or full MSRV gate resolved.
