---
id: SR-003
title: "Code review of the legacy-evidence deletion"
type: SpecReview
analysis: code-review
review_set: all
scope: "a33b68f..HEAD, issue #29"
---
# SR-003: Code review of the legacy-evidence deletion

## Summary

An irreversible deletion of 17 tracked files and every reference to them. The
only question worth real scrutiny is whether anything still needs what was
removed, so this review is a reachability audit rather than a style pass: every
deleted artifact was traced to its consumers, and every consumer was either
deleted with it or rewritten to stop asking. Eight findings — one high, three
medium, four low. The high and all three mediums are FIXED; the four lows are
ACCEPTED or pre-existing and linked.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
|---|---|---|---|---|
| FND-001 | high | `schemas/differential-report-v1.schema.json` would have been deleted by the brief's "frozen schema family" step; it is a live output contract embedded by `src/report.rs`. | FIXED (kept) | wrong-requirement |
| FND-002 | medium | Deleting `PROOF-legacy-compatibility` drops `attestation_results_are_read_from_producer_output` from three cross-checked producers to two, silently weakening the test. | FIXED | correct-requirement-no-evidence |
| FND-003 | medium | Deleting the four `consumed_artifacts` pins would leave `PROOF-shared-pins` asserting "every pinned artifact still hashes to its digest" over an empty list — a vacuously true claim. | FIXED | correct-requirement-no-evidence |
| FND-004 | medium | `no_local_generic_machinery_remains` asserted the frozen manifest **is present**; deleting the manifest without inverting that assertion turns the surviving AC-6 test red for the wrong reason. | FIXED | implementation-bug-despite-evidence |
| FND-005 | low | `plan/PLAN-001-analyze-v01/` stated in the present tense that deleted records "retain" exact inputs, MSRV outcomes and closed findings. | FIXED | wrong-requirement |
| FND-006 | low | `adopted_pins_are_classified_upstream_and_name_no_mirror` required FR-006 to cite `engineering-assurance#21`, a gap this change makes moot. | FIXED | correct-requirement-no-evidence |
| FND-007 | low | `quire coverage` reports one unbacked row, `FR-003-AC-6`. Pre-existing on `origin/main`; not introduced here. | ACCEPTED (pre-existing) | correct-requirement-no-evidence |
| FND-008 | low | Both matrix tables still head their status column `Coverage Status`, so `status_lies` remains structurally inert. | DEFERRED (#14, pre-existing) | correct-requirement-no-evidence |

## FND-001 — the schema that looked frozen and is not · FIXED (kept)

The shared brief's step 6 is "schemas frozen only because retained envelopes
named them by digest", with an explicit instruction not to inherit a sibling's
freeze list. This repository has exactly one file under `schemas/`,
`differential-report-v1.schema.json`, and a name-based reading would have taken
it.

It is live. `src/report.rs:309` embeds it with `include_str!` and compiles it as
the Draft 7 validator that `validate_report_document` runs on **every**
`quire.differential-report/v1` document, and `tests/analysis_semantics.rs:853`
validates a rendered report against the same bytes independently. Deleting it
would not have failed a grep for `legacy`; it would have failed to compile, and
had it been reachable another way it would have removed the only schema check on
this repository's published report format.

**Kept. Zero schemas deleted.** The `pgm01Envelope` field in that schema and in
`src/report.rs` is this repository's own forward declaration of a PGM-01 identity
envelope — `status: "unavailable"` pending `quire-contract-ir#20` — and has
nothing to do with the retained tree. Likewise `interface-001`'s
`identity_envelope: quire.derivation-evidence/v1` declares the report identity
this crate will emit, not a record it retained.

## FND-002 — a test that would have quietly measured less · FIXED

`attestation_results_are_read_from_producer_output` exists to prove each
attestation result is *read from the producer's own bytes* rather than assumed.
It did that by comparing three named producers' documents to the chain's
`observedResults`. `PROOF-legacy-compatibility` was one of the three. Removing
its assertion and nothing else leaves a test with the same name, the same green
result, and one third less evidence behind it — and the remaining two are the
census and the availability probe, which on this host report `passed` and
`unavailable`, so the surviving pair no longer covers a third producer's shape.

**Fix.** The deleted assertion is replaced by the equivalent one for
`PROOF-shared-pins`, which was previously unchecked. The test still cross-checks
three producers. The `observed.len()` census moves 6 → 5, which is the honest
count of declared proofs.

## FND-003 — a digest gate over an empty list · FIXED

`assurance/pins.json` pinned four upstream artifacts by digest:
`verification_semantics.py`, `pgm01-compatibility-view-v1.schema.json`, and the
two `pgm01-v*.json` release fixtures. All four existed **only** to serve the
compatibility view — `verification_semantics.py` supplies `map_pgm01_bytes`, and
`legacy_evidence_view.py` was the only reader of the fixture pins.

Deleting all four leaves `consumed_artifacts: []`, and
`artifact_digest_mismatches` then iterates nothing and returns nothing.
`PROOF-shared-pins` would still state "every upstream artifact this repository
pins by digest still hashes to that digest" — true of no artifact. That is a
green check that checks nothing, which is the exact failure class this
repository's own `pairs_with` refusal was written to prevent.

**Fix.** The four dead pins are replaced by one live pin:
`engineering_assurance/compatibility.py`, sha256
`62829251…`. That module is what `check_shared_pins.py` actually imports —
`classify_all` and `accepted` are the sole authority on every version verdict the
surviving gate reports — so a silent upstream change to it would silently change
what `compatible` means here. This is a deviation from the shared brief, which
did not anticipate a repository whose entire pin set was compat-view-only.
**Probe:** editing one byte of the installed `compatibility.py` makes
`make pins` report the mismatch and exit 1.

## FND-004 — an assertion that had to be inverted, not deleted · FIXED

`no_local_generic_machinery_remains` is the executable backing for FR-006-AC-6,
the one FR-006 criterion that survives this change. It contained:

    assert!(root().join("evidence/manifest.sha256").is_file(),
            "the retained manifest was deleted; it is frozen, not removed");

Deleting `evidence/` without touching this turns the surviving test red with a
message asserting the opposite of the owner's decision. Deleting the assertion
outright would have been worse: the claim "no repository-local retention store
remains" would then rest on the `scripts/` allow-list alone, and an `evidence/`
tree could reappear with no script at all.

**Fix.** The assertion is **inverted**, not dropped: `!root().join("evidence")
.exists()`, with the reason stated — a reappearing `evidence/` tree is a local
retention store returning, not a record being preserved.
`legacy_evidence_view.py` is removed from the allow-list, which is closed, so the
script cannot return either. **Probe:** `mkdir evidence && touch
evidence/manifest.sha256` fails the test; `touch scripts/collect_evidence.py`
fails it separately.

## FND-005 — present-tense claims about records that no longer exist · FIXED

Three places stated that a deleted file retains something:
`Task-002`'s entire "Completion Evidence" section, `Task-003`'s "Current
Evidence" pointer, and `log.md`'s round-2 entry.

The trap in the brief is specific: such a claim is *removed* with the evidence and
is **not restated more weakly**. So Task-002 and Task-003 now say the record was
deleted, name the authority, and say plainly that nothing retained attests to the
work — they do not substitute a softer version of the original claim. `log.md` is
an append-only dated history, so its past entries are left exactly as written and
a new dated entry records the deletion and states that the earlier entries
describe what was true when written.

## FND-006 — a test requiring a citation this change makes moot · FIXED

`adopted_pins_are_classified_upstream_and_name_no_mirror` asserted
`FR_006.contains("engineering-assurance#21")`. That issue is "map_pgm01_bytes has
no reader for the envelopes four campaign repositories retained" — a gap that
only exists while the retained records do. `engineering-assurance#7` records that
it closes as moot rather than as fixed.

**Fix.** The `#21` assertion is removed along with the FR-006 bullet it guarded.
The `#20` assertion — the acceptance-packaging gap, which is unaffected — stays,
so the test still fails if FR-006 stops recording a live gap.

## FND-007 / FND-008 — pre-existing, measured, not touched

`quire coverage --scope . --json` reports **1 unbacked row**, `FR-003-AC-6` at
`spec/functional/FR-003-bounded-adapters.md:50`, whose declared verification is
`Inspection` and which therefore binds no symbol. Measured against `origin/main`
at `a33b68f` from a clean `git archive` tree: the same single row, identical.
This change moved the totals 53 → 52 and backed 50 → 49 — exactly the one row it
deleted — and introduced no new unbacked row. The shared brief's "0 unbacked
rows" is not reachable in this repository and was not reachable before this work
either; the honest gate result is "no *new* unbacked row, one pre-existing".

`status_lies: 0` is reported but structurally inert here: both matrix tables head
their status column `Coverage Status` where quire expects `Status`, which is
open issue [#14](https://github.com/agent-ix/quire-analyze/issues/14). This
change does not rename the column — that is #14's fix and its probe — and does
not rely on `status_lies` for anything.

## What was traced and found to need nothing

Every consumer of every deleted artifact, enumerated by grep over the whole tree
and confirmed after the change:

| Deleted | Who read it | Disposition |
|---|---|---|
| `evidence/**` (9 records) | `legacy_evidence_view.py`; `tests/foundation.rs::retained_evidence_is_censused…` | both deleted |
| `evidence/manifest.sha256` | the same two, plus `include_str!` in `tests/foundation.rs` and the presence assertion in `tests/shared_assurance.rs` | const and test deleted, assertion inverted |
| `scripts/legacy_evidence_view.py` | `Makefile` (`assurance-inputs`, `compat-view`, `compat-fixtures`); the `scripts/` allow-list | all removed |
| `tests/fixtures/legacy-compat/**` | `legacy_evidence_view.py`; `PROOF-legacy-compatibility.configuration` | both deleted |
| `legacy-compatibility.json` | `assurance_chain.py` `INPUTS`; two assertions in `tests/shared_assurance.rs` | all removed |
| `PROOF-legacy-compatibility` | `assurance_chain.py`; `change-assurance.json` | both removed |
| `FR-006-AC-4` | the matrix row; `change-assurance.json` requirements | both removed |
| `pgm01-v*.json` / `pgm01-compatibility-view-v1.schema.json` pins | `legacy_evidence_view.py::release_fixture`; `check_shared_pins.py::artifact_digest_mismatches` | reader deleted, pin set replaced (FND-003) |

`sha2` stops being used by `tests/foundation.rs` but remains a real dependency of
`src/report.rs`, `src/smt.rs`, `src/solver.rs`, three other test files and both
examples, so the manifest entry stays. `requirements-assurance.txt` still
installs `engineering-assurance`, which `check_shared_pins.py` imports.

## Verdict

**Eight findings: one high, three medium, four low.** The high and all three
mediums are FIXED. FND-005 and FND-006 are FIXED. FND-007 and FND-008 are
pre-existing, measured against `origin/main`, and left to their own issues.

Nothing in this repository still needs what was removed.
