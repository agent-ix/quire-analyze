---
id: SR-17019
title: "Base spec review of PR #75"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze; IR-1290 PR #75; spec/assurance/MP-001-analyze-measurements.md, spec/functional/FR-003-bounded-adapters.md, spec/functional/FR-005-evidence-cli.md, spec/nonfunctional/NFR-001-determinism-bounds.md, spec/test/TC-005-adapter-faults.md, spec/test/TC-007-evidence-integrity.md; context spec/nonfunctional/NFR-002-evidence-authority.md"
review_set: subset
---
# SR-17019: Base spec review of PR #75

## Summary

The edited FR, NFR, TC and MP artifacts remove solver pins and duplicate byte hashes. TC-007 contradicts itself, and NFR-002 still requires the identity and digest behavior the PR withdraws.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | TC-007 says raw solver byte mutation remains visible, then says every mutation fails closed | spec/test/TC-007-evidence-integrity.md:31,44 |
| FND-002 | high | NFR-002 still requires configuration/output identity binding and changed-byte detection after FR-005 removes those checks | spec/nonfunctional/NFR-002-evidence-authority.md:25,27,34 |

## Scope

Examined: FR-003 Description, Behavior and AC-4; FR-005 Behavior and AC-6; NFR-001 boundary table and AC-2; TC-005 procedure; TC-007 procedure and expected results; MP-001 quantitative profile. Also examined: NFR-002 AC-1, AC-3 and Verification. Context: AP-001 review policy.

## Dispositions

| FND | Outcome | Evidence |
| --- | --- | --- |
| FND-001 | fixed | TC-007 now separates a raw-stream contradiction (refuse) from a self-consistent raw-stream substitution (standalone may admit; authoritative-run comparison refuses). |
| FND-002 | fixed | NFR-002 AC-1 and AC-3 now name retained proof-input identities and the authoritative-run comparison; Verification no longer promises configuration/output digest checks. |
