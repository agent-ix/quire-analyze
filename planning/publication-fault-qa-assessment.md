---
id: REV-024
title: "Atomic report publication quality assurance assessment"
type: Review
---
# Atomic report publication quality assurance assessment

## Measurement

The complete local gate executes 50 tests with zero failures and one separately controlled pinned
real-engine test ignored by default. LLVM line coverage is 91.49% overall and 91.62% for
`src/report.rs`, above the enforced 90% project floor. The default and Rust 1.75 suites both execute
the fault state machine, five abrupt-termination child probes, and the production concurrency path.

## Covered Risk Classes

- invalid destination and exclusive-create refusal;
- deterministic partial write, file sync, no-replace rename, parent-directory sync, and cleanup
  failures with exact public stage/state classification;
- existing developer-owned destination preservation and no partial destination publication;
- successful complete-byte file and directory durability path;
- eight simultaneous production publishers with one winner;
- unknown-owner stale staging preservation;
- abrupt subprocess termination after every durable publication boundary;
- CLI/library byte parity and stable exit classes through the production publisher.

## Residual Limitation

An uncatchable pre-rename termination leaves one private named staging file. The test asserts that
the destination is absent and the residue is never classified as a report. Automatic stale cleanup
would require a reviewed interprocess ownership protocol; prefix scanning alone could delete a live
publisher's data. This limitation is the remaining #23 review item.

## QA Verdict

The fault campaign is sufficient for the recoverable Linux named-temp/rename boundary and exposes
the true crash state. It is not evidence that a dead process removed its own staging entry. Full
local gate counts and coverage are retained with the source-bound validation record.
