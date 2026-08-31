# ADR-0010 issue #6 validation summary

## Subject

- Source revision: `2690c2554e49cf67b542014e40f7ebcd962fe83f`
- Source tree: `dcd6d72b4f3c927dcb815f5709a91630df27a310`
- Collected: `2026-08-31T19:29:11Z`
- Contribution: agent-assisted research, architecture, fixtures, and producer review

## Exact Inputs

- Accepted contract-IR head: `bb5d30cbb1519b7ac286250114c96ba967661cba`
- Contract-IR corpus implementation: `5c49ebfd1c87415f74420ad047392bd03b1bd202`
- Package schema SHA-256: `748d98def7c0a67e3e12f882cd9ef7d0948c8eacbff1e5f6135faa7fd29d642d`
- Conformance schema SHA-256: `63fe642ebe7e7f49acf59094a8edaa488b96b13806886f0af2779629900bdb75`
- Corpus manifest SHA-256: `aed86fa6fd5e88412b3a771b594011884ef6df1e8256827ccf87bc9ae53fced4`
- Historical ADR source revision: `a642c91c3560c022276b77a31ee54141b3a8f97a`
- Real-FR source revision: `ce852bb77cc3a30df56aba5ce60e49aa44449e34`
- Real-FR file SHA-256: `c929d1fa4638430d201c4f9e28f856eb803e26cfdecb1bb82382f7c5c2a308db`

## Artifact Digests

| Artifact | SHA-256 |
|---|---|
| `research/adr-0010-shared-variable-fixture.tsv` | `2a32739bf9abb04afc4150e5eaa748be6463272be4d4cc358f430092e8b3a806` |
| `spec/architecture/ADR-0010-analysis-algebra.md` | `52d2d7a34ff593d8b0ee6698079e658ab40e499bf7f0aba396cdaa9d1af1103c` |
| `planning/adr-0010-research.md` | `85e08804eff4de35f1e512dd61f8891fd5ef5c1ef318c1751ccf773761fca06e` |
| `tests/adr_0010.rs` | `2764ee1c7ec37697682ba5ffaa1b18c7f99dcd51645e9da5e438b49b9dbd005b` |

## Outcomes

| Command | Outcome |
|---|---|
| `CARGO_TARGET_DIR=target make ci` | pass; 9 tests, 0 failed, 0 ignored |
| `quire validate --scope . 'spec/**/*.md' 'planning/**/*.md' 'plan/**/*.md'` | pass; 43 documents |
| `quire validate --scope . 'spec/**/*.md'` | pass |
| `CARGO_TARGET_DIR=target-msrv cargo +1.75.0 test --locked` | pass; 9 tests, 0 failed, 0 ignored |
| `CARGO_TARGET_DIR=target RUSTDOCFLAGS=-Dwarnings cargo doc --locked --no-deps` | pass |
| `git diff --check` | pass |

TC-009 exhaustively evaluated 55 unordered fixture pairs. Complete IR, name-only, typed-name, and
explicit reviewed binding candidates produced `(0, 0, 2)`, `(2, 12, 0)`, `(2, 3, 0)`, and
`(2, 0, 0)` respectively. Incompatible root kind, type, observation, and exact execution-point
members reject. The direct SMT example has three checked upper-bound assertions.

## Review and Gap Disposition

REV-005 closes A10-F01 through A10-F08, including three high findings: undefined cross-requirement
type identity, ambiguous execution-point compatibility, and missing direct-encoding upper bounds.
REV-006 finds no unresolved issue #6 blocker and maps all production implementation to existing issue
#7. No duplicate ticket was filed.

## Limitations and Authority

- The 11-reference fixture is adversarial architecture evidence, not prevalence data. Its explicit
  labels are a human-reviewed premise, not an inference result.
- The accepted upstream corpus has one expression owner and cannot measure cross-requirement alias prevalence.
- No production hash encoder, type-shape projection, analysis-model validator, SMT lowering, solver,
  semantic conclusion, or cross-platform result exists; those claims remain open.
- Cross-execution-point binding is explicitly unsupported in v0.1.
- Hosted CI remained manual-only and was not dispatched.
- This record accepts an architecture decision, not a source-release, qualification, accreditation,
  certification, or consuming-project decision. AA-001 remains open.
