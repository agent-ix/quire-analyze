---
id: SR-17079
title: code-review of VER-95 code PR 104
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-analyze; src/, tests/, crates/method-contract/, spec/; VER-95
  PR #104'
review_set: subset
relationships:
- target: ix://agent-ix/quire-analyze/FR-042
  type: references
---

## Summary

Reviewed frozen PR #104 head 673863e0a8d32d1964f8601f32e8a2df979e1bbf against origin/main cad3fdc03849d4147288adffb9c9b73d9bc76766. Ticket: VER-95. This is the code PR; SR-17070 belongs to the already merged spec PR #103, so these artifacts use distinct pr104 slugs. No branch edits or merge were performed.

## Verdict

FAIL — blocking findings remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Reject malformed or incomplete PHAVer output before constructing the normalized result. A completed stub returning the unterminated text `R = A.{m & 0 <= x <= 2` is accepted as an Enclosure for all requested steps; for the existing m=[0,2], m-to-t reset +5 request it silently fills t with None and omits reachable t=5..7. The parser accepts EOF as the end of a mode and validates only JSON it generated itself, so its strict JSON unit tests do not protect the public tool-result boundary. | src/hybrid_tool.rs:348-389 |
| FND-002 | high | Bound and clean up the producer-execute process itself, and share the existing invocation seam instead of adding a second implementation. The new invoke duplicates src/package_runner/execution.rs:296-340 (spawn, piped request, result read and wait); it has no deadline on write/read/wait and drops the existing oversized-response kill path. A stub that reads the request and sleeps leaves reach_hybrid blocked beyond 8 seconds despite the 5,000 ms tool budget and 1,000 ms cleanup ceiling; the reviewer had to kill its process group. Reusing a shared helper must also add the missing supervisor deadline. | src/hybrid_tool.rs:702-728 |
| FND-003 | medium | Do not require a nonempty enclosure for an empty oracle state set. oracle_next inserts a target entry before testing guard intersection, while assert_contains_oracle unconditionally unwraps that target interval. With m={0}, flow 0 and an m-to-t guard [1,1], the exact helper panics at step 1 on a correct result containing m={0} and explicitly empty t. Skip empty state sets or avoid creating them; retain an unreachable-target case from a reachable source. | src/hybrid_tool.rs:1264-1307 |
| FND-004 | medium | Gate the Unix-only directory API or provide a supported-platform implementation. hybrid_tool is included unconditionally from src/lib.rs, but CapabilityRoot::create imports std::os::unix::fs::DirBuilderExt and calls mode unconditionally. Non-Unix library builds now fail during name resolution, even for callers selecting tool=None; existing platform boundaries in package_runner and solver are conditionally compiled. This is a source-inspected portability regression; no Windows target was installed or executed. | src/hybrid_tool.rs:675-679 |
| FND-005 | low | Add the repository SPDX and copyright header to the new source file. The loaded rust-style implementation convention requires both on every source file; the adjacent hybrid_synthesis.rs provides the repository spelling. | src/hybrid_tool.rs:1 |

## Assurance Context

AP-001 (spec/assurance/AP-001-analyze-release.md) applies to this source candidate and its material false-conclusion/resource-containment scenarios. AD-001, CAC-001 and MP-001 were inspected. The hybrid result remains separate from SMT proof classifications; direct PHAVer spawning was not introduced. AD-001 still names the removed reach provider, so its boundary inventory is stale. The independent finite-set comparison and exact helper reproduction assess the selected oracle dimension, not global producer qualification. Full three-repetition MP-001 measurement records, a PHAVer-specific producer-reliance decision, cross-platform execution, structural coverage dispositions and release independence records were unavailable. No standing exception was found; release authority remains human-owned.

## Checks

The review independently ran make fmt-check lint test under the assigned locked-build wrapper, RUSTUP_TOOLCHAIN=1.98.1 and CARGO_PROFILE_DEV_DEBUG=0; exit 0. This broad command had finished when the lead clarified that review should run only focused checks, so it was not interrupted. No make ci, coverage, deny, documentation or aggregate final-candidate gate was run. The lead owns the final gate. Logs: /tmp/codex-theories/ver95-review-logs/gates.log.

