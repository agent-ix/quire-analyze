---
id: FR-003
title: "Run bounded Z3 and cvc5 process adapters"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface-001
    type: implements
---
# FR-003: Run bounded Z3 and cvc5 process adapters

## Description

The analyzer shall invoke exact identified Z3 or cvc5 executables through bounded process adapters
and normalize their protocol responses without shell interpretation.

## Behavior

- Arguments are passed as an argv vector; query bytes use a bounded stdin channel.
- Z3 and cvc5 executable paths are independently configured and may refer to air-gapped installs;
  query content cannot select or modify a path.
- No Z3 or cvc5 library is linked; engines remain external processes and therefore do not become
  transitive libraries in a consumer binary.
- Wall time, output bytes, diagnostic bytes, model bytes, and process cleanup are bounded.
- Timeout or cancellation terminates the process tree and remains `timed-out` or `cancelled`.
- Missing executables, spawn errors, malformed output, nonzero exit, signals, and solver `unknown`
  remain distinct non-conclusive outcomes.
- A conclusive answer requires a recognized `sat` or `unsat` response, successful exit, complete
  protocol parse, exact engine/version/executable/configuration identities, and no truncated output.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-003-AC-1 | Timeout and cancellation reap the process tree within the declared cleanup bound. | Test (TC-005) |
| FR-003-AC-2 | Malformed, excessive, contradictory, signaled, and nonzero-exit responses cannot become conclusive. | Test (TC-005) |
| FR-003-AC-3 | Z3 and cvc5 adapters expose the same normalized result contract. | Test (TC-006) |
| FR-003-AC-4 | Evidence identifies exact engine, version, executable digest, argv, and configuration digest. | Test (TC-007) |
| FR-003-AC-5 | Independently configured absolute solver paths work without network or shell resolution. | Test (TC-005) |
| FR-003-AC-6 | Dependency and binary inspection finds no linked Z3 or cvc5 library. | Inspection |

## Dependencies

FR-002 and native issue #3.
