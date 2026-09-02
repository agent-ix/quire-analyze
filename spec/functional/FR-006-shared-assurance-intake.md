---
id: FR-006
title: "Adopt the shared assurance evidence contract"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: depends_on
  - target: ix://agent-ix/quire-analyze/NFR-002
    type: depends_on
---
# FR-006: Adopt the shared assurance evidence contract

## Description

Analysis results shall be produced by this repository's own tools and transcribed
into the official Engineering Assurance, Quire and Quoin contracts, without a
local generic runner, evidence envelope, manifest, identity framework, retention
store, audit store or aggregate verdict, and without either shared tool executing
a producer.

## Behavior

- Component versions are classified by the packaged Engineering Assurance
  compatibility matrix. This repository observes what is installed and delegates
  every verdict to `engineering_assurance.compatibility`; it does not restate the
  matrix locally, because a second copy is a second authority that can drift from
  the one the acceptance decision was made against.
- `compatible`, `incompatible` and `unknown` remain three answers. A version the
  matrix has never seen, and a component that could not be observed at all, are
  both `unknown`, and `unknown` never satisfies a gate.
- Human acceptance is reported separately from version compatibility and is never
  synthesized here. The pinned `engineering-assurance` v0.2.0 release records
  `pending_human_acceptance` and ships no `human_acceptance_recorded` predicate;
  the acceptance itself is recorded on that repository's `main` at `ae50e13`.
  This repository reports the state the pinned release records and gates only on
  version compatibility. Tracked as `agent-ix/engineering-assurance#20`.
- `make assurance-inputs` is the only target that executes a producer. The
  driver, Quire and Quoin all consume its output and refuse to create it. An
  absent input is an error naming that target, never a step the driver performs
  for itself.
- Every proof attestation states the verdict read out of the bytes its producer
  wrote. A result is never assumed, defaulted, or taken from a caller's
  expectation, and a producer whose output cannot be read stops the run rather
  than being recorded as passed.
- The producer outcome vocabulary is closed. `passed`, `failed`, `malformed`,
  `unavailable`, `not_computed`, `vacuous` and `inconclusive` each map to a named
  attestation result; an outcome the table does not name is refused rather than
  defaulted.
- Where a stream carries several outcomes, the strongest observed one is
  reported: a single failure outranks any number of passes, and an unavailable
  outranks a not-computed.
- The pinned Z3 and cvc5 release assets are located by declared environment
  variables and checked against their pinned executable digests. When they are
  absent the real-engine differential corpus is `unavailable`: it did not run, so
  nothing about it was decided, and that is neither a pass nor a failure.
- This repository retains no evidence tree, manifest or digest oracle of its own.
  The records it once retained were deleted under the preservation constraint
  `agent-ix/engineering-assurance#7` released for the pre-stable phase; they are
  deleted rather than rewritten, and no claim that they still verify anything is
  made here in weaker form.
- Every negative case names the positive control it is paired with, and a control
  naming a case that did not run is refused rather than passing vacuously.
- The published crate gains no runtime dependency on Quire, Quoin or
  engineering-assurance.
- Hosted CI remains `workflow_dispatch` only and is not dispatched by this
  change.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-006-AC-1 | Adopted component versions are classified by the packaged compatibility matrix rather than by a local restatement, and an unobservable component is `unknown`. | Test (TC-011) |
| FR-006-AC-2 | Solver execution, analysis classification and differential comparison are produced by this repository's tools in a declared structured format and transcribed by Quoin without Quoin executing the producer. | Test (TC-011) |
| FR-006-AC-3 | Static specification, obligation and coverage facts come from a Quire export, and Quire executes no producer. | Test (TC-011) |
| FR-006-AC-5 | Every non-conclusive solver state stays distinct from a conclusion and from every other non-conclusive state across the intake path. | Test (TC-011) |
| FR-006-AC-6 | No repository-local generic runner, envelope, manifest, identity framework, retention store, audit store or aggregate verdict remains in the execution path. | Test (TC-012) |

## Verification

TC-011 verifies the intake path, producer isolation, result derivation and the
closed outcome vocabulary. TC-012 verifies the absence of repository-local
generic assurance machinery.

## Dependencies

FR-005, NFR-002, and the accepted compatibility matrix published by
`agent-ix/engineering-assurance#8`.
