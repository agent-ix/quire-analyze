---
id: SR-17050
title: "Code review of adversarial hybrid enclosure soundness tests"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; tests/hybrid_enclosure_soundness.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-040", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-040", type: reviews }
---
# Code Review of Adversarial Hybrid Enclosure Soundness Tests

## Summary

Ticket: VER-94. PR agent-ix/quire-analyze#91, diff `origin/main...HEAD` (one file,
`tests/hybrid_enclosure_soundness.rs`, 680 lines). `src/hybrid_synthesis.rs` is
unchanged. Includes the rust-review lane (rust-style loaded). Focused run with
toolchain 1.98.1: 14 passed, 12 ignored; `-- --ignored`: all 12 fail, each with
the panic its ignore reason names.

The oracle enumerates concrete executions in `i128` from the request fields
alone (no call into module internals): per step every state flows by every
integer delta, stays in its mode, and also takes every transition whose guard
contains the flowed state. That is the non-forced semantics of FR-040-AC-4 and
is correct for the request type.

## Verdict

**FAIL (one medium).** The AC-7 test cannot pass for any provider of the current
request type and attributes to the module a violation that does not exist. The
other ignored tests fail for the stated reasons and every passing test can fail.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `fr040_ac7_state_leaving_a_dropped_invariant_is_not_reported_reached` builds the invariant-dropping request itself, the translation FR-040-AC-7 forbids; under the submitted model state 8 is reachable, so any sound provider reports it and the test can never pass. No adapter builds a `HybridRequest` outside the module and tests, and the exhaustive struct literal in `request()` already proves at compile time that the type carries no invariant. | `tests/hybrid_enclosure_soundness.rs:434-452` |
| FND-002 | low | In `fr040_ac12_extreme_requests_end_in_exactly_one_typed_outcome_without_panic` the `matches!` over `hybrid_method_outcome` is tautological (a total const match onto exactly those three variants); only the no-panic part can fail. | `tests/hybrid_enclosure_soundness.rs:627-634` |
| FND-003 | low | `nfr040_ac2_huge_horizon_returns_in_bounded_time` leaves a detached thread iterating `u64::MAX` steps for the rest of the test process after the 2 s timeout fires. | `tests/hybrid_enclosure_soundness.rs:669-680` |

## Coverage

Examined every test, the oracle, `assert_sound`, `exceeds_i64`, `exact_hull`, and
`reach`/`validate_hybrid`/`overlaps`/`join_interval` in `src/hybrid_synthesis.rs`.
Rust lane: imports grouped std/external/crate, no `unwrap` in non-test helpers
beyond documented `expect`, ignore reasons carry finding ids. No hash, SHA or
pin ceremony and no roadmap or standard text in the diff.