Focused installed-PHAVer tests passed 4/4 in 28.86 seconds, including both unchanged F10 variants returning exactly Incomplete/tool_timeout with no enclosure, the smoke test and the current fit sweep. Evidence is recorded in focused-real-phaver.log. Public API reproductions are in reproduction.log, with probe.rs and stub executables; the exact unchanged oracle helper reproduction is in oracle-rust-probe.log. The separate finite-set comparison of interval Minkowski sums, guard intersections and additive resets passed 33,075 bounded combinations (oracle-probe.log); this checks the arithmetic independently and is not a substitute for the missing delegated fit fixtures.

The request uses an absolute PHAVer path, the model input-artifact role, producer-execute only, timeout 5,000 ms, input 16 MiB and stderr 1 MiB. The protocol caps stdout at 8 MiB, below NFR-001's 16 MiB ceiling; this is disclosed in the PR and the test expects the producer cap. NFR-001 is unchanged. The author source archive phaver/src/phaver.cpp:86-90 documents the -vXXXXXX output-detail and timer/progress flags; -v000000 is encoded as a literal argument. The author page and source archive are https://www-verimag.imag.fr/~frehse/phaver_web/index.html and https://www-verimag.imag.fr/~frehse/phaver_web/phaver.zip. PR #104 records version 0.38, GPLv2 licensing and external-execution evidence. No PHAVer source/binary is vendored in the repository.

The removed reach/replay_hybrid/bind_hybrid_result/hybrid_method_outcome symbols are absent. Canonical synthesis assertions are preserved and pass. No CI, Cargo or Makefile gate was changed. No fallback selects the old algorithm.

## Coverage

Plan completion: not assessed

Reconciliation: quoin matrix 0.30.1 and quire 0.45.0 / engine 0.60.0, followed by quire trace --scope . --id FR- --prefix --format json and the corresponding NFR- command, plus source inspection. Matrix raw output reports 203 untagged records and 20 method-without-symbol records (including NFR measurement rows); these are not treated as 203 real missing tests because the #[trace] matrix regression is recorded as PLAT-1672. Trace resolves 359 FR and 50 NFR binding claims. FR-010-AC-11 has a qualified #[ix_trace_rs::trace] marker that trace itself did not return; it is retained as a tooling limitation, not a missing-test finding. FR-042-AC-1 is conditional on the unselected keep-and-harden path and is not an active delegate-path gap. Eleven independently source-confirmed baseline AC trace gaps are listed separately from the PR regression. No run evidence was bound by quoin matrix.

The repository surface inventory covers src modules, their public types/functions, the publication and package-runner CLI paths, crates/method-contract, and tests. Existing behavior maps to FR-001 through FR-005, FR-008 through FR-012, FR-020 through FR-022, FR-030 through FR-034, FR-041/FR-042 and their NFRs. No new unowned production behavior or hollow production stub was found in this diff; the greeting scaffold is not promoted as requirement evidence. The review found a missing required fit-case census and an oracle false rejection, rather than treating passing coverage as semantic proof.

Optional whole-repository semantic review: skipped; no separate opt-in was supplied. The requested detailed hybrid oracle and intent checks ran under code-review. All AC scope entries below are examined for trace reconciliation only unless discussed in findings; no semantic PASS is claimed for untouched areas.

## Examined Scope

