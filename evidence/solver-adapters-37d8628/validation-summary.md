# Issue #3 bounded solver adapters validation summary

> **Producer validation attestation.** The source revision and commands below are immutable and
> reproducible, but no raw command transcript or machine-signed verdict is retained. This record does
> not claim independent review, derivation-evidence authority, or a release decision.

## Subject

- Source revision: `37d8628a983ffc38cd0c5fe3e11e1f18df7670d8`
- Source tree: `a2e99cb68be050a85a4681c99e6552383443e98e`
- Collected: `2026-08-31T22:10:38Z`
- Scope: issue #3 bounded Linux Z3/cvc5 process adapters and repository-local assurance gates

## Observed Local Validation

The producer ran `make ci` from the subject tree before committing this record. The command exited
zero after running the closed Makefile gate census:

| Gate | Observed result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| stable `cargo test` | 33 passed, 0 failed |
| cargo-deny advisories, bans, licenses, sources | pass; three unused-license warnings only |
| unsafe-comment audit | pass |
| Quire specification/plan validation | pass; installed-module duplicate-definition warnings only |
| Rust 1.75 locked tests | 33 passed, 0 failed |
| rustdoc with warnings denied | pass |
| LLVM coverage | 90.97% lines overall; 90.36% for `src/solver.rs`; 90% floor passed |
| retained evidence checksum verification | pass for every then-declared artifact |

The stable and MSRV totals comprise four library tests, three ADR tests, six foundation tests, one
integration test, thirteen lowering tests, and six issue #3 adapter tests.

TC-005 was also rerun with captured measurement output. The six timeout/cancellation cleanup
durations were `[4, 4, 4, 4, 4, 4]` milliseconds; the maximum was 4 ms against the 1,000 ms limit.
Every repetition asserted that both the fake solver and its descendant no longer existed.

## Reviewed Exceptions and Limits

- This is a Linux process-group result. Non-Linux execution returns `unsupported-platform` before
  spawn and equivalent containment remains tracked by native issue #20.
- Fake executables isolate protocol and process failures. Pinned real Z3/cvc5 differential execution
  remains owned by issue #5; stress and OS-boundary fault injection remain issue #21.
- Hosted CI was not dispatched. The workflow trigger remains exactly `workflow_dispatch`; this
  record reports the complete local gate only.
- The record covers bounded execution and normalized adapter results, not analysis conclusions,
  model replay, machine-produced evidence, CLI behavior, or release suitability.

## Review Bindings

- Specification review: REV-010
- Code review: REV-011
- Gap analysis: REV-012
- QA assessment: REV-013
- Requirement-tagged slice: TC-005
- Residual QA campaigns: native issues #20 and #21
