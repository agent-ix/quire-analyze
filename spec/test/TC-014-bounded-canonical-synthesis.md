---
id: TC-014
title: "Verify bounded canonical synthesis"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-008
    type: verifies
---
# TC-014: Verify bounded canonical synthesis

## Description

Verify bounded synthesis canonicalizes equivalent set-like inputs, retains exact candidate identities,
requires independent validation, and distinguishes a finite exhaustive miss from an incomplete search.

## Test Procedure

Run `tests/hybrid_synthesis.rs::tc_014_canonical_candidate_requires_exact_independent_validation` and
`tests/hybrid_synthesis.rs::tc_014_distinguishes_exhaustive_no_candidate_from_incomplete_search`.
Permute and duplicate input atoms, reject a candidate in validation, change the validation problem
identity, tamper with the candidate identity, run an exhaustive impossible search, and exhaust a
non-exhaustive search bound.

## Expected Results

Equivalent inputs yield the same candidate and identity. Rejected, mismatched, or tampered candidates
cannot become satisfied. An exhaustive search with no candidate returns `NoCandidate`; a bounded search
that cannot finish returns `Incomplete`.

## Limitations

This case establishes behavior within the declared finite search bound. A synthesis candidate remains
non-authoritative until the independent validator supplies matching evidence.