```yaml
scope:
- id: FR-008-AC-1
  path: spec/functional/FR-008-bounded-canonical-synthesis.md
  role: examined
  excerpt: Equivalent set-like inputs yield the same canonical candidate order and
    identity.
- id: FR-008-AC-2
  path: spec/functional/FR-008-bounded-canonical-synthesis.md
  role: examined
  excerpt: A candidate without independent validation is not a proof, and validation
    failure cannot become satisfaction.
- id: FR-008-AC-3
  path: spec/functional/FR-008-bounded-canonical-synthesis.md
  role: examined
  excerpt: Exhaustive no-candidate and incomplete bounded search are distinct terminal
    outcomes.
- id: FR-042-AC-1
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Where the recorded path is keep-and-harden, each case in the review's adversarial
    set yields either an enclosure containing every enumerated reachable state or
    a typed Incomplete or Refused outcome. This branch does not apply when the recorded
    path is delegate-and-remove.
- id: FR-042-AC-2
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove, reach, replay_hybrid, bind_hybrid_result and
    hybrid_method_outcome are absent from the crate, and no code path computes a hybrid
    enclosure inside the crate.
- id: FR-042-AC-3
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove, FR-007, FR-040, NFR-040, TC-013 and TC-040,
    including their index entries and test-matrix rows, are removed.
- id: FR-042-AC-4
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: For each fit case in TC-041 that returns an enclosure, the enclosure contains
    every state reached by the independent oracle for every mode and step.
- id: FR-042-AC-5
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove with a named tool, the tool is started only through
    producer-execute with the time and output limits of NFR-001, and a stub executor
    records exactly one request per valid hybrid call. Malformed request models are
    rejected before producer-execute and cause zero external launches.
- id: FR-042-AC-6
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove with the configured tool none, every valid hybrid
    request ends Refused with reason `tool_not_configured`, producer-execute request
    count and external invocation count are both zero, and no enclosure is returned.
- id: FR-042-AC-7
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: With a named tool, the crate validates the exact declared result schema,
    requires every requested mode at every requested step to be present exactly once
    or explicitly empty, and rejects unknown or missing members and inverted intervals.
    Timeout maps to Incomplete with reason `tool_timeout`; output-limit breach maps
    to Incomplete with reason `tool_output_limit_exceeded`; non-zero exit maps to
    Incomplete with reason `tool_exit_nonzero`; missing tool maps to Refused with
    reason `tool_unavailable`; schema-invalid output maps to Refused with reason `result_schema_invalid`;
    and an inverted interval
- id: FR-042-AC-8
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: The applied path is the recorded one only. Neither path contains a switch,
    flag or retry that selects the other.
- id: FR-042-AC-9
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: When IR-1469 is recorded, the applied path agrees with it, or the application
    ticket records why it does not. When IR-1469 is not recorded, the application
    ticket records that fact and the path proceeds.
- id: FR-042-AC-10
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: When a named external tool is selected, the application pull request records
    the tool version, the license terms published by its authors, and evidence that
    invoking it as an external executable is permitted.
- id: NFR-001-AC-1
  path: spec/nonfunctional/NFR-001-determinism-bounds.md
  role: examined
  excerpt: Repetition and irrelevant-order permutation preserve query and normalized
    evidence bytes.
- id: NFR-001-AC-2
  path: spec/nonfunctional/NFR-001-determinism-bounds.md
  role: examined
  excerpt: Every adapter process, time, input, output, model, and path boundary has
    a numeric limit and exact boundary test; lowering recursion remains owned by TC-010.
- id: NFR-001-AC-3
  path: spec/nonfunctional/NFR-001-determinism-bounds.md
  role: examined
  excerpt: A breached limit is explicit and never conclusive.
- id: TC-041-step-1
  path: spec/test/TC-041-delegated-hybrid-reachability.md
  role: examined
  excerpt: At each later step, it adds every integer produced by every allowed flow
    delta, keeps the flowed state in its source mode under non-forced transition semantics,
    and adds the reset image of each flowed state that satisfies a closed guard.
- id: TC-041-step-2
  path: spec/test/TC-041-delegated-hybrid-reachability.md
  role: examined
  excerpt: Valid passing cases include a guard touched at one point, a partial-guard
    reset, multiple transitions joined into one target, duplicate and reordered transitions,
    exact bounded-state enumeration, valid error-bound widening, an unreachable target,
    and bounded small models.
- id: rust-style-portability
  path: src/hybrid_tool.rs
  role: examined
  excerpt: '        use std::os::unix::fs::DirBuilderExt;'
- id: rust-style-header
  path: src/hybrid_tool.rs
  role: examined
  excerpt: //! PHAVer adapter for bounded hybrid reachability (FR-042).
bindings:
- test_id: tests::producer_failures_and_invalid_outputs_never_return_enclosures
  ac_id: FR-042-AC-7
  trace: correct
- test_id: tests::producer_request_uses_artifact_binding_and_declared_limits_once
  ac_id: FR-042-AC-5
  trace: correct
- test_id: tests::malformed_and_tool_none_requests_do_not_launch_a_process
  ac_id: FR-042-AC-6
  trace: correct
- test_id: tests::result_schema_rejects_unknown_fields_duplicate_modes_and_bad_rationals
  ac_id: FR-042-AC-7
  trace: correct
- test_id: tests::tc_041_fit_cases_contain_independent_finite_oracle_states
  ac_id: FR-042-AC-4
  trace: correct
- test_id: tests::f10_same_target_timeout_is_typed_incomplete_without_enclosure
  ac_id: FR-042-AC-7
  trace: correct
- test_id: tests::f10_distinct_modes_timeout_is_typed_incomplete_without_enclosure
  ac_id: FR-042-AC-7
  trace: correct
```


## Disposition pass 1

