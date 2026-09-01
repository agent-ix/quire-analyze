# Issue #23 atomic publication fault validation summary

> **Producer validation attestation.** This record binds the exact source revision and observed
> local commands. It does not claim independent approval, PGM-01 derivation authority, source
> release, or that an uncatchable terminated process removed its own named staging file.

## Subject

- Source revision: `1e69613bb4eb4d72a4011959fc587afc8d3f89fd`
- Source tree: `082202b38d76e9f69165d7a789a904d25d7c41ad`
- Collected: `2026-09-01T00:06:38Z`
- Host: Linux 6.18.33.2-microsoft-standard-WSL2 x86_64
- Default toolchain: rustc/cargo 1.94.1
- MSRV lane: Rust 1.75.0
- Specification validator: Quire 0.31.0 (CLI 4f6ed024, engine 0.46.0@ca7362d4)
- Scope: native QA issue #23 recoverable failure, concurrency, durability, and crash-state slice

## Observed Local Validation

The producer ran `make ci` from the exact clean subject commit with writable local Cargo cache and
target paths. It exited zero without dispatching hosted CI.

| Gate | Observed result |
|---|---|
| formatting and Clippy warnings-as-errors | pass |
| stable default tests | 50 passed, 0 failed, 1 explicit pinned real-engine test ignored |
| cargo-deny advisories, bans, licenses, sources | pass; three unmatched-license warnings only |
| unsafe-comment audit | pass |
| specification/plan validation | pass; installed-module duplicate-definition warnings only |
| Rust 1.75 locked tests | 50 passed, 0 failed, 1 explicit pinned real-engine test ignored |
| rustdoc with warnings denied | pass |
| LLVM coverage | 91.49% lines overall; 91.62% `report.rs`; 90% project floor passed |
| retained evidence checksum verification | pass for every previously declared artifact |

## Fault and Recovery Census

- create, partial-write, file-sync, rename, directory-sync, and cleanup failures are injected through
  the production state machine;
- every recoverable pre-rename error preserves the destination, closes and removes owned staging,
  and synchronizes the cleanup directory entry;
- a post-rename parent-sync error retains complete destination bytes and returns
  `published-durability-unknown` rather than inviting a destructive retry;
- eight simultaneous production publishers produce exactly one complete winner and no recoverable
  staging residue;
- an unknown-owner stale staging collision is refused and not deleted;
- child-process exits after create, write, file sync, rename, and directory sync prove that no
  partial destination becomes visible at any boundary.

## Retained Limitation

The three pre-rename termination probes leave one private named staging entry because the process is
no longer running and cannot execute its cleanup path. The destination remains absent. Prefix-based
cleanup by an unrelated process would be unsafe without an interprocess ownership protocol because
a matching publisher can still be live. REV-022 through REV-024 therefore hold issue #23 closure on
review of the literal zero-residue crash wording instead of recording a false cleanup claim.

## Review Bindings

- Code review: REV-022
- Gap analysis: REV-023
- QA assessment: REV-024
- Requirement-tagged test: TC-008
