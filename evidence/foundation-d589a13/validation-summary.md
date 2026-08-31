# Issue #2 foundation validation summary

## Subject

- Source revision: `d589a13ef3ed50a32cbc2e18812135753b1f4864`
- Source tree: `b40404da2040879a83c373a30dac558cfcb6622f`
- Collected: `2026-08-31T19:01:43Z`
- Contribution: agent-assisted implementation; producer self-review, not independent approval

## Authoritative Inputs

- PGM-01 merged revision: `7dac9d8c19952412b56a0347387666e2ca81e01d`
- Contract-IR Wave 1 accepted head: `bb5d30cbb1519b7ac286250114c96ba967661cba`
- Contract-IR implementation/corpus merge: `5c49ebfd1c87415f74420ad047392bd03b1bd202`
- Package schema SHA-256: `748d98def7c0a67e3e12f882cd9ef7d0948c8eacbff1e5f6135faa7fd29d642d`
- Conformance schema SHA-256: `63fe642ebe7e7f49acf59094a8edaa488b96b13806886f0af2779629900bdb75`
- Corpus manifest SHA-256: `aed86fa6fd5e88412b3a771b594011884ef6df1e8256827ccf87bc9ae53fced4`

## Tool Identities

- `rustc 1.94.1 (e408947bf 2026-03-25)`
- `cargo 1.94.1 (29ea6fb6a 2026-03-24)`
- `cargo-deny 0.19.8`
- `quire 0.31.0 (cli 4f6ed024, engine 0.46.0@ca7362d4)`
- MSRV lane: `rustc/cargo 1.75.0`

## Outcomes

| Command | Outcome |
|---|---|
| `quire validate --scope . 'spec/**/*.md' 'planning/**/*.md' 'plan/**/*.md'` | pass; 39 documents |
| `quire validate --scope . 'spec/**/*.md'` | pass |
| `CARGO_TARGET_DIR=target make ci` | pass |
| `CARGO_TARGET_DIR=target-msrv cargo +1.75.0 test --locked` | pass; 6 tests, 0 failed, 0 ignored |
| `CARGO_TARGET_DIR=target RUSTDOCFLAGS=-Dwarnings cargo doc --locked --no-deps` | pass |
| `git diff --check` | pass |

The first local Cargo attempt used the sandbox-provided global target directory
`/home/peter/.cargo-target` and failed before compilation because that directory is read-only. The
rerun used repository-local target directories and passed. This environmental failure is retained
here rather than reclassified as a test failure or silently omitted.

## Baseline and Remote Protection Audit

- Both `LICENSE-MIT` and `LICENSE-APACHE` exist; `Cargo.toml` declares
  `MIT OR Apache-2.0`; the license gate passed.
- `Cargo.toml` has `publish = false`.
- `.github/workflows/ci.yml` has only `workflow_dispatch`; no automatic trigger was added or run.
- GitHub `main` protection observed on 2026-08-31 requires strict `Rust Checks` and `License Check`,
  one non-stale CODEOWNER approval, and resolved conversations; force pushes and deletions are
  disabled. Administrator enforcement is disabled.

## Review and Coverage

REV-001 covers dependency, risk, evidence, integrity, scope, failure-domain, and QA sufficiency.
REV-002 maps every open semantic gap to existing native issue #6, #7, #3, #4, #5, epic #8, or the
PGM-02 Wave 4 human gate; it found no duplicate ticket to create. Four requirement-backed foundation
tests pass. The two original placeholder tests are scaffold health only.

Semantic implementation coverage is **zero**: all TC-001 through TC-008 matrix rows remain planned.
No solver adapter, SMT lowering, conclusion, counterexample, differential, or semantic evidence is
claimed by this record.

## Limitations and Authority

- Results are Linux-local producer evidence, not independent review or hosted CI.
- GitHub Actions remained manual-only and was not dispatched.
- Cross-platform, solver, semantic, fault-injection, differential, and counterexample campaigns are
  future native-issue work and remain open.
- AA-001 is open. This record selects no source-release candidate, creates no tag, authorizes no
  publication, and confers no qualification, accreditation, certification, or consuming-project approval.
