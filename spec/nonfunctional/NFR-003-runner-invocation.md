---
id: NFR-003
title: "Package runner is a bounded, hermetic, one-shot producer"
type: NFR
quality_attribute: reliability
relationships:
  - target: ix://agent-ix/quire-analyze/FR-009
    type: constrains
---
# NFR-003: Package runner is a bounded, hermetic, one-shot producer

## Statement

The `run-package` runner shall terminate within its configured bounds on every package. The runner
shall depend only on its arguments, the package directory and the configured solver executables.

## Measurement and Evaluation

| Metric | Target | Threshold | Method |
|---|---|---|---|
| Runs exceeding configured solver time or output limits without a typed non-conclusive outcome | 0 | 0 | fault tests with a stalled and a flooding solver |
| Runs whose exit status disagrees with the report's agreed status | 0 | 0 | fixture tests |
| Hidden inputs (stdin, network, undeclared environment) affecting the report | 0 | 0 | hermetic-run test |

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| NFR-003-AC-1 | A solver that stalls past its time limit or floods past its output limit yields a published report with a typed non-conclusive outcome and exit `3`, never a hang or exit `0`/`1`. | Test |
| NFR-003-AC-2 | For every fixture package the exit status equals the class derived from the published report (satisfied `0`, refuted `1`, non-conclusive `3`). | Test |
| NFR-003-AC-3 | Changing stdin, network availability or unrelated environment variables does not change the report bytes. | Test |
| NFR-003-AC-4 | Runner output claims no qualification, accreditation or release approval, per [NFR-002](./NFR-002-evidence-authority.md). | Inspection |

## Verification

TC-015 verifies all criteria except NFR-003-AC-4, which is verified by inspection.

## Dependencies

[FR-009](../functional/FR-009-package-runner.md), [NFR-001](./NFR-001-determinism-bounds.md) and [NFR-002](./NFR-002-evidence-authority.md).
