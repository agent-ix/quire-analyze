---
id: SR-004
title: "Gap analysis of the legacy-evidence deletion"
type: SpecReview
analysis: gap-analysis
review_set: all
scope: "a33b68f..HEAD, issue #29"
---
# SR-004: Gap analysis of the legacy-evidence deletion

## Summary

Scope check on an irreversible deletion. Three questions: is the scope in issue
#29 fully executed, is every surviving matrix row still backed by a real test,
and did the deletion leave any orphan — a criterion with no test, a test with no
criterion, a proof obligation with no producer, or a claim about a file that no
longer exists. Six findings: zero high, two medium, four low. Both mediums are
FIXED. Nothing in scope is left undone.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-001 | medium | TC-012 backs three criteria (AC-4, AC-5, AC-6); deleting it whole would orphan AC-6, and deleting only AC-4 would leave TC-012's title and procedure describing work that no longer happens. | FIXED | wrong-requirement |
| FND-002 | medium | `tests/foundation.rs` asserts an exact census of ✅ matrix rows (32) and of FR-006 ✅ rows (6). Deleting a row without moving both numbers fails the gate; moving them without deleting the row hides a row that lost its test. | FIXED | correct-requirement-no-evidence |
| FND-003 | low | The shared brief's gate "0 unbacked rows" is unreachable in this repository. | ACCEPTED (pre-existing) | correct-requirement-no-evidence |
| FND-004 | low | The brief predicted 7 legacy-compat fixture *files*; there are 6 files declaring 7 cases. | ACCEPTED (both counts recorded) | wrong-requirement |
| FND-005 | low | `engineering-assurance#21` loses this repository as a contributor but is not closed here. | ACCEPTED (out of scope) | wrong-requirement |
| FND-006 | low | `assurance/change-assurance.json` keeps `record_id: quire-analyze/issue-25` while its definition is amended under #29. | ACCEPTED (deliberate) | correct-requirement-no-evidence |

## FND-001 — the test case that had to be narrowed, not deleted · FIXED

TC-012's declared scope was three things: the read-only compatibility view
(FR-006-AC-4), the fixture corpus that separated `incompatible` / `unreadable` /
`stale` / a readable control (contributing to FR-006-AC-5), and the closed
`scripts/` census plus the absence of a local evidence verifier (FR-006-AC-6).
Only the first two die with the evidence. FR-006-AC-6 — "no repository-local
generic runner, envelope, manifest, identity framework, retention store, audit
store or aggregate verdict remains in the execution path" — is the criterion this
whole campaign exists to establish, and it is not affected by the owner's
decision.

Two options were weighed. Folding AC-6 into TC-011 and deleting TC-012 collapses
two distinct claims (the intake path works / no local machinery remains) into one
test case and loses a traceability edge. Keeping TC-012 and narrowing it to its
surviving claim keeps the edge and costs a rename.

**Resolution: narrowed.** `spec/test/TC-012-legacy-compatibility-view.md` is
renamed to `TC-012-no-local-generic-machinery.md`, retitled, and rewritten to the
AC-6 claim alone; its `verifies FR-006` and `verifies NFR-002` relationships and
its `TC-012` id are unchanged. `FR-006-AC-5` now cites TC-011 only, which already
backs it through `every_non_conclusive_solver_state_stays_its_own_answer` and
`an_absent_engine_is_unavailable_and_never_a_decision`. No orphan on either side:
every surviving FR-006 criterion names a test, and every surviving test names a
criterion.

## FND-002 — two exact censuses that had to move together · FIXED

`foundation_plan_advances_only_first_unblocked_child` asserts
`complete_rows.len() == 32` and that exactly 6 of them start `| FR-006 |`. Those
are deliberate freeze counts: a new ✅ row without an executable trace binding is
supposed to break the build.

Deleting the AC-4 row moves both. **Fix.** 32 → 31 and 6 → 5, which is exactly
one row in exactly the right family. **Probe:** re-inserting the deleted AC-4 row
fails the test with `right: 31`, so the census is still load-bearing rather than
a number that was simply lowered to fit.

## FND-003 — the "0 unbacked rows" gate is not reachable here · ACCEPTED

Measured, not assumed. `quire coverage --scope . --json` on a clean `git archive`
of `origin/main` at `a33b68f` reports **one** unbacked row: `FR-003-AC-6` at
`spec/functional/FR-003-bounded-adapters.md:50`, whose declared verification is
`Inspection`, which binds no symbol. The same single row is reported at this
head.

| | `origin/main` `a33b68f` | this head |
|---|---|---|
| `unbacked_rows` | 1 (`FR-003-AC-6`) | 1 (`FR-003-AC-6`) |
| `no_symbol_rows` | 1 (same row) | 1 (same row) |
| `coverage.backed` | 50 / 53 | 49 / 52 |
| `authoring.tag_rate` | 52 / 64 | 50 / 62 |
| `unmatched_tags` | 8 | 7 |
| `untracked_symbols` | 2 | 2 |

The deltas are exactly the deletion and nothing else: one matrix row, two tests,
and one unmatched tag (`SHA-256`, picked out of the deleted
`retained_evidence_is_censused_and_cannot_claim_machine_verification`). **No new
unbacked row, no new unmatched tag, no new untracked symbol.** The honest gate
result is "one pre-existing unbacked row, unchanged", not "zero".

