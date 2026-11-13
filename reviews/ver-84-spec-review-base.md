---
id: SR-17037
title: "Base spec review of VER-84 enums, datatypes and records (FR-020..022, NFR-010, TC-020)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; quire-analyze#88 diff origin/main...HEAD: spec/functional/FR-020-datatype-lowering.md, spec/functional/FR-021-datatype-witness-replay.md, spec/functional/FR-022-datatype-package-translation.md, spec/nonfunctional/NFR-010-datatype-bounds.md, spec/test/TC-020-enum-datatype-analysis.md"
review_set: subset
---

## Summary

Ticket: VER-84. Base checklist review of the spec-only diff of quire-analyze#88: one commit, five
new files, 324 added lines. The sub-analyses ran as separate artifacts: integrity (SR-17038),
EARS conformance (SR-17039) and failure-domain (SR-17040). Criterion-strength analysis was skipped
because no Jev client is installed on this host. Code-review and gap-analysis were skipped because
the diff has no code. `quire validate --scope .` on the five files exited 0. `quire validate --okf`
exited 0 with 24 `dangling-trace-reference` warnings on the new Verification cells (SR-17038
FND-004). `quire` 0.44.0 (engine 0.59.1) and `quoin` 0.30.1 were used.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-022-AC-1 says the fixtures "exit `0` or `1` as the independent expectation requires", but does not fix which exit each fixture must produce. An implementation that returned the wrong one of the two could pass. Fix: name the expected status per fixture (for example, satisfiable membership exits `0` and the contradictory record-field claim exits `1`). | FR-022-AC-1, spec/functional/FR-022-datatype-package-translation.md:51 |

## Verdict

FAIL overall, on the integrity and failure-domain findings (two high and six medium across
SR-17038 and SR-17040). This checklist adds one low finding. The ACs are otherwise concrete and
falsifiable. The real-solver cases are scoped "where installed", which matches repo convention
(TC-010, TC-015). TC-020 covers each FR and NFR it declares.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | low | FR-020-AC-1 says "`or` of `role = Admin` and `role = Auditor`, with and without a further `role = Guest`". This does not say whether `role = Guest` is a further disjunct, which keeps the formula sat, or a conjoined assertion, which makes it unsat. Only "respectively" implies the second reading. Fix: say "with and without a further assertion `role = Guest`", as FR-022-AC-1 does. | FR-020-AC-1 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-022-AC-1 now fixes an exit per fixture: 0 for the sat membership claim, 1 for the membership claim with Guest, 0 for the record claim and 1 for the record claim with Guest. These match FR-009 (sat→0, unsat→1). |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | low | FR-022-AC-1 opens with "each with a fixed expected exit", but its fifth fixture (`quire.op.option.value` under `implies` with `quire.op.option.present`) only "translates to an `Unwrap` and runs". The AC gives no expected exit for it, only for its unguarded twin (`2`). Fix: state the claim and its exit, for example the guarded value equal to `Admin` with no contradicting claim exits `0`. | FR-022-AC-1 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-002 | fixed | FR-020-AC-1 is now one assertion `role = Admin or role = Auditor` plus one further assertion. `role != Admin` is `sat` and `role = Guest` is `unsat`, which is unambiguous and correct. |