Reviewed fix 5bab70078d4e589762158fd6b93bf4c193adef22. Before appending these dispositions, verified the committed reviews/ copy was byte-identical to the original scratchpad artifact. Original findings and scope are preserved unchanged. No newly numbered finding was added; remaining failures are unresolved parts of the existing findings.

FAIL — FND-001 and FND-002 remain open; PR #104 is not mergeable.

Focused checks only: cargo test --lib hybrid_tool::tests -- --include-ignored --test-threads=1 passed 11/11 (37.44 s), including both unchanged F10 timeout cases, real fit sweep, malformed-output checks, ordinary supervisor cleanup, and empty-target regression. cargo test --test package_runner passed 32 tests with 2 existing ignored tests (25.67 s), covering the caller moved to the shared seam. Every cargo/rustc command used the assigned locked-build.sh wrapper, target directory, RUSTUP_TOOLCHAIN=1.98.1 and CARGO_PROFILE_DEV_DEBUG=0. No aggregate gate ran. Public API reproductions are in /tmp/codex-theories/ver95-disposition1/public-reproductions.log; test and probe-build logs are in the same directory. The first nested-group harness used captured stderr, which the surviving descendant held open; the repeated file-backed run measured the adapter return directly. Reproduction descendants were explicitly killed by the reviewer.

NFR-001 and the literal -v000000 PHAVer request argument are unchanged. The shared seam still invokes only producer-execute. Platform and header fixes were inspected directly. Baseline FND-002 through FND-012 in SR-17080 were checked against unchanged source and the explicit review scope.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The original unterminated prefix is now Refused/result_schema_invalid, but malformed inner constraints remain accepted. On current HEAD, completed stdout `R = A.{m & 0 <= x <= 2, t & };` returns Enclosure with t=None at every step for the existing request whose reachable t states are 5..7. A trailing empty disjunct is also accepted. src/hybrid_tool.rs:388-406 skips empty constraint tokens instead of refusing them. Reject empty constraints/disjuncts and cover them through the public adapter, not only the normalized JSON validator. |
| FND-002 | still-open | The duplicate invocation code was removed and ordinary stalled producers now time out, but cleanup remains incomplete. invoke_producer_execute signals only the outer producer process group and returns on timeout without joining the I/O workers. A stub creating a child with os.setpgrp (a separate group in the same session, matching engineering-assurance build_command process_group(0)) returns Incomplete/tool_timeout after about 5.55 s while its sleep child remains alive and threads_after_return=2. The reviewer killed the reproduction child group. src/producer_evidence.rs:186-191 and 235-257 must contain/terminate nested tool groups and retire bounded I/O workers before cleanup is claimed complete. |
| FND-003 | fixed | The oracle now creates target entries only for nonempty guard intersections and also skips empty expected sets. A focused regression accepts explicitly empty t for m={0}, flow 0, guard [1,1]. |
| FND-004 | fixed | DirBuilderExt and mode(0700) are guarded by cfg(unix); non-Unix code uses fs::create_dir. Unix-specific test helpers are gated. Source-inspected; no non-Unix cross-build was run. |
| FND-005 | fixed | The new file now has the repository SPDX and copyright header. |
| FND-001 | fixed | Round 2: outer framing and empty disjuncts fail closed; explicit-empty or omitted target output requires the request empty_targets authorization. Public-adapter tests cover unauthorized false and leading, trailing and doubled disjunction separators. |
| FND-002 | still-open | Still open and not waived. The fix changes only src/hybrid_tool.rs; src/producer_evidence.rs and the shared supervisor/cleanup path are unchanged from disposition pass 1. Nested tool process groups can therefore survive timeout and the I/O reader can remain detached. Per the explicit review brief, the required cleanup belongs in upstream Engineering Assurance; review of specification PR #250 is not a code fix, merge, or consumed implementation. Keep PR #104 blocked until the upstream implementation is delivered and consumed and the nested-group/worker-cleanup regression is verified. |
| FND-002 | fixed | Round 3: fixed for the supported producer-execute path. Merged upstream Engineering Assurance PR #251 (b77f902) supplies observed-descendant cleanup, direct-child reaping and completed reader joins. The installed 0.8.2 consumer returns its typed timeout result before the bounded 6,500 ms outer deadline; the PHAVer request remains exactly 5,000 ms. Quire adds no nested-process cleanup workaround. Independent focused hybrid tests pass, including both real F10 variants with no enclosure and the ordinary stalled-executor fallback. |


### Disposition evidence

