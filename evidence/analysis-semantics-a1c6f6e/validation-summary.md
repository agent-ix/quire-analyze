# Issue #4 Boolean analysis semantics validation summary

> **Producer validation attestation.** The source revision and commands below are immutable and
> reproducible, but no raw command transcript or machine-signed verdict is retained. This record does
> not claim independent review, derivation-evidence authority, or a release decision.

## Subject

- Source revision: `a1c6f6e55872e8139200c6fdb5200ac48de7ca21`
- Source tree: `f4bc3d36fd3dbbb5650a5c543056f2cb1aa01906`
- Collected: `2026-08-31T22:42:46Z`
- Scope: issue #4 exact Boolean-v1 analysis requests, conclusions, model replay, and local gates

## Observed Local Validation

The producer ran `make ci` from the subject tree before committing this record. The command exited
zero after running the closed Makefile gate census:

| Gate | Observed result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| stable `cargo test` | 39 passed, 0 failed |
| cargo-deny advisories, bans, licenses, sources | pass; three unused-license warnings only |
| unsafe-comment audit | pass |
| Quire specification/plan validation | pass; installed-module duplicate-definition warnings only |
| Rust 1.75 locked tests | 39 passed, 0 failed |
| rustdoc with warnings denied | pass |
| LLVM coverage | 91.89% lines overall; 93.70% for `src/analysis.rs`; 90% floor passed |
| retained evidence checksum verification | pass for every then-declared artifact |

The stable and MSRV totals comprise six library tests, three ADR tests, four issue #4 analysis tests,
six foundation tests, one integration test, thirteen lowering tests, and six adapter tests.

## Reviewed Exceptions and Limits

- The four integration tests cover all ten analysis-kind/sat-unsat cells with solver-independent
  finite Boolean expectations, exact role/polarity failures, verified mapping/replay, malformed model
  states, and non-conclusive adapter states.
- Fake solver processes provide controlled raw outcomes. Pinned real Z3/cvc5 model acquisition,
  differential agreement, and report publication remain issue #5.
- Property/fuzz/mutation expansion remains native issue #22; it does not block the seeded Boolean-v1
  acceptance decision.
- Hosted CI was not dispatched. The workflow trigger remains exactly `workflow_dispatch`; this
  record reports the complete local gate only.
- This record does not cover broader data theories, machine-produced derivation evidence, CLI
  behavior, cross-platform solver containment, or release suitability.

## Review Bindings

- Specification review: REV-014
- Code review: REV-015
- Gap analysis: REV-016
- QA assessment: REV-017
- Requirement-tagged slices: TC-001, TC-004, TC-006, TC-007
- Residual QA campaign: native issue #22
