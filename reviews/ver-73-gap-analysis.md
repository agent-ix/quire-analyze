---
id: SR-17036
title: Gap analysis of VER-73 PR 84 against FR-009, NFR-003 and TC-015
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze (quire-analyze#84, diff origin/main...HEAD at a0f0f3c); spec/functional/FR-009-package-runner.md, spec/nonfunctional/NFR-003-runner-invocation.md, spec/test/TC-015-package-runner.md, src/package_runner.rs, tests/package_runner.rs"
review_set: subset
---

## Summary

Ticket: VER-73. Planless gap analysis scoped to the PR diff. Plan completion was not
assessed. `quire matrix --scope .` (quire 0.44.0) reports every criterion in the repo as
`untagged`. The cause is repo-wide `criterion-tagged-outside-row` warnings from the
traceability model, not this PR. So each `#[trace]` tag was read by hand against its AC
text and the test's assertions. Every FR-009 AC from 1 to 27 and NFR-003-AC-1 to AC-3
has a test or an inspection target.

## Verdict

**CONDITIONAL**. These ACs are backed by assertions that can fail: AC-1/2/3, AC-4/17
(all twelve non-completed rows, plus completed `unknown`, `solver-error`,
`malformed-output` and sat against unsat), AC-5 (nine exit-4 scenarios, including
signal, no line, two lines and garbage), AC-7, AC-8, AC-9, AC-12 (the `0700` mode is
observed by the stub, and no leftovers remain after exits 0, 2, 3 and 4), AC-13, AC-16,
AC-18, AC-19 and AC-24. The stub producer-execute is realistic. Its result lines match
engineering-assurance 0.8.1's `CapturedStream` (bytes as a number array), and the real
executor validated the logged requests (AC-15) and gave `timed-out`/`stdout-limit` and
exit 3 (NFR-003-AC-1).

FR-009-AC-14 is not evidenced on this host: z3 and cvc5 are absent, and the test returns
`ok` on its skip branch. The spec allows the skip, and the PR body states it, so this is
not overclaiming. AC-14 stays unproven until it runs on a host with both solvers.

The gaps are:
- a spec determinism claim that real timing breaks (FND-001)
- AC-6 cases that do not show their stated cause (FND-002)
- TC-015 case 20 spawn coverage that covers only some constructs (FND-003)
- one misplaced trace tag (FND-004)

## Scope

- examined: FR-009-AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-7, AC-8, AC-9, AC-10, AC-11, AC-12, AC-13, AC-14, AC-15, AC-16, AC-17, AC-18, AC-19, AC-23, AC-24, AC-26, AC-27
- examined: FR-009 Behavior "The runner shall produce byte-identical report bytes for identical package bytes, identical solver executables and identical limits."
- examined: NFR-003-AC-1, NFR-003-AC-2, NFR-003-AC-3 (inspection: no stdin read, `env::var` only in cfg(test) publication code)
- examined: TC-015 cases 1-21
- context_only: NFR-001 Statement (elapsed times excluded from the deterministic projection); src/producer_evidence.rs:657-658 (`elapsed_ms` from `timing.executionNanos`); src/report.rs:743 (`elapsedMs` rendered)

## Findings

| ID      | Severity | Summary | Refs |
| ------- | -------- | ------- | ---- |
| FND-001 | medium | FR-009 requires byte-identical report bytes for identical package bytes, executables and limits. The report renders `elapsedMs`, taken from `producer-execute` `timing.executionNanos`, so real runs differ. NFR-001 excludes elapsed times from the deterministic projection. FR-009-AC-11 and NFR-003-AC-2 pass only because the stub returns fixed timing. | spec/functional/FR-009-package-runner.md:284, src/producer_evidence.rs:657-658, src/report.rs:743 |
| FND-002 | medium | FR-009-AC-6: the "names a dependency" case (a `dependency_selections` mutation re-serialized through `serde_json::Value`) asserts only exit 2 and `--package` on stderr, so it could be refused for a reason other than the empty dependency map. The at-limit case exits 2 for another reason and shows "not refused for size" only by the absence of the text "read limit". | tests/package_runner.rs:694-697, tests/package_runner.rs:730-745 |
| FND-003 | low | TC-015 case 20 requires spawning the binary over each case-19 refusal. Spawn tests cover only `quire.op.integer.lt`, the 61-node chain and empty occurrences. The integer_range claim, the integer operand, the non-expression reference, the cycle, depth 129, 10,001 nodes and a 100,001 total are shown only by unit tests. | tests/package_runner.rs:987-1022 |
| FND-004 | low | `identical_inputs_give_identical_bytes_and_leave_no_trace` is tagged FR-009-AC-26 (equal translations) but asserts report bytes and directory state, not clause order, ids, declarations or spans. AC-26 is covered correctly at tests/package_translation.rs:234. | tests/package_runner.rs:839 |
