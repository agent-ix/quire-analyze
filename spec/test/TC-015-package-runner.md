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

Spawn the built binary as a child process. Provide `quire.contract-package/v1` fixtures, none of them
committed (FR-009-AC-27): documents built at test time through the `quire-contract-*` APIs (a `quire.checked-package/v2` document built in test code and read with `CheckedPackageV2::read`, lowered with `CheckedPackageV2::lower` and taken as canonical bytes: consistent,
contradictory, no analysable clause, non-Boolean claim, untranslatable claim), the malformed and
oversized documents made by byte-level mutation of a built one, and one produced from QSL: run `quire-spec-language`'s `checked_handoff::write` over the
`examples/checked-handoff/` units and model (referenced by checkout path, not copied), admit the written
checked package, lower it with `CheckedPackageV2::lower` (requested keys: every claim node key of the package;
profile: `CompleteLoweringProfileV2` with authority `quire-analyze`, identity `run-package-test`, no
required source selections, `supported_tags` `CheckedNodeTag::ALL`, `require_bounds` true and
`work_limit` `u64::MAX`), and use the canonical bytes of the resulting `CompleteContractPackageV2`.
Because QSL emits no `analysis_claim` today, this package has no analysable clause: the runner must
exit `2` on it. The exit `0`/`1` cases use the built packages. Use a
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
13. Repeat case 2 with null stdin and an empty environment, and compare with a run whose stdin is an open pipe holding unread data and whose environment is full (reports compared with `elapsedMs` set to `0`; no network restriction is applied).
14. With the real executable and real solvers on a stall and a flood stub solver: exit `3`.
15. Real executable and real solvers on a built contract package: exit `0` or `1`; skipped with a stated reason
    when absent.

16. One stub result per row of the FR-009 result-state table: engine outcome, disposition, exit `3`.
17. Symlinked component in each solver path, the `producer-execute` path and the `--output` parent: exit `2`.
18. Package with a Boolean `analysis_claim` the translation cannot lower: exit `2` naming
    `semantic_graph.nodes[N]` (FR-009-AC-19).
19. Translation, as a unit test of the translation function over built packages, claiming only what a
    unit test shows: clause and requirement identity and anchor (AC-20), each operator row (AC-21),
    parameter declarations and shared symbols (AC-22), spans (AC-24), and determinism over two runs
    (AC-26). For the refusals the unit test shows that the translation returns a refusal naming
    `semantic_graph.nodes[N]` (AC-23: each construct, the cycle, the depth of 128 against 129, the node
    count of 10,000 against 10,001, the total count over claims sharing one tree and the 61-node
    shared-subterm chain, which returns within the time of a normal run; AC-24: a node with empty
    occurrences). The `integer_range`-typed parameter as a claim body is refused as a non-Boolean claim
    naming the claim node; an integer-typed parameter operand is made by byte-level mutation of a built
    package.
20. The exit status `2` and the absence of a report for the refusals of case 19 (AC-23, AC-24), by
    spawning the binary over the same built packages, including the 61-node shared-subterm chain, which
    exits `2` without hanging.
21. Inspect the tests directory: no committed contract-package document (AC-27).
22. A stub that leaves an unreadable entry in the capability root: the root path is named on stderr, the report is published and the exit status is unchanged (AC-28).

Inspect `run-package --help` for the arguments and exit classes.

## Expected Results

Every case produces the stated exit status. Reports validate; refused inputs leave no report; repeated
runs are byte-identical apart from `elapsedMs`; the package directory is unchanged; the hermetic run equals the normal run.
