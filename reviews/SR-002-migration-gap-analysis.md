---
id: SR-002
title: "Gap analysis and adversarial false-green hunt for the issue 25 migration"
type: SpecReview
analysis: gap-analysis
review_set: all
scope: "2d99ef0..HEAD, the shared assurance migration"
---
# SR-002: Gap analysis and adversarial false-green hunt

## Summary

The migration was reviewed adversarially before the pull request opened, by a
reviewer whose only instruction was to break it. Twenty-five findings: five
high, nine medium, eleven low. All five highs and nine of the mediums and lows
are fixed with mutation probes; the rest are deferred to a tracked issue. Wave
1's three high false greens were specifically attacked and did not reproduce.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-001 | high | [FG-01] The chain reported `passed` and exit 0 with three proofs declaring they established nothing; only byte identity was gated. | FIXED | correct-requirement-no-evidence |
| FND-002 | high | [FG-02] A one-line hand-written file was a complete MSRV proof, and the attestation named a compiler that did not produce the stream. | FIXED | correct-requirement-no-evidence |
| FND-003 | high | [FG-03] The impact snapshot claimed completeness about a file nothing opened; `{}` was green. | FIXED | correct-requirement-no-evidence |
| FND-004 | high | [FG-04] `PRESERVE-legacy-bytes` promised absolutely what this branch had already done once. | FIXED | wrong-requirement |
| FND-005 | high | [FG-05] The analysis status was asserted by the obligation and checked by nothing, so any non-conclusive state could claim a decided status. | FIXED | correct-requirement-no-evidence |
| FND-006 | medium | [FG-06] The demonstrated-states test recomputed the driver's own expression and could not fail. | FIXED | correct-requirement-no-evidence |
| FND-007 | medium | [FG-07] The "census" of generic machinery was a six-name blocklist. | FIXED | correct-requirement-no-evidence |
| FND-008 | medium | [FG-08] `an_absent_engine_is_unavailable_and_never_a_decision` contains an unfalsifiable assertion. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-009 | medium | [FG-09] Three assertions are green only because the pinned solvers are absent. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-010 | medium | [FG-10] Two discriminator fields were read under the wrong key and were null for every case; the stale fixture demonstrated nothing. | FIXED | correct-requirement-no-evidence |
| FND-011 | medium | [FG-11] Four producers attest the crate version as their own tool version. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-012 | medium | [FG-12] `configuration_digest` sealed files that do not determine producer behaviour. | PARTIALLY FIXED | correct-requirement-no-evidence |
| FND-013 | medium | [FG-13] TC-011 declared two steps nothing implemented, including the one that would have caught FG-01, while the matrix said ✅. | FIXED | correct-requirement-no-evidence |
| FND-014 | medium | [FG-14] The census reached 15 of 24 outcomes, claimed 22 in prose, and published a count with no denominator. | FIXED | correct-requirement-no-evidence |
| FND-015 | medium | [FG-15] "All eight retained records" — there are nine. | FIXED | correct-requirement-no-evidence |
| FND-016 | low | [FG-16] The single-altered-byte probe asserted that SHA-256 is a hash. | FIXED | correct-requirement-no-evidence |
| FND-017 | low | [FG-17] `worst()` and `RESULT_PRECEDENCE` were dead code documenting a doctrine that never ran. | FIXED | correct-requirement-no-evidence |
| FND-018 | low | [FG-18] `Row.mode: Option` documented a branch that would panic. | FIXED | correct-requirement-no-evidence |
| FND-019 | low | [FG-19] `tamper_probes` returns an empty list on unparseable bytes. | DEFERRED (#27) — caught downstream by a length assertion. | correct-requirement-no-evidence |
| FND-020 | low | [FG-20] The tamper control does not check that its own seal succeeded. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-021 | low | [FG-21] `pairs_with` checks that a name exists, not that the partner exercises the same path. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-022 | low | [FG-22] `demonstrates` labels mix chain artifacts with solver-state vocabulary. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-023 | low | [FG-23] `receipt-re-verifies` accepts exit 0 or 1. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-024 | low | [FG-24] `mirror_references` scans six fixed paths; the obligation's claim is broader. | DEFERRED (#27) | correct-requirement-no-evidence |
| FND-025 | low | [FG-25] Two commands are used for the same attested fact across two scripts. | DEFERRED (#27) | correct-requirement-no-evidence |

## Why this review exists

Wave 1 of this campaign shipped seventeen false greens, three of them high, and
its own self-review found none of them. The worst sealed `result: "passed"` for
every proof without reading what the producer wrote — rewriting every producer
output to total failure still gave exit 0.

So this migration was reviewed adversarially before the pull request was opened,
by a reviewer whose only instruction was to break it. It did not read the code
and reason about it; it mutated producer output, planted scripts, injected
producer calls into the driver, relabelled states, and re-ran the gates.

## Verdict

**Twenty-five findings: five high, nine medium, eleven low.** All five highs and
seven mediums are FIXED with mutation probes. Wave 1's three highs did **not**
reproduce.

## High — all fixed

### SR-002-FG-01 — the chain was green over proofs that established nothing · FIXED

Each honest scenario's `matched` was byte identity alone: did Quoin retain what
the producer wrote. `observedResult` was recorded and never gated. Setting the
census to `inconclusive`, the compatibility view to `unavailable` and the pin
gate to `not_computed` gave `cases: 13/13 matched`, `outcome: passed`, exit 0.
`statesDemonstrated` actually *grew*, so the "not everything can be a pass" test
was better satisfied by the degraded run. Only the literal string `failed` was
caught, and only incidentally.

**Fix.** Every proof declares `accepted_results`. Only `PROOF-engine-availability`
may be `unavailable`, and only because an absent pinned engine is a fact about
the host rather than about this change. `matched` now requires byte identity
*and* an acceptable result, and `worst()` — the precedence doctrine the module
documented and never executed (FG-17) — produces a `proofResultRollUp`.
**Probe:** the original demonstration now gives `10/13`, three named mismatches,
exit 1.

### SR-002-FG-02 — the MSRV proof proved nothing, and attested the wrong compiler · FIXED

`echo '{"reason":"build-finished","success":true}' > msrv.jsonl` was a complete
MSRV proof: chain exit 0, ten tests passing. Separately, `tool_version("cargo")`
ran bare `cargo --version` while the declared command was `rustup run 1.75.0
cargo check` — so the sealed attestation stated **1.94.1** for a stream produced
by **1.75.0**.

**Fix.** The stream must contain a `compiler-artifact` for this crate. The
version is read from the toolchain the declared argv names, so command and
attestation cannot drift. **Probe:** the one-line file now exits 2 with *"A
build-finished line on its own is not a compilation"*; `tool_version("cargo",
"1.75.0")` returns `1.75.0`.

### SR-002-FG-03 — a completeness claim about a file nothing opened · FIXED

`impact_snapshot` hardcoded `completeness: complete`, `truncated: false`,
`gaps: []`. `echo '{}' > quire-static-export.json` still sealed a record
asserting a complete, ungapped snapshot, and no test read the export at all.

**Fix.** All three are derived from the export document; an empty section is
`incomplete` and named in `gaps`. **Probe:** `{}` now gives a mismatch and
exit 1.

### SR-002-FG-04 — the declaration promised something this branch had already broken · FIXED

`PRESERVE-legacy-bytes` said *"Every byte under `evidence/` is read-only to this
migration … it never rewrites a record to make a check pass."* This branch
rewrote `evidence/publication-faults-1e69613/validation-summary.md` and
re-derived its manifest line. `censusMismatches: 0` was therefore earned partly
by updating the expected value.

The correction itself was right — the record's banner omitted disclaimers its own
gate requires, and committing it broke `make ci`. The *claim* was wrong.

**Fix.** The constraint now describes the assurance path, which genuinely cannot
rewrite anything: the view opens every record read-only. The one correction is
recorded as a declared exception pinned by its pre-correction digest
`9f15af50…`, with the reason, and noted as a human-reviewed commit visible in git
history rather than a check updating its own oracle.

### SR-002-FG-05 — the analysis status was asserted by the obligation and checked by nothing · FIXED

The census recorded `analysisStatus` per row and compared it to nothing. Nothing
downstream compared it either. Rewriting every row's status to `"satisfied"` —
collapsing five distinct statuses to one — left the census `passed`, ten tests
green, chain exit 0. A timed-out, signaled, malformed-output or version-mismatched
solver could report the analysis status of a *decided* analysis, invisibly to the
entire lane. This is the precise distinction this repository exists to protect.

**Fix.** `Row` carries `expected_status`, checked per row into `mismatches`. The
traced test asserts the pairing and that no non-conclusive outcome carries a
decided status.

## Medium

| ID | Finding | Disposition |
|---|---|---|
| FG-06 | `each_demonstrated_state_is_backed_by_a_case_that_matched` recomputed the driver's own expression over the same JSON — it could not fail, and accepted a case claiming a fabricated state. | **FIXED** — asserts an independent state-to-case list plus a closed allow-list. Probe: a fabricated label now fails. |
| FG-07 | `no_local_generic_machinery_remains` called itself a census and was a six-name blocklist; a `.py` extension and a `_v2` suffix defeated it. | **FIXED** — a closed allow-list of the scripts this repository declares. Probe: a planted `collect_evidence_v2.py` now fails it. |
| FG-10 | The compatibility view read `record_id`/`schema_version` while the mapper returns `source_record_id`/`source_schema_version`, so two of the three fields `expectations.json` named as discriminators were `null` for **all seven cases** — and the `stale` fixture's verdict was identical to its unmutated control, so it demonstrated nothing. | **FIXED** — keys corrected, and the stale case now discriminates on the mapped evidence state, which is where the mapper actually surfaces staleness. `stale` now appears in `statesDemonstrated`. |
| FG-13 | TC-011 declared two procedure steps that were never automated — including the one that would have caught FG-01 — while the matrix marked TC-011 ✅ Complete. | **FIXED** — both run against a mutated copy of producer output, with an unmutated positive control first. |
| FG-14 | The census reached 15 of 24 outcomes while its own prose claimed 22, and published `distinctOutcomes: 15` with no denominator. | **FIXED** — 20 of 24 now, the denominator published, the four unreachable ones named, and an exhaustive `match` that breaks the build on a new variant. |
| FG-15 | "All eight retained evidence records" — there are nine; this migration added the ninth and did not update the count. | **FIXED** — the count is read from the census, not restated. |
| FG-17 | `worst()` and `RESULT_PRECEDENCE` were dead code documenting a doctrine that never executed. | **FIXED** with FG-01. |
| FG-08 | `an_absent_engine_is_unavailable_and_never_a_decision` asserts `matches!(outcome, "passed" \| "unavailable")` over a producer that can only emit those two, and its `assert_ne!(outcome, "failed")` sits inside `if outcome == "unavailable"`. Unfalsifiable. | **DEFERRED** — [#27](https://github.com/agent-ix/quire-analyze/issues/27). Adjacent assertions are real; this one is decorative. |
| FG-09 | Three assertions are green only because the pinned solvers are absent; installing them turns the lane red. The unavailable demonstration records a permanent environmental absence, not a preserved capability. | **DEFERRED** — [#27](https://github.com/agent-ix/quire-analyze/issues/27). The honest fix is a synthetic attestation so the scenario holds regardless of what is installed. Recorded plainly in the PR body. |
| FG-11 | Four producers attest the crate version as their tool version, including two Python scripts the crate version does not describe. | **DEFERRED** — [#27](https://github.com/agent-ix/quire-analyze/issues/27). Not a fabricated version, but a version that does not track what it names. |
| FG-12 | `configuration_digest` sealed files that do not determine producer behaviour — the Z3 pin could be swapped without moving the digest. | **PARTIALLY FIXED** — the two census/availability proofs now seal their own source. The remaining two seal specification files that genuinely configure them. |

## Low

`FG-16` the single-altered-byte probe asserted that SHA-256 is a hash —
**FIXED**, it now probes the binding against the retained digest · `FG-18`
`Row.mode: Option` documented a branch that would panic — **FIXED**, the type is
now `&'static str` · `FG-19`, `FG-20`, `FG-21`, `FG-22`, `FG-23`, `FG-24`,
`FG-25` — **DEFERRED** to [#27](https://github.com/agent-ix/quire-analyze/issues/27).

## Attacked and could not break

Wave 1's three highs do not reproduce here, and the reviewer verified that
positively rather than by absence:

- **Producer isolation is genuine.** Injecting `cargo build --offline` and
  `quire coverage --scope . --json` into the driver makes the test fail, naming
  both invocations. Both really resolve through the shimmed `PATH`; the `quoin`
  control run really does fail with a non-empty log. Re-verified after the shim's
  exemption was widened to version queries anywhere in argv — needed so the MSRV
  proof can read its pinned toolchain, and costing nothing because neither
  injected command carries a version flag.
- **No fabricated version.** `tool_version` raises for an absent tool, a non-zero
  exit, or output with no semver. No `0.0.0` anywhere.
- **`derive_result` never defaults.** Unreadable output exits 2; an unlisted
  outcome exits 2.
- **Quoin intake genuinely discriminates** — feeding it untampered bytes makes the
  tamper control correctly go red, so it detects the tamper rather than an error.
- **The five report tamper probes are real**, each a single edit to authoritative
  bytes, with the positive control checked first.
- **Committed compatibility fixtures are re-derived at run time** from
  digest-pinned release bytes and compared byte-for-byte.
- **Declared commands match executed commands**, including the
  `.venv-assurance/bin/python` interpreter split.

## Coverage of the migration's own acceptance criteria

Every FR-006 criterion has a named executable test. FR-006-AC-1
`adopted_pins_are_classified_upstream_and_name_no_mirror` · AC-2
`attestation_results_are_read_from_producer_output`,
`the_chain_never_executes_a_producer_and_the_probe_can_prove_it`,
`a_producer_that_did_not_pass_cannot_produce_a_green_chain` · AC-3 the export
scenario · AC-4
`retained_evidence_is_read_through_the_pinned_mapping_without_being_changed` ·
AC-5 `every_non_conclusive_solver_state_stays_its_own_answer`,
`an_absent_engine_is_unavailable_and_never_a_decision` · AC-6
`no_local_generic_machinery_remains`.

## Gates at this head

`make ci` exit 0 · 63 tests passed, 0 failed, 1 ignored, on stable and MSRV
1.75 · coverage 91.05% (floor 90) · census 24/24 rows matched, 20/24 outcomes
reached · 6/6 compatibility mutation probes detected · chain 13/13 cases, 7
states demonstrated · hosted CI not dispatched.
