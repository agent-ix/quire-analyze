---
id: SR-17033
title: Code and Rust review of VER-77 PR 83 analysis_claim translation
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze (quire-analyze#83, diff origin/main...HEAD); src/package_translation.rs, src/lib.rs, tests/package_translation.rs, Cargo.lock, spec/functional/FR-009-package-runner.md"
review_set: subset
---

## Summary

Ticket: VER-77. Code review with the Rust review lane folded in, scoped to
`git diff origin/main...HEAD` of quire-analyze#83 (head fd244f6), checked against the
translation section of `spec/functional/FR-009-package-runner.md` (AC-19 to AC-27)
and `spec/test/TC-015-package-runner.md` case 19.

Measured with Rust 1.98.1 through `locked-build.sh`: `cargo test --test
package_translation` passes (14 tests), `cargo clippy --lib --test package_translation
-- -D warnings` is clean, `cargo fmt --check` is clean. Probes were run from a
scratch copy of the branch (tests built the same way as the PR's tests, not committed).

## Verdict

**FAIL on FND-001.** Per-clause translation is correct: clause order, ids, anchor,
requirement, declarations in ascending node order, spans, path-based cycle detection
(a shared DAG node is not misreported as a cycle), reference hops followed in a loop
so chains do not deepen the stack, and pre-order counting that stops at 10,001 without
expanding a 2^61 tree. The `Cargo.lock` change is exactly what
`cargo update -p quire-contract-model --precise 270410bb` produces from the `main`
lock (measured: cargo moves the six other branch-`main` git dependencies with it), so
it is not a finding. No vendored or committed fixture; packages are built at test time.
FND-001 is an unbounded aggregate cost across claims.

## Scope

- examined: FR-009 Identity and anchor: "The runner shall build one clause per analysable clause, in the order of `semantic_graph.nodes`, with `ClauseId` `claim-N` for node `N`, `ClauseKind::Assertion`, and the anchor `ExecutionPoint::Handler` named `analysis`."
- examined: FR-009 Terms table and "The operation identity is `CheckedApplicationOperation::read().identity`".
- examined: FR-009 Node bound: "The translation shall count every `Expression` it produces, each inlined copy counted again, and shall stop at the first count above 10,000 ... The translation shall do work proportional to this bound, never to the size of the expansion it refuses."
- examined: FR-009 Depth bound.
- examined: FR-009 Value declarations.
- examined: FR-009 Source spans.
- examined: FR-009 Refusals.
- examined: FR-009-AC-23.
- context_only: FR-009 Dependencies gap 6 (lock bump).

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | high | The node bound resets per claim, so total work and memory are claims x 10,000. Measured: 200 claims that each reference one shared 8,191-node `and` tree, in a 241,775-byte package, took 126 s to translate (debug build) at 1.79 GB peak RSS, and the request was then refused anyway by `lower_analysis_request` ("generated query exceeds the public byte bound"). A 1 MiB package holds about 4x as many claims. FR-009 bounds only "the translated `Expression` tree" of one clause, so the spec needs an aggregate bound too. | src/package_translation.rs:261-265, 430-433, 590-597; spec/functional/FR-009-package-runner.md:129-139 | missing-requirement |
| FND-002 | medium | FR-009 names `CheckedApplicationOperation::read().identity`, but `read()` returns `Err(NotAdmitted)` on every operation of a package read by `ContractPackageWireV1::read` (measured: `identity()=Some("quire.op.boolean.and") read()=Err(NotAdmitted)`). The code's `operation.identity()` is right and the spec is wrong. | spec/functional/FR-009-package-runner.md:117; src/package_translation.rs:560 | wrong-requirement |
| FND-003 | medium | FR-009-AC-23 says "a chain of 129 nested applications is refused, one of 128 is not". Under the depth rule (root at depth 1, each operand one deeper), 128 nested `not` over a parameter put the `ValueReference` at depth 129, which is refused. The test (correctly) refuses 128 nested and accepts 127, so the AC text contradicts both the depth rule and the test. | spec/functional/FR-009-package-runner.md:140-145, 313; tests/package_translation.rs:696-712 | wrong-requirement |
| FND-004 | medium | `#[trace("FR-009-AC-6", ...)]` and `#[trace("FR-009-AC-19", ...)]` sit on unit tests that only assert a `TranslationError`. AC-6 and AC-19 state runner behaviour (exit 2, no report, cause on stderr), which TC-015 assigns to cases 6 and 18. TC-015 case 19 says the unit test claims "only what a unit test shows". The tags inflate coverage before `run-package` exists. | tests/package_translation.rs:633, 666 | implementation-bug-despite-evidence |
| FND-005 | low | FR-009 names "the node whose translation produced the 10,001st `Expression`" but does not fix the count order or which node is named for a parameter leaf. The code counts in pre-order and names the node whose body holds the reference (the test asserts this). Two implementers could name different nodes. | spec/functional/FR-009-package-runner.md:134-139; src/package_translation.rs:197-199, 521, 575 | wrong-requirement |

## Recommendations

- FND-001: add one count across all claims of the package, refused above a fixed total
  (for example 100,000, `MAX_EXPRESSION_NODES` of `src/smt.rs`), naming the node that
  produced the first `Expression` over it. Add a test with many claims over one shared tree.
  Suggested spec text, appended to the Node bound bullet: "The count shall also be kept
  across all claims of the package, without reset between claims, and the translation
  shall stop at the first total above 100,000 and exit `2` naming the node
  `semantic_graph.nodes[N]` whose translation produced that `Expression`." Add the case
  to AC-23.
- FND-002: replace FR-009 line 117, "The operation identity is
  `CheckedApplicationOperation::read().identity`, an entry of
  `quire.checked-operation-catalog/v1` of `quire-verification-contracts`," with "The
  operation identity is `CheckedApplicationOperation::identity()`, an entry of
  `quire.checked-operation-catalog/v1` of `quire-verification-contracts`
  (`CheckedApplicationOperation::read()` returns `NotAdmitted` for a package read by
  `ContractPackageWireV1::read`, which installs no admitted operation views); an
  application with no identity is one the translation cannot lower,".
- FND-003: replace in AC-23 "a tree deeper than 128 (a chain of 129 nested applications
  is refused, one of 128 is not)" with "a tree deeper than 128 (128 nested `not`
  applications over a Boolean parameter, depth 129, are refused; 127, depth 128, are not)".
- FND-004: drop `FR-009-AC-6` and `FR-009-AC-19` from the two `#[trace]` lists; they belong
  to the `run-package` tests of VER-73.
- FND-005: add to the Node bound bullet: "`Expression` nodes are counted in pre-order, an
  application before its operands, and a `ValueReference` or `BooleanLiteral` is produced
  by the node whose body holds the reference or literal."
