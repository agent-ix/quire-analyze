# Issue #25 shared assurance dual-run summary

> **Producer validation attestation.** The source revision and commands below are immutable and
> reproducible, but no raw command transcript or machine-signed verdict is retained. This record does
> not claim independent review, derivation-evidence authority, or a release decision.

## Subject

- Source revision: `12702ba2483152e3e4491b9bd09a7de782fa9668`
- Inherited base: `2d99ef03b0099082f21d726642e86e6777966d00` (`origin/issue-23-publication-faults`)
- Collected: `2026-09-01`
- Host: Linux 6.18.33.2-microsoft-standard-WSL2 x86_64
- Default toolchain: rustc/cargo 1.94.1; MSRV lane Rust 1.75.0
- Pinned toolchain: quire-cli 0.31.0 (engine quire-rs 0.46.0), quoin 0.23.1, ix-flow 0.0.4,
  engineering-assurance 0.2.0
- Scope: the exact-revision dual run required before the superseded local evidence verifier is
  removed

## The baseline was already red, and this record says so

`cargo test` at the inherited head `2d99ef0` **failed**. The last of the thirteen inherited commits
added `evidence/publication-faults-1e69613/validation-summary.md`, whose attestation banner omitted
the two disclaimers `retained_evidence_is_censused_and_cannot_claim_machine_verification` requires,
so committing that record broke the gate the record itself claimed had exited zero. Those commits
had never been pushed, so nothing ever ran against them.

| Gate | At `2d99ef0` (inherited) | At `12702ba` (candidate) |
|---|---|---|
| `cargo test` | **FAIL** — 1 test, `retained_evidence_is_censused_and_cannot_claim_machine_verification` | pass — 59 passed, 0 failed, 1 ignored |
| `make verify-evidence` (old path) | pass | pass |
| `make ci` | **FAIL** (via `cargo test`) | pass |

No green baseline is claimed for the inherited head. The defect is fixed in `12702ba`.

## Dual run at the identical candidate revision

Both paths were run against `12702ba`, unmodified, in the same working tree.

| Path | Command | Result |
|---|---|---|
| Old (superseded) | `make verify-evidence` → `sha256sum --check evidence/manifest.sha256` | pass; 8/8 records OK |
| New (shared) | `make compat-view` → `scripts/legacy_evidence_view.py` | pass; 8 retained records, all `unreadable`; 7/7 fixtures matched; 6/6 mutation probes detected |

The new path is a strict superset of the old one. `sha256sum --check` verifies that each declared
record still hashes to its declared digest. The compatibility view does that **and** compares the
census in both directions — a retained record that is undeclared, and a declared record that is
absent, are each reported — **and** binds each manifest digest as the `expected_digest` the pinned
upstream mapping checks against, so an altered retained byte reads as a tampered source rather than
being mapped as though nothing had happened.

`evidence/manifest.sha256` is **frozen, not deleted**: it remains the digest source the compatibility
view binds. What is removed is the verifier, not the record.

## Complete local gate at `12702ba`

`make ci` exited zero without dispatching hosted CI.

| Gate | Observed result |
|---|---|
| formatting and Clippy warnings-as-errors | pass |
| stable tests | 59 passed, 0 failed, 1 ignored (the pinned real-engine corpus) |
| Rust 1.75 locked tests | 59 passed, 0 failed, 1 ignored |
| cargo-deny advisories, bans, licenses, sources | pass |
| unsafe-comment audit | pass |
| specification/plan validation (`quire validate`) | pass; installed-module duplicate-definition warnings only |
| rustdoc with warnings denied | pass |
| LLVM coverage | 91.49% lines overall; 90% floor passed |
| retained evidence checksum verification | pass; 8/8 |
| shared pin classification | 4/4 compatible; 0 artifact mismatches; 0 mirror references |
| legacy compatibility view | 8 retained records `unreadable`; 7/7 fixtures; 6/6 probes |
| assurance chain | 6 proofs attested; 13/13 cases; 7 states demonstrated |

## Attested proof results, as read from producer bytes

| Proof | Result | Read from |
|---|---|---|
| PROOF-solver-state-census | passed | `solver-state-census.json` `outcome` |
| PROOF-engine-availability | **unavailable** | `engine-availability.json` `outcome` |
| PROOF-shared-pins | passed | `shared-pins.json` `outcome` |
| PROOF-legacy-compatibility | passed | `legacy-compatibility.json` `outcome` |
| PROOF-quire-static-export | passed | populated Quire export |
| PROOF-msrv | passed | cargo's `build-finished` `success` |

`unavailable` is genuine and not simulated: the pinned Z3 5.1.0 and cvc5 1.3.4 assets are not
installed on this host, so the real-engine differential corpus did not run and nothing about it was
decided. It reaches the receipt as `result_unavailable`.

The receipt is `incomplete` with a missing decision. No ix-flow decision event exists for this
change and none was synthesized, because only the repository owner may create one.

## Reviewed Exceptions and Limits

- Hosted CI was not dispatched. The workflow trigger remains exactly `workflow_dispatch`.
- The real-engine differential corpus remains unrun; native issue #24 stays open.
- `engineering-assurance` v0.2.0 records `pending_human_acceptance` and ships no
  `human_acceptance_recorded` predicate; the acceptance is on that repository's `main` at `ae50e13`.
  Reported, not gated on. `agent-ix/engineering-assurance#20`.
- The pinned mapping refuses all eight retained records because they are Markdown narratives rather
  than PGM-01 envelopes. `agent-ix/engineering-assurance#21`.
- This record covers the dual run and the local gate. It is not independent review, not a release
  decision, and not a certification.

## Review Bindings

- Code review: SR-001
- Gap analysis: SR-002
- Requirement-tagged tests: TC-011, TC-012
