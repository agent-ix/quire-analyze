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

Verify `run-package` writes a valid report and a faithful exit status for executable-projection
fixtures, fails cleanly on every refused input, and behaves identically when spawned as a bare child
process.

## Test Procedure

Spawn the built binary as a child process. Use executable-projection fixtures kept under this
repository's `tests/fixtures/` (one consistent, one contradictory, one with no executable clause, one
malformed, one oversized) and stub solver scripts that answer `sat` with a replaying model for the consistent fixture, answer `unsat`, are absent, stall
or flood. Cases:

1. Consistent fixture with `sat`/`sat` stubs emitting replaying models: exit `0`, report validates, agreed status `satisfied`.
2. Contradictory fixture with `unsat`/`unsat` stubs: exit `1`, agreed status `refuted`.
3. `sat`/`unsat`, one absent, one stalled and one flooding solver: exit `3`, both engine records retained.
4. Missing package file, decoder-refused file, file over 16,777,216 bytes, no-executable-clause fixture: exit `2`, no report.
5. Unknown, repeated and missing arguments and an option without a value: exit `2`, usage on stderr.
6. Relative, empty, non-UTF-8 and over-long solver paths: exit `2`, no report. An absolute path naming no executable: exit `3`.
7. Existing output, missing parent, unwritable parent: exit `4`; existing bytes unchanged.
8. Unit test of the runner function (not the spawned binary): a directory-sync failure after rename through the TC-008 publication seam maps to exit `4` with the durability state named.
9. Run each passing case twice and compare bytes; snapshot the package directory before and after.
10. Repeat case 1 with closed stdin, no network and a minimal environment, and compare with a normal run.

Inspect `run-package --help` for the arguments and exit classes.

## Expected Results

Every case produces the stated exit status. Reports validate; refused inputs leave no report; repeated
runs are byte-identical; the package directory is unchanged; the hermetic run equals the normal run.