`status_lies: []` is reported but proves nothing here: both matrix tables head
their status column `Coverage Status` where quire expects `Status`, which is open
issue #14. That is #14's fix and its flip probe, not this change's, and nothing
in this change relies on `status_lies`.

## FND-004 — the fixture count · ACCEPTED

The shared brief expected 7 legacy-compat fixture files. `tests/fixtures/
legacy-compat/` held **6 files** — five derived fixtures and `expectations.json`
— declaring **7 cases**, because two of the seven are `release-control` cases read
from the installed release rather than committed here. Both counts are recorded
so neither reads as a miscount.

## FND-005 — `engineering-assurance#21` · ACCEPTED, not closed here

This repository was one of the contributors to
`agent-ix/engineering-assurance#21` ("`map_pgm01_bytes` has no reader for
`quire.derivation-evidence/v1`"), though its own contribution was the sharper
case: its records are Markdown and never parse at all, so the mapping answers
`unreadable` rather than `incompatible`. After this change it retains nothing and
contributes nothing to that issue. `engineering-assurance#7` records that #21
closes as moot rather than as fixed. That is another repository's issue and is
deliberately not touched from here.

## FND-006 — the record id · ACCEPTED, deliberate

`assurance/change-assurance.json` is the FR-063 change-assurance record body.
Its `record_id` stays `quire-analyze/issue-25` and its `revision` stays `1`.
Re-keying it to #29 would assert a fresh record chain (`revision: 1`,
`parent_digest: null`) about a different change, and every neighbouring artifact
— `CLAUDE.md`, `requirements-assurance.txt`, `examples/solver_state_census.rs` —
identifies this lane as issue #25's work. Instead the `purpose` field now states
plainly that the declaration was established under #25 and amended under #29 when
the preservation constraint was released, so the record does not silently present
amended content under an unchanged description.

## Scope execution against issue #29

| # | Scope item | Done |
|---|---|---|
| 1 | Delete `evidence/` including the frozen manifest | 10 files |
| 2 | Delete `scripts/legacy_evidence_view.py` | 491 lines |
| 3 | Remove `PROOF-legacy-compatibility`, its `INPUTS` entry, `PRESERVE-legacy-bytes`, `UNKNOWN-retained-evidence-is-not-pgm01` | done; `evidence` also dropped from `subject.scope` |
| 4 | Remove `compat-view`, `compat-fixtures`, `COMPAT_RESULT`, the `assurance-inputs` line and the `assurance:` prerequisite | done; `ci:` prerequisites unchanged |
| 5 | Delete `tests/fixtures/legacy-compat/` | 6 files |
| 6 | Keep `schemas/differential-report-v1.schema.json` | kept — live, see SR-003 FND-001 |
| 7 | Delete `FR-006-AC-4` and its row; narrow TC-012; drop TC-012 from the AC-5 row | done |
| 8 | Delete the AC-4 test, invert the manifest assertion, drop the allow-list entry, delete `retained_evidence_is_censused…` | done |

Out of scope and untouched, as issue #29 states: [#27](https://github.com/agent-ix/quire-analyze/issues/27)
(deferred SR-001/SR-002 findings) and [#28](https://github.com/agent-ix/quire-analyze/issues/28)
(the Make execution-control guard). Neither is worked or closed. The guard is not
re-added.

**One item of #27 is resolved as a side effect and is reported rather than
closed:** SR-002 `FG-21` — "`pairs_with` checks that a name exists, not that the
partner exercises the same path" — was raised against
`tests/fixtures/legacy-compat/expectations.json` and
`legacy_evidence_view.py::view_fixtures`, both of which are now deleted. The
identically named check in `assurance_chain.py`, which pairs chain controls with
chain scenarios, is untouched and the finding still stands there. #27 stays open
and is not edited from this change.

## Mutation probes

Four, each run at this head, each observed red then green:

| Probe | Expected | Observed |
|---|---|---|
| `mkdir evidence && touch evidence/manifest.sha256` | `no_local_generic_machinery_remains` fails | FAILED: *"a local evidence retention tree is back"* |
| `touch scripts/collect_evidence.py` | same test fails on the closed allow-list | FAILED, naming `collect_evidence.py` |
| re-insert the deleted `FR-006-AC-4` matrix row | the ✅ census fails | FAILED: `right: 31` |
| append one byte to the installed `engineering_assurance/compatibility.py` | `make pins` fails on the new digest pin | `outcome: failed`, exit 1, mismatch named; restored → exit 0 |

## Gates at this head

`make ci` **exit 0** · 61 tests passed, 0 failed, 1 ignored, on stable and MSRV
1.75 (63 → 61: the two deleted tests) · line coverage **91.05%** (floor 90) ·
chain **12/12 cases matched**, 7 states demonstrated (`inconclusive`,
`malformed`, `partial`, `passed`, `tampered`, `unavailable`, `unsupported`) ·
5 proof obligations, all within `accepted_results` · `quire coverage` 49/52
backed, 1 pre-existing unbacked row · hosted CI not dispatched.

## Verdict

**Six findings: zero high, two medium, four low.** Both mediums FIXED, four lows
ACCEPTED with the measurement behind each. Issue #29's scope is fully executed.
No orphan criterion, no orphan test, no proof obligation without a producer, and
no surviving claim that a deleted record verifies anything.
