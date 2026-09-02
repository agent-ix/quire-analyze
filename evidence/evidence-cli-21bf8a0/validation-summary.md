# Issue #5 differential evidence and CLI validation summary

> **Producer validation attestation.** The source revision, pins, and commands below are immutable
> and reproducible, but no raw command transcript or machine-signed PGM-01 envelope is retained.
> The shared PGM-01 component remains unavailable on `agent-ix/quire-contract-ir#20`. This record
> does not claim independent review, derivation-evidence authority, Task-007 completion, or release.

## Subject

- Source revision: `21bf8a0fd0fb872f9b8ca263867a3c340cea7556`
- Source tree: `3249f3a0bb26f340422e0fceefc2ac06b5adcb27`
- Collected: `2026-08-31T23:14:26Z`
- Host: Linux 6.18.33.2-microsoft-standard-WSL2 x86_64
- Default toolchain: rustc/cargo 1.94.1
- MSRV lane: Rust 1.75.0
- Specification validator: Quire 0.31.0 (CLI 4f6ed024, engine 0.46.0@ca7362d4)
- Scope: dependency-independent issue #5 report, differential, real-engine, validator, and publisher slice

## Official Solver Inputs

| Engine | Official release asset | Archive SHA-256 | Extracted executable SHA-256 | Normalized version identity |
|---|---|---|---|---|
| Z3 | `z3_solver-5.1.0.0-py3-none-manylinux_2_27_x86_64.whl` from `Z3Prover/z3` tag `z3-5.1.0` | `dfad9e309d7010b1ff6bdb33f21570a1603ef4727373221c7117a74448f0cfef` | `54bae839dd54e262edac6f755fc99659ce2a121301faff20a3e3b94478dcead0` | `Z3 version 5.1.0 - 64 bit` |
| cvc5 | `cvc5-Linux-x86_64-static.zip` from `cvc5/cvc5` tag `cvc5-1.3.4` | `dcdbfada0ce493ee98259c0816e0daafc561c223aadb3af298c2968e73ea39c6` | `7562a8b0b835e3eaad5f1a7b4616cd762350cf567b6be03d7e8ee24fa5ced5ee` | Complete output beginning `cvc5 1.3.4 [git f3b21c4 on branch HEAD]`; retained by each runtime engine record |

Both downloaded archive digests were checked before extraction. The ignored/manual integration test
then checked the extracted executable digests and version identities before constructing either
`SolverConfig`.

## Observed Local Validation

The producer ran `make ci` from the exact subject commit. It exited zero with:

| Gate | Observed result |
|---|---|
| formatting and Clippy warnings-as-errors | pass |
| stable default tests | 43 passed, 0 failed, 1 explicit real-engine test ignored |
| cargo-deny advisories, bans, licenses, sources | pass; three unmatched-license warnings only |
| unsafe-comment audit | pass |
| specification/plan validation | pass; installed-module duplicate-definition warnings only |
| Rust 1.75 locked tests | 43 passed, 0 failed, 1 explicit real-engine test ignored |
| rustdoc with warnings denied | pass |
| LLVM coverage | 91.00% lines overall; 93.09% analysis, 88.96% report, 90.47% solver, 74.07% thin CLI wrapper; 90% floor passed |
| retained evidence checksum verification | pass for every previously declared artifact |

The producer separately ran the exact ignored/manual
`official_z3_cvc5_differential_corpus_agrees` test against the verified binaries. It passed one SAT
case with independently replay-verified models and one UNSAT case, with both engines classified as
agreement and the expected satisfied/refuted status respectively.

## Failed Observations Retained in the Review History

The first real SAT run was inconclusive: Z3 exposed a declared `:named` assertion alias in its model,
while cvc5 emitted only the requested variable. The decoder initially treated the alias as an
unexpected non-literal model value. The stable regression now ignores only query-sealed assertion
aliases, still rejects unknown symbols, requires every declared variable exactly once, and replays
the original assertions.

The next real UNSAT run was inconclusive: Z3 returned the valid `unsat` primary result and the
expected model-unavailable error but exited 1, while cvc5 exited 0. The stable regression accepts
only Z3 exit 1 with empty stderr, an unsat/unknown primary result, and one exact whitelisted
model-unavailable response. Every other nonzero exit remains a tool failure.

## Reviewed Limits

- The default gate deliberately does not download or auto-run external solvers. Hosted automatic CI
  remains disabled; `.github/workflows/ci.yml` was not modified.
- The real-engine test covers seeded Boolean SAT and UNSAT. Complete retained positive, negative,
  unsupported, timeout, and disagreement corpus/CLI execution remains issue #24.
- Deterministic create/write/sync/rename/crash fault injection remains issue #23.
- Generated/fuzz/model campaigns remain #22; containment stress and non-Linux execution remain
  #21 and #20.
- `pgm01Envelope.status` is exactly `unavailable`. Therefore FR-005-AC-2, Task-007, native issue #5,
  and the epic remain open.

## Review Bindings

- Specification review and plan delta: REV-018
- Code review: REV-019
- Gap analysis: REV-020
- QA assessment: REV-021
- Requirement-tagged slices: TC-006, TC-007, TC-008
- Residual QA: native issues #23 and #24