```yaml
dispositions:
- fnd: FND-001
  outcome: still-open
  reason: The original unterminated prefix is now Refused/result_schema_invalid, but
    malformed inner constraints remain accepted. On current HEAD, completed stdout
    `R = A.{m & 0 <= x <= 2, t & };` returns Enclosure with t=None at every step for
    the existing request whose reachable t states are 5..7. A trailing empty disjunct
    is also accepted. src/hybrid_tool.rs:388-406 skips empty constraint tokens instead
    of refusing them. Reject empty constraints/disjuncts and cover them through the
    public adapter, not only the normalized JSON validator.
- fnd: FND-002
  outcome: still-open
  reason: The duplicate invocation code was removed and ordinary stalled producers
    now time out, but cleanup remains incomplete. invoke_producer_execute signals
    only the outer producer process group and returns on timeout without joining the
    I/O workers. A stub creating a child with os.setpgrp (a separate group in the
    same session, matching engineering-assurance build_command process_group(0)) returns
    Incomplete/tool_timeout after about 5.55 s while its sleep child remains alive
    and threads_after_return=2. The reviewer killed the reproduction child group.
    src/producer_evidence.rs:186-191 and 235-257 must contain/terminate nested tool
    groups and retire bounded I/O workers before cleanup is claimed complete.
- fnd: FND-003
  outcome: fixed
  after_excerpt: "                    if lower <= upper {\n                      \
    \  next.entry(transition.to_mode.clone())\n                            .or_default()\n\
    \                            .push((lower + reset, upper + reset));\n        \
    \            }\n                }\n            }\n  "
- fnd: FND-004
  outcome: fixed
  after_excerpt: "        #[cfg(unix)]\n        {\n            use std::os::unix::fs::DirBuilderExt;\n\
    \            let mut builder = fs::DirBuilder::new();\n            builder.mode(0o700);\n\
    \            builder.create(&root).map_err(|_| ())?;\n        }\n        #[cfg(not(unix))]\n\
    \        fs::create_dir(&root).map_err(|_| ())?;\n"
- fnd: FND-005
  outcome: fixed
  after_excerpt: '// SPDX-License-Identifier: AGPL-3.0-or-later

    // Copyright (C) 2026 Agent-IX'
```


## Disposition pass 2

Reviewed ed1f53556bc1ba069dbab299c8e61c48f19045a4 by source inspection only, as explicitly requested: no builds, tests, runtime probes or aggregate gate were run by this reviewer. FND-001 is fixed under the user-directed empty_targets contract. Request validation permits declarations only for valid transition targets, excluding the initial mode. The native parser rejects empty tokens, mixed false/nonempty disjunctions, duplicates, unauthorized empty results and unauthorized missing modes; normalization and document validation retain the same authorization. All HybridRequest constructors in source were updated. Existing empty-target oracle and fit-census fixes remain intact. No new finding or regression was identified by this source pass. FND-002 remains still-open; overall review verdict remains FAIL.

Coder-provided logs were inspected as supplementary, unrerun evidence: ver-95-round2-hybrid-tests.log records 7 passed/5 ignored; ver-95-round2-fit.log records 1 passed; ver-95-round2-f10.log records 2 passed. These are not claimed as independent reviewer executions.

Artifact custody: the committed reviews/ files are byte-identical to the original review-pass files, and are exact prefixes of the latest scratchpad artifacts. They do not yet include disposition pass 1. The lead/coder must copy the latest scratchpad artifacts, including this pass, into reviews/ in the next authorized commit. The reviewer made no branch edits. SR-17080 has no prior still-open or undisposed finding; its fixed/accepted-no-change outcomes are unchanged, so it gets no round-2 rows or comment.

### Disposition pass 2 evidence

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "        if disjuncts.is_empty() || disjuncts.iter().any(|constraint|\
    \ constraint.is_empty()) {\n            return Err(ParseError::Schema);\n    \
    \    }\n        let is_explicit_empty = disjuncts.as_slice() == [\"false\"];\n\
    \        if disjuncts.contains(&\"false\") && !is_explicit_empty {\n         \
    \   return Err(ParseError::Schema);\n        }\n        if is_explicit_empty &&\
    \ !request.empty_targets.contains(mode) {\n            return Err(ParseError::Schema);\n\
    \        }\n"
- fnd: FND-002
  outcome: still-open
  reason: 'Still open and not waived. The fix changes only src/hybrid_tool.rs; src/producer_evidence.rs
    and the shared supervisor/cleanup path are unchanged from disposition pass 1.
    Nested tool process groups can therefore survive timeout and the I/O reader can
    remain detached. Per the explicit review brief, the required cleanup belongs in
    upstream Engineering Assurance; review of specification PR #250 is not a code
    fix, merge, or consumed implementation. Keep PR #104 blocked until the upstream
    implementation is delivered and consumed and the nested-group/worker-cleanup regression
    is verified.'
