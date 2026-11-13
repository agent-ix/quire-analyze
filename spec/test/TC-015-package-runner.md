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
oversized) and one produced at test time by lowering `examples/checked-handoff/model.semantic-ir.json`
of `quire-spec-language` through `quire-contract-model` (referenced by checkout path, not copied). Use a
stub `producer-execute` script that logs each request and returns canned
`producer-execution-result/v1` lines, and for the final case the real `engineering-assurance` executable
with real or stub solvers. Cases:

1. Request log: two requests per engine (version, query), valid request protocol, absolute solver path,
   empty environment, built-in response binding; no other process start (FR-009-AC-1).
2. Both `sat` with a replaying model: exit `0`, report validates, `satisfied`.
3. Both `unsat`: exit `1`, `refuted`.
4. Each non-completed state for one engine, and `sat` against `unsat`: exit `3`, both records retained.
5. Stub that cannot start, exits `2`, or prints garbage: exit `4`, no report.
6. Missing, reader-refused, no-clause and oversized package: exit `2`, no report.
7. Unknown, repeated and missing arguments and an option without a value: exit `2`, usage on stderr.
8. Relative, empty, non-UTF-8 and over-long solver paths and a relative `producer-execute` path: exit `2`.
9. Existing output, missing parent, unwritable parent: exit `4`; existing bytes unchanged.
10. Unit test of the runner function (not the spawned binary): a directory-sync failure after rename
    through the TC-008 publication seam maps to exit `4` with the durability state named.
11. Run each passing case twice and compare bytes; snapshot the package directory before and after.
12. Repeat case 2 with closed stdin, no network and a minimal environment, and compare with a normal run.
13. With the real executable and real solvers on a stall and a flood stub solver: exit `3`.
14. Real executable and real solvers on a lowered fixture: exit `0` or `1`; skipped with a stated reason
    when absent.

Inspect `run-package --help` for the arguments and exit classes.

## Expected Results

Every case produces the stated exit status. Reports validate; refused inputs leave no report; repeated
runs are byte-identical; the package directory is unchanged; the hermetic run equals the normal run.
