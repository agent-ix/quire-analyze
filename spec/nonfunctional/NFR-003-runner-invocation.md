---
id: NFR-003
title: "Package runner is bounded, hermetic and one-shot"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-009
    type: constrains
---
# NFR-003: Package runner is bounded, hermetic and one-shot

## Statement

The `run-package` runner shall complete each solver run within the time and output limits of
[NFR-001](./NFR-001-determinism-bounds.md) and shall derive its report only from its arguments, the
package file and the named solver executables.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Solver runs that exceed a configured time or output limit without a typed non-conclusive outcome | 0 | 0 | fault tests with a stalled and a flooding solver |
| Hidden inputs (stdin, network, undeclared environment) affecting the report or exit status | 0 | 0 | hermetic-run test |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-003-AC-1 | A solver that stalls past its wall-time limit or floods past its output limit yields a published report with a typed non-conclusive outcome and exit `3`, never a hang and never exit `0` or `1`. | Test |
| NFR-003-AC-2 | A run with closed stdin, no network namespace access and an environment limited to what process spawning needs yields the same report bytes and exit status as a run with open stdin and a full environment. | Test |
| NFR-003-AC-3 | The runner reads no standard input and consults no environment variable to select a solver, a limit or an output; inspection of `run-package` finds no such read. | Inspection |

## Verification

TC-015 verifies NFR-003-AC-1 and NFR-003-AC-2; NFR-003-AC-3 is verified by inspection.

## Dependencies

[FR-009](../functional/FR-009-package-runner.md) and [NFR-001](./NFR-001-determinism-bounds.md).
