---
id: SR-001
title: "Code review of the thirteen previously unreviewed domain commits"
type: SpecReview
analysis: code-review
review_set: all
scope: "c5f181c..2d99ef0 plus the issue #25 migration diff"
---
# SR-001: Code review of the thirteen previously unreviewed domain commits

## Summary

Thirteen commits implementing this crate's entire domain had never been pushed,
reviewed, or run by anything but their author. This is their first review. Two
high defects were found and demonstrated: a failed solver process being read as
a conclusive answer, and a report validator that never asked the evidence it
retains whether it agreed with the conclusion it carries. Both are fixed with
mutation-probed regression tests. Five medium and nine low findings are
dispositioned below.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-001 | high | [SR-001-H1] The Z3 exit-code-1 exception read a bare `unsat` with a failed exit as a conclusive answer, and only for one engine label. | FIXED | implementation-bug-despite-evidence |
| FND-002 | high | [SR-001-H2] Report validation proved self-consistency but never re-derived the conclusion or the query identity from the evidence the report retains; a forged conclusion validated and the CLI exited 0. | FIXED | implementation-bug-despite-evidence |
| FND-003 | medium | [SR-001-M1] Five distinct version-probe failures collapse into one `IdentityError` with one diagnostic. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-004 | medium | [SR-001-M2] The Z3 exit-1 catch-all swallowed every other parse verdict into `nonzero-exit`. | FIXED | implementation-bug-despite-evidence |
| FND-005 | medium | [SR-001-M3] A `Sat` whose model fails replay verification is still `is_conclusive()`. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-006 | medium | [SR-001-M4] Default limits permit a render larger than the validator's own byte ceiling. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-007 | medium | [SR-001-M5] `contractIrRevision` was shape-checked but never compared to the compiled-in constant. | FIXED | implementation-bug-despite-evidence |
| FND-008 | low | [SR-001-L1] The `pre_exec` SAFETY comment misstates its load-bearing invariant. | DEFERRED (#27) | missing-requirement |
| FND-009 | low | [SR-001-L2] Clearing `FD_CLOEXEC` is unnecessary and leaks a read fd of the pinned binary. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-010 | low | [SR-001-L3] `setpgid` race between fork and the child's own call; `Command::process_group(0)` removes it and one `unsafe`. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-011 | low | [SR-001-L4] PID reuse after `try_wait` reaps the child. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-012 | low | [SR-001-L5] `drain` has no iteration ceiling and ignores the wall deadline. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-013 | low | [SR-001-L6] `Capture::eof` is dead state. | DEFERRED (#27) | missing-requirement |
| FND-014 | low | [SR-001-L7] The contradiction check is whitespace-token based. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-015 | low | [SR-001-L8] TOCTOU between `metadata` and `read` in the CLI. | DEFERRED (#27) | implementation-bug-despite-evidence |
| FND-016 | low | [SR-001-L9] A `#[cfg(test)]` branch sits inside the production publication path. | ACCEPTED — the only way to observe crash boundaries; compiles to nothing outside the test binary. | implementation-bug-despite-evidence |
| FND-017 | low | [SR-001-T1] The three "census is closed" tests assert over hand-maintained arrays and would not fail on a new enum variant. | DEFERRED (#27) — the migration's census producer covers it from the other side with an exhaustive `match`. | implementation-bug-despite-evidence |
| FND-018 | low | [SR-001-T2] Nothing pins `status_for` against `status_for_values`, two transcriptions of one table. | DEFERRED (#27) | implementation-bug-despite-evidence |

## Why this review exists

Thirteen commits implementing this crate's entire domain — `0da1747` through
`2d99ef0` — had never been pushed to GitHub. No branch on origin, no pull
request, no review. `main` contained a stub `src/lib.rs`. They existed on one
disk.

So this is not a review of a delta against reviewed work. Everything at or below
`2d99ef0` was treated as unaudited: 5,014 lines of `src/`, 3,152 lines of tests,
the SMT lowering, the bounded solver adapters, the analysis conclusions, the
report format and the publication state machine.

The review was adversarial and ran the code. Every finding below was
demonstrated, not inferred.

## Verdict

**Two high defects, five medium, nine low.** Both highs are now FIXED with
mutation-probed regression tests. The crate's central property — that a solver
which could not decide is never reported as one that decided — was violated in
one place and unverifiable in another.

## High

### SR-001-H1 — a failed process was read as a decision · FIXED

`src/solver.rs`. The Z3 exit-code-1 exception existed for one real shape: Z3
answers a non-`sat` status, then fails the `(get-model)` the query asked for, and
exits 1. As written it required neither that the query request a model nor that
the response contain the model-unavailable error. A bare `unsat\n` with a failed
exit and empty stderr was therefore read as a conclusive `unsat` — and only under
`SolverEngine::Z3`, while the identical bytes under `Cvc5` were correctly
`nonzero-exit`.

Demonstrated with a fake solver `printf 'unsat\n'; exit 1`: Z3 → `Unsat`, cvc5 →
`NonzeroExit`.

An outcome that depends on which engine label the adapter was handed is not a
property of the response. This is the exact inversion of what this crate exists
to prevent.

**Fix.** Both conditions are now required, each documented as load-bearing. The
new `is_model_unavailable_response` recognises the motivating shape explicitly
rather than inferring it from `parse_response` returning `Ok` — which it also
does for a bare `unsat`. The catch-all arm now propagates the specific
non-conclusive verdict instead of collapsing `contradictory-output`,
`malformed-output` and `model-limit` into `nonzero-exit` (SR-001-M2, fixed with
it).

**Regression.** `a_nonzero_exit_is_not_a_conclusion_for_either_engine` covers
three modes and asserts both engines agree. **Probe:** removing the two guard
conditions makes it fail with *"Z3 read unsat_exit_one as the conclusive Unsat;
a process that reported failure decided nothing"*.

### SR-001-H2 — validation never asked the evidence whether it agreed · FIXED

`src/report.rs`. `validate_report_document` proved `stdoutHex` matched
`stdoutSha256`, that `queryHex` matched `querySha256`, and that `reportDigest`
covered the payload. All of that is self-consistency, which an author editing
both halves of a pair together satisfies. It never re-derived the `outcome` from
the retained stdout, and never re-derived `queryDigest` from the `requestDigest`
and query bytes the document already carries — even though both inputs are
fields of the report.

`src/main.rs` turns this function's result straight into a conclusive exit code.
Demonstrated: a `refuted` report forged into `satisfied`, resealed, validated
`Ok`, and the CLI exited 0.

**Fix.** For the outcomes where stdout *is* the evidence — `sat`, `unsat`,
`unknown` — the claim is re-parsed from the retained bytes. `queryDigest` is
re-derived from `requestDigest` and the query bytes. The failure outcomes are
deliberately not re-derived: they are decided by process state that stdout alone
cannot witness, and requiring agreement would reject honest records.

`reportDigest` remains an unkeyed SHA-256. It establishes transcription
integrity, not authenticity, and this fix is the difference between
"self-consistent" and "re-derived from evidence".

**Regression.**
`a_forged_conclusion_is_refused_by_re_derivation_from_retained_evidence`, with a
positive control first. **Probes:** removing the stdout re-derivation fails with
*"a report whose retained stdout says unsat validated as sat"*; removing the
query re-derivation fails with *"the query bytes were replaced and queryDigest
still validated against them"*.

## Medium

| ID | Finding | Disposition |
|---|---|---|
| SR-001-M1 | Five distinct version-probe failures collapse into one `IdentityError` with one diagnostic string. The crate's own anti-property applied to the identity probe. | **DEFERRED** — [#27](https://github.com/agent-ix/quire-analyze/issues/27). Fail-closed today; the states are lost to a reader, not to the gate. |
| SR-001-M2 | The Z3 exit-1 catch-all swallowed every other parse verdict into `nonzero-exit`. | **FIXED** with H1. |
| SR-001-M3 | A `Sat` whose model fails replay verification is still `is_conclusive() == true`; only `explanation` is downgraded. The differential path guards this via `verified_if_required`; the single-engine public API does not. | **DEFERRED** — [#27](https://github.com/agent-ix/quire-analyze/issues/27). Changing `is_conclusive` is a public-API semantic change and belongs to its own ticket with its own review. |
| SR-001-M4 | Default limits permit a ~130 MB render against a 64 MB `MAX_REPORT_BYTES`, so a legitimate run can produce a report its own validator rejects. | **DEFERRED** — [#27](https://github.com/agent-ix/quire-analyze/issues/27). Latent: unreachable at the sizes any current corpus produces. |
| SR-001-M5 | `contractIrRevision` was shape-checked by the schema and never compared to the compiled-in constant, so a report from a different contract-IR revision validated. | **FIXED** — added to the pinned-field loop, with a regression assertion. |

## Low

`L1` the `pre_exec` SAFETY comment misstates its own load-bearing invariant (the
`CString`s are *not* captured; only erased pointers are, so correctness rests on
the vectors outliving `spawn()` in the same frame) · `L2` clearing `FD_CLOEXEC`
is unnecessary and leaks a read fd of the pinned binary · `L3` a `setpgid` race
between fork and the child's own call · `L4` PID reuse after `try_wait` reaps ·
`L5` `drain` has no iteration ceiling and ignores the wall deadline · `L6`
`Capture::eof` is dead state · `L7` the contradiction check is token-based ·
`L8` TOCTOU between `metadata` and `read` in the CLI · `L9` a `#[cfg(test)]`
branch inside the production publication path.

All **DEFERRED** to [#27](https://github.com/agent-ix/quire-analyze/issues/27).
`L3` is the one worth doing first: `Command::process_group(0)` removes the race
*and* one `unsafe` operation, and this crate's own test already uses it.

## Test quality

**ACCEPTED with a recorded reservation.** The three "census is closed" tests
assert over hand-maintained arrays declared inside the test, so a
twenty-fifth `SolverOutcome` would not fail them. The census producer added by
this migration now carries an exhaustive `match` that *does* break the build on a
new variant, which covers the gap from the other side. Nothing pins
`analysis.rs::status_for` against `report.rs::status_for_values` — two
independent transcriptions of one table — and a drift would silently make every
honest report fail validation. **DEFERRED** to
[#27](https://github.com/agent-ix/quire-analyze/issues/27).

`all_ten_truth_table_cells_match_independent_finite_models` over-claims in its
name: the oracle selects which canned response a fake solver returns, so nothing
independently checks the lowering. **ACCEPTED** — correct as a classification
test.

## Checked and found sound

The more valuable half of an audit of never-reviewed code.

`verify_model` / `parse_boolean_model` could not be made to accept a model that
does not satisfy the assertions — exact symbol-set equality, no duplicates, no
unexpected symbols, no trailing tokens, and every replay assertion evaluating to
`Some(true)`, with a missing symbol yielding `None` which cannot pass ·
`parse_response` head parsing, including the three exact model-unavailable shapes
· `valid_s_expression`, including SMT-LIB2 string escaping, `|…|` quoting,
comments, single-top-level-list enforcement, and no depth underflow · executable
pinning: the digest is taken from an open fd, `fexecve` runs that same fd, and
the digest is re-taken after both the version probe and the query run, so
replacing the binary at the path cannot swap what executes · argv is never shell
interpreted, `env_clear()` with only `LANG`/`LC_ALL` · the precedence chain's
fail-closed ordering everywhere except H1 · the publication state machine:
`DestinationUnmodified` holds on every pre-rename stage because
`renameat2(RENAME_NOREPLACE)` fails atomically, and the eight-thread race
produces exactly one winner with no residue · canonical form is not vacuous
(`serde_json` here has no `preserve_order`, verified through `cargo tree -e
features`) · no reachable panic in `validate_report_document` under `{}`, `[]`,
`null` or a wrong-typed field, probed with `catch_unwind` ·
`disposition_from_values` faithfully mirrors `compare_solver_records` including
the `verified_if_required` guard · every `AdapterLimits` field is bounds-checked
with a one-over test · integer conversions at process boundaries · the six
`unsafe` blocks are genuinely async-signal-safe with an empty waiver baseline.

## Gates

`cargo clippy --all-targets -- -D warnings` clean · `cargo fmt --check` clean ·
`cargo test` 63 passed, 0 failed, 1 ignored · `check_unsafe_comments.sh` passes
with an empty baseline.
