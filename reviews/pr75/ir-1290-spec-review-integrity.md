---
id: SR-17020
title: "Integrity analysis of PR #75"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; IR-1290 PR #75; spec/assurance/MP-001-analyze-measurements.md, spec/functional/FR-003-bounded-adapters.md, spec/functional/FR-005-evidence-cli.md, spec/nonfunctional/NFR-001-determinism-bounds.md, spec/test/TC-005-adapter-faults.md, spec/test/TC-007-evidence-integrity.md; context spec/nonfunctional/NFR-002-evidence-authority.md"
review_set: subset
---
# SR-17020: Integrity analysis of PR #75

## Summary

The new TC-007 procedure and its expected results conflict, and the unchanged constraining NFR-002 has semantics incompatible with the new FR-005 report contract.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Reconcile NFR-002 AC-1/AC-3 and Verification with FR-005 report fields and TC-007 | spec/nonfunctional/NFR-002-evidence-authority.md:25,27,34 |
| FND-002 | high | TC-007 gives incompatible expected results for changed raw solver bytes | spec/test/TC-007-evidence-integrity.md:31,44 |

## Scope

Examined: FR-003 Description, Behavior and AC-4; FR-005 Behavior and AC-6; NFR-001 boundary table and AC-2; TC-005 procedure; TC-007 procedure and expected results; MP-001 quantitative profile. Also examined: NFR-002 AC-1, AC-3 and Verification. Context: AP-001 review policy.

## Dispositions

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | NFR-002 AC-1 and AC-3 now name retained proof-input identities and the authoritative-run comparison; Verification no longer promises configuration/output digest checks. |
| FND-002 | fixed | TC-007 now separates a raw-stream contradiction (refuse) from a self-consistent raw-stream substitution (standalone may admit; authoritative-run comparison refuses). |