```


## Disposition pass 3

Reviewed frozen head `9ea1b599a13bdf0bf0910f6e40e779d129cbe025`. Latest outcome: PASS for the reviewed code findings; FND-002 is fixed. FND-001 remains fixed from round 2, with public-adapter malformed/empty-token tests passing. FND-003 through FND-005 remain fixed. No new finding was identified. Original findings and earlier dispositions are preserved.

The only production delta since round 2 changes the outer supervisor deadline to 6,500 ms: 5,000 ms PHAVer execution + 1,000 ms upstream cleanup + 500 ms startup margin. The producer request continues to use exactly 5,000 ms, absolute installed executable selection, literal `-v000000`, the model artifact binding, and producer-execute only. Its stream/input limits and NFR-001 are unchanged. The shared invocation seam and its existing 500 ms outer fallback cleanup are unchanged; no process-tree enumeration, nested-group termination, or additional cleanup implementation was added to Quire. The startup-margin comment records the coder's 20-launch maximum of 31.6 ms; this microbenchmark was not independently repeated.

Upstream consumption evidence: GitHub reports Engineering Assurance PR #251 merged as `b77f902adedec78056fd9dac2c1bd47624d4631e`. Its production supervisor and fixture/test files are byte-equivalent to the independently reviewed `2a99ea66ae6b6dba27afb9259464b269926d2562` source. That separate review ran all three TC-125 integration tests successfully, including the observed same-session nested-group/held-pipe regression with no surviving observed child and both readers joined within the post-deadline bound. The configured installed Engineering Assurance CLI reports version 0.8.2; the retained installation log reports a successful replacement from the upstream consumer build. That temporary build worktree has since been removed, so its historical checkout could not be reinspected. This does not claim stronger adversarial or unobserved-descendant containment, nor a cryptographic installed-binary/source attestation.

Independent focused verification used the required locked-build wrapper, assigned worktree target, `RUSTUP_TOOLCHAIN=1.98.1`, and `CARGO_PROFILE_DEV_DEBUG=0`: `cargo test --lib hybrid_tool::tests -- --include-ignored --test-threads=1 --nocapture`. test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 39 filtered out; finished in 36.61s The real same-target F10 returned in 5132 ms and the distinct-target F10 in 5462 ms, both strictly before the 6,500 ms outer deadline and both exactly `Incomplete/tool_timeout` without an enclosure. Since the outer fallback cannot return before that deadline, these results exercise the upstream typed timeout path. The passing request-recording test independently asserts one request, the exact arguments, artifact role, and execution/resource budgets. The same focused run covers the fit census, oracle empty-state case, public malformed responses, tool-none behavior, schema validation and ordinary stalled-executor cleanup. A post-run process census found no Engineering Assurance or PHAVer processes remaining. No aggregate gate or branch edit was performed.

Trace verification with `quire trace --scope . --id FR-042 --prefix --format json` resolves the request-shape and both F10 tests to FR-042-AC-5/AC-7. It emits existing conditional-symbol collision diagnostics in report/solver code and an installed-module unused-import diagnostic; these were retained, not suppressed. No trace failure affects these bindings. No matrix-derived false missing-tag inference was used.

Artifact custody: before this pass, both committed SR copies were byte-identical to the scratchpad versions, including prior dispositions. Only SR-17079 needs this round's new disposition row and comment. SR-17080's FND-001 remains fixed and FND-002 through FND-012 retain their explicit accepted-no-change outcomes for unrelated baseline trace gaps. No new evidence reopens them. Its artifact is unchanged and is revalidated alongside this one. The updated SR-17079 must be copied into the branch by the authorized coding lane; the reviewer made no branch edits.

### Disposition pass 3 evidence

```yaml
dispositions:
- fnd: FND-002
  outcome: fixed
  after_excerpt: "        Duration::from_millis(\n            MAX_SOLVER_WALL_TIME_MS\n\
    \                + PRODUCER_EXECUTE_CLEANUP_MS\n                + PRODUCER_EXECUTE_STARTUP_MARGIN_MS,\n\
    \        ),\n        Duration::from_millis(PRODUCER_EXECUTE_FALLBACK_CLEANUP_MS),"
```
