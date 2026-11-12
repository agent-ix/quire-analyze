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

Verify `run-package` produces a valid report and a faithful exit status for the QSL fixture packages,
fails cleanly on bad input, and behaves identically when spawned as a bare child process.

## Test Procedure

Spawn the built binary as a child process (the producer-execute shape: explicit arguments, closed
stdin, minimal environment) over `artifacts/compiled-protocol-v1`, `artifacts/compiled-protocol-v2`
and `examples/protocol-handoff/` from `quire-spec-language`, referenced by checkout path and not
copied. Repeat each run and compare bytes. Snapshot the package tree before and after. Run with a
missing directory, an empty package, and an existing output path. Use stub solvers that agree,
disagree, are absent, stall and flood.

## Expected Results

Each fixture yields a report that validates and an exit status equal to the class of its agreed
status. Bad input exits `2` without a report; an existing destination exits `4` untouched; engine
faults exit `3` with a typed outcome. Repeated runs are byte-identical and the package tree is
unchanged.
