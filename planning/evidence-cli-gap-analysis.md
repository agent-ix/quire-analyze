---
id: REV-020
title: "Differential evidence and CLI completion gap analysis"
type: Review
---
# Differential evidence and CLI completion gap analysis

## Acceptance Coverage

| Issue #5 acceptance area | Evidence | Status |
|---|---|---|
| Versioned machine and human-readable reports | Strict `quire.differential-report/v1`, canonical renderer, stderr summary, complete query/engine bytes and identities | closed for differential-report v1 |
| Positive and negative real-engine agreement | Explicit ignored/manual test against verified official Z3 5.1.0 and cvc5 1.3.4 assets; both SAT and UNSAT cases pass | closed for seeded Boolean SAT/UNSAT |
| Unsupported, timeout, unavailable, disagreement, and incomplete states | Requirement-tagged deterministic fake-process/lowering fixtures preserve every state and both engine records | closed for controlled fixtures |
| Same supported corpus through both real engines | SAT/UNSAT corpus runs through both; complete retained corpus/CLI campaign does not | partial; issue #24 |
| Library-first API and deterministic CLI | Library executes and renders; CLI schema-validates, byte-preserves, refuses overwrite, and returns stable 0/1/2/3/4 classes | closed for v1 publisher boundary; direct analysis CLI is issue #24 |
| Schema and evidence mutation rejection | Production Draft-07 validation plus resealed unknown-field, raw-byte, query-byte, engine-order, status, disposition, and canonical-form mutations | closed for application report v1 |
| Atomic output at every failure boundary | Success and rename no-replace are covered; write/sync/crash injection is absent | partial; issue #23 |
| Shared PGM-01 envelope and integrity validation | Explicit unavailable envelope bound to upstream dependency | blocked on `quire-contract-ir#20`; FR-005-AC-2 remains open |
| Code review, gap analysis, QA assessment, and source-bound record | REV-019, REV-020, REV-021 and validation capture | pending source-bound capture |

## Ticket Census

- #23 owns deterministic atomic publication fault/crash injection.
- #24 owns the complete retained real-engine corpus and any future trusted direct-analysis CLI.
- #22 owns generated/fuzz/mutation analysis and model campaigns.
- #21 and #20 retain solver containment stress and non-Linux containment.
- `quire-contract-ir#20` owns selection of the shared PGM-01 envelope/integrity component.

## Verdict

All currently safe, dependency-independent issue #5 implementation work is present and locally
verified. Native issue #5, Task-007, and the epic must remain open; the upstream PGM-01 dependency is
a completion blocker, while #23 and #24 accurately preserve broader QA and end-to-end scope.
