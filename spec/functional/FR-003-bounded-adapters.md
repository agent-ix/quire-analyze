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
- The `quire.solver-process/v1` default profile has exact finite ceilings: 5,000 ms wall time,
  1,000 ms total termination/cleanup time (including a 100 ms graceful interval), 16,777,216 query
  bytes, 16,777,216 stdout bytes, 1,048,576 stderr bytes, 8,388,608 model bytes within stdout,
  65,536 version-output bytes, a 536,870,912-byte executable identity input, and a 4,096-byte
  canonical executable path. The monitor interval is at most 5 ms. Callers may select smaller
  positive limits; they cannot exceed these profile ceilings.
- Timeout or cancellation terminates the process tree and remains `timed-out` or `cancelled`.
- Missing executables, spawn errors, malformed output, nonzero exit, signals, and solver `unknown`
  remain distinct non-conclusive outcomes.
- A conclusive answer requires a recognized `sat` or `unsat` response, successful exit, complete
  protocol parse, exact engine/version/executable/configuration identities, and no truncated output.
- v0.1 process-tree containment uses a fresh POSIX process group and executes the already-hashed
  open file descriptor on Linux targets.
  Other targets return `unsupported-platform` before spawn; a missing platform campaign cannot be
  represented as successful cleanup.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-003-AC-1 | In each of three timeout and cancellation repetitions on Linux, zero members of the spawned POSIX process group survive 1,000 ms after cleanup begins; the measured maximum is retained. | Test (TC-005) |
| FR-003-AC-2 | Malformed, excessive, contradictory, signaled, and nonzero-exit responses cannot become conclusive. | Test (TC-005) |
| FR-003-AC-3 | Z3 and cvc5 adapters expose the same normalized result contract. | Test (TC-006) |
| FR-003-AC-4 | The record identifies exact engine, full normalized version output, executable SHA-256, argv, query digest, limits, configuration digest, exit state, stdout/stderr, and elapsed/cleanup milliseconds. | Test (TC-005, TC-007) |
| FR-003-AC-5 | Independently configured absolute solver paths work without network or shell resolution. | Test (TC-005) |
| FR-003-AC-6 | Dependency and binary inspection finds no linked Z3 or cvc5 library. | Inspection |

## Dependencies

FR-002 and native issue #3.
