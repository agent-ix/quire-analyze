---
id: REV-009
title: "SMT lowering quality assurance and test coverage assessment"
type: Review
---
# SMT lowering quality assurance and test coverage assessment

## Assessment

The code is sufficiently tested for issue #7, but it is not fully tested in the absolute sense. The
local suite contains 24 passing tests, including 13 requirement-tagged lowering tests. LLVM source
coverage is 91.56% overall and 91.49% for `src/smt.rs`, above the enforced 90% line floor. Coverage
percentage is a regression signal, not proof of semantic completeness.

## Covered Risk Classes

- byte determinism under input reordering and an independently readable golden;
- every supported Boolean v1 operator, including equality and inequality;
- authoritative package/clause membership, recomputed digest, owner, anchor, and typed-expression
  checks at the input boundary;
- arithmetic, ordering, quantification, and text/non-Boolean rejection;
- explicit binding success plus duplicate, incompatible, repeated, and unused-member failures;
- owner-independent structural type shapes, including nested composites and declaration-order
  invariance;
- state observation identity, source/assertion maps, material-input digest invalidation, empty,
  duplicate, and over-limit statement requests;
- exact dependency revision, Rust 1.75, no solver library, warning-free docs/lints, specification
  validation, supply-chain policy, and retained-evidence integrity.

## Residual Test Gaps

- mutation testing of the exhaustive lowering and identity encoders;
- property/fuzz tests for arbitrary valid expression trees and permutation invariance;
- generated collision-family tests across assertion, variable, binding, request, and query identities;
- direct runtime fixtures for query-byte, expression-depth, and expression-node boundaries;
- one executable fixture for every individual unsupported IR variant rather than representative
  category fixtures;
- parsing the golden with independent SMT parsers and later differential engines.

These additions are tracked in native issue #19. Solver failure injection, timeout/cancellation,
process cleanup, model replay, differential adjudication, evidence publication, and CLI parity belong
to existing later-wave issues rather than issue #7.

## QA Verdict

PASS for issue #7 acceptance and merge review. NOT FULLY TESTED as a complete analyzer: the result is
bounded to deterministic Boolean lowering, and the residual campaigns remain visible in filed work.
