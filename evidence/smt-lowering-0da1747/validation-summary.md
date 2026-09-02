# Issue #7 deterministic SMT lowering validation summary

> **Producer validation attestation.** The source revision and commands below are immutable and
> reproducible, but no raw command transcript or machine-signed verdict is retained. This record does
> not claim independent review, derivation-evidence authority, or a release decision.

## Subject

- Source revision: `0da174753836fe3ae23447c786338ebf53def1a7`
- Source tree: `ddb47553cd1d46c8c2525876f2a8e4737a787e56`
- Collected: `2026-08-31T21:10:50Z`
- Scope: issue #7 Boolean SMT-LIB2 v1 lowering and repository-local assurance gates

## Observed Local Validation

The producer ran `make ci` from a cleanly staged subject tree before committing this record. The
command exited zero after running the closed Makefile gate census:

| Gate | Observed result |
|---|---|
| `cargo fmt --all -- --check` | pass |
| `cargo clippy --all-targets -- -D warnings` | pass |
| stable `cargo test` | 24 passed, 0 failed |
| cargo-deny advisories, bans, licenses, sources | pass; three unused-license warnings only |
| unsafe-comment audit | pass |
| Quire specification/plan validation | pass; installed-module duplicate-definition warnings only |
| Rust 1.75 locked tests | 24 passed, 0 failed |
| rustdoc with warnings denied | pass |
| LLVM coverage | 91.56% lines overall; 91.49% for `src/smt.rs`; 90% floor passed |
| retained evidence checksum verification | pass for every then-declared artifact |

The stable and MSRV totals comprise one library test, three ADR tests, six foundation tests, one
integration test, and thirteen issue #7 lowering tests.

## Reviewed Exceptions and Limits

- RUSTSEC-2026-0009 affects `time`'s RFC 2822 parser. The sole downstream user,
  `jsonschema 0.17.1`, contains no `Rfc2822` reference and uses `time` for date/RFC 3339 parsing.
  The exact cargo-deny exception is documented in `deny.toml`; cleanup remains tracked by
  `agent-ix/quire-contract-ir#37` because the patched release requires edition 2024/Rust 1.88.
- Hosted CI was not dispatched. The workflow trigger remains exactly `workflow_dispatch`; this
  record reports the complete local gate only.
- This record covers deterministic Boolean lowering, not solver execution, semantic conclusions,
  differential validation, machine-produced evidence, CLI behavior, or release suitability.

## Review Bindings

- Code review: REV-007
- Gap analysis: REV-008
- QA assessment: REV-009
- Requirement-tagged slice: TC-010
- Residual QA campaigns: native issue #19
