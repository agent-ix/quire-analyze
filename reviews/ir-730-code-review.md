---
id: SR-5580
title: IR-730 Rust MSRV dependency review
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze@1c0625699abe8dfc5167c040ca5fccbf8616ec88; Cargo.toml; Cargo.lock; Makefile; .github/workflows/ci.yml; tests/hybrid_synthesis.rs; tests/foundation.rs"
review_set: subset
---

## Summary

Ticket: IR-730. I applied the formal Rust-review checklist to the manifest, lockfile, test consumer, and unchanged gate recipes. The candidate narrows the sole `proptest` dev-dependency from the semver-compatible `1.8.0` range (which locked 1.11.0) to the 1.8 series and selects 1.8.0 in Cargo.lock. The old package declares Rust 1.85; the new one declares Rust 1.74. No Rust source, test, CI recipe, MSRV declaration, or threshold changed.

## Examined units

| Unit | Role | Excerpt |
| --- | --- | --- |
| Cargo.toml | examined | `rust-version = "1.75"`; `[dev-dependencies] proptest = "~1.8.0"` |
| Cargo.lock | examined | `name = "proptest"`; `version = "1.8.0"`; registry checksum and dependency set from Cargo resolution |
| Makefile | examined | `msrv: $(CARGO) +1.75.0 test --locked`; `ci` retains `msrv` |
| .github/workflows/ci.yml | examined | Installs Rust 1.75.0 and runs `make ci` |
| tests/hybrid_synthesis.rs | examined | `tc_014_canonicalizes_arbitrary_finite_set_like_inputs` uses `proptest!` and asserts synthesis equality under reversal; its trace names TC-014 and FR-008-AC-1 |
| tests/foundation.rs | context_only | Line 77 searches interface prose for `only a complete recognized sat/unsat response` |
| MP-001 M-09 | context_only | Local quality gates include an MSRV pass |

## Assurance Context

AP-001, `spec/assurance/AP-001-analyze-release.md`, applies to this 0.1 source candidate. Evaluated 1c0625699abe8dfc5167c040ca5fccbf8616ec88 against fbbf42fe03b698d472ff90c791c375a8a794e58e. The material concern is a false claim that the declared minimum compiler can build the test graph. AD-001, MP-001 M-09, the manifest, lockfile, Makefile, workflow, and focused Rust 1.75 log were available. AP-001 and MP-001 are structurally invalid on this baseline, so neither is treated as validated release evidence. No selected independence, structural-gap, producer-reliance, or human release decision was available; no standing exception was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

**PASS for the changed dependency scope.** Rust 1.75 compiled all crates and tests on this exact lockfile in `/tmp/ir730-msrv-focused.log`; the first failing test was the unchanged `tests/foundation.rs:77` prose assertion, so `make msrv` and the full gate are not yet green. This is an independently visible baseline failure, not a regression caused by the dependency change. Normal-toolchain testing and final full gates remain outstanding before PR/merge.
