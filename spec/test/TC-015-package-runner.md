---
id: TC-015
title: "Verify the package runner"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-009
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-003
    type: verifies
---
# TC-015: Verify the package runner

## Description

Verify `run-package` runs solvers only through `producer-execute`, writes a valid report and a faithful
exit status, fails cleanly on every refused input, and behaves identically as a bare child process.

## Test Procedure

Spawn the built binary as a child process. Provide `quire.contract-package/v1` fixtures: documents kept
under this repository's `tests/fixtures/` (consistent, contradictory, no analysable clause, malformed,
oversized) and one produced at test time: run `quire-spec-language`'s `checked_handoff::write` over the
`examples/checked-handoff/` units and model (referenced by checkout path, not copied), admit the written
checked package, lower it with `CheckedPackageV2::lower` (requested keys: every claim node key of the package;
profile: `CompleteLoweringProfileV2` with authority `quire-analyze`, identity `run-package-test`, no
required source selections, `supported_tags` `CheckedNodeTag::ALL`, `require_bounds` true and
`work_limit` `u64::MAX`), and use the canonical bytes of the resulting `CompleteContractPackageV2`.
Because QSL emits no `analysis_claim` today, this package has no analysable clause: the runner must
exit `2` on it. The exit `0`/`1` cases use contract packages kept in `tests/fixtures/`. Use a
stub `producer-execute` script that logs each request and returns canned
`producer-execution-result/v1` lines, and for the final case the real `engineering-assurance` executable
with real or stub solvers. Cases:

1. Request log: two requests per engine (version, query), valid request protocol, absolute solver path,
   empty environment, built-in response binding; no other process start (FR-009-AC-1).
2. Both `sat` with a replaying model: exit `0`, report validates, `satisfied`.
3. Both `unsat`: exit `1`, `refuted`.
4. Each non-completed state for one engine, and `sat` against `unsat`: exit `3`, both records retained.
5. Stub that cannot start, exits `1`, `2` or another non-zero status, dies by signal, prints no line, two lines or garbage: exit `4`, no report.
6. Missing, reader-refused, dependency-naming, no-`analysis_claim`, non-Boolean-claim package and a document one byte over 1,048,576 bytes (exit `2`, no report); a document exactly at the limit is not refused for size.
7. Unknown, repeated and missing arguments and an option without a value: exit `2`, usage on stderr.
8. Relative, empty, non-UTF-8 and over-long paths, each as `--z3`, `--cvc5` and `--producer-execute`: exit `2`.
9. Existing output, missing parent, unwritable parent: exit `4`; existing bytes unchanged.
10. Unit test of the runner function (not the spawned binary): a directory-sync failure after rename
    through the TC-008 publication seam maps to exit `4` with the durability state named.
11. Run each passing case twice and compare bytes; snapshot the package directory before and after.
12. Validate every logged request against `producer-execute --schema request` and check the budget values; check the version-then-query sequencing and the unavailable-on-bad-version rule; check the capability root is `0700` during the run and gone after each exit path.
13. Repeat case 2 with closed stdin, no network and a minimal environment, and compare with a normal run.
14. With the real executable and real solvers on a stall and a flood stub solver: exit `3`.
15. Real executable and real solvers on a `tests/fixtures/` contract package: exit `0` or `1`; skipped with a stated reason
    when absent.

16. One stub result per row of the FR-009 result-state table: engine outcome, disposition, exit `3`.
17. Symlinked component in each solver path, the `producer-execute` path and the `--output` parent: exit `2`.
18. Package with a Boolean `analysis_claim` the translation cannot lower: exit `2`.

Inspect `run-package --help` for the arguments and exit classes.

## Expected Results

Every case produces the stated exit status. Reports validate; refused inputs leave no report; repeated
runs are byte-identical; the package directory is unchanged; the hermetic run equals the normal run.
