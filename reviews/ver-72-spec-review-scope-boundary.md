---
id: SR-17029
title: "Scope-boundary review of VER-72 producer-execute contract"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-analyze; quire-analyze#80 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md (Producer-execute invocation contract), spec/nonfunctional/NFR-003-runner-invocation.md"
review_set: subset
---

## Summary

Ticket: VER-72. Review of the `producer-execute` contract FR-009 defines. The term is not
circular, but it cites boundaries that no longer exist on quire-analyze main and imposes
obligations on Quire, Quoin and unnamed caller steps that this repository cannot verify.
Its only testable content duplicates NFR-003 and FR-009-AC-8.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-009 cites "the existing boundaries that Quire exports and never executes a producer and Quoin transcribes and never executes one" and gives `make assurance-inputs` as its example. That text, `assurance/README.md` and the `assurance-inputs` Makefile target were all removed from quire-analyze main by #46 ("Delete the shared-assurance tracking lane"), so nothing in this repo establishes the boundary. Fix: drop the citation and the example, or cite a live owner of the boundary. | FR-009:56-57, FR-009:63-65, Makefile |
| FND-002 | medium | FR-009 places obligations on external systems: "Quire and Quoin shall not spawn the runner", and "Any caller step that reports 'invoked through producer-execute' shall use exactly the arguments...". Both are outside this system's boundary and cannot be verified here. Fix: state only the runner's own interface (arguments, exit classes, no stdin, network or environment selection) and delete the obligations on Quire, Quoin and callers. | FR-009:63-68, FR-009-AC-9 |
| FND-003 | medium | The named term adds no behaviour beyond NFR-003 and FR-009-AC-8. The "reports 'invoked through producer-execute'" clause is a label claim, and it is ambiguous whether exit `1` (a conclusive refutation) is passing. It reads "conclusive results" alongside "`2`, `3`, `4` as non-passing". Fix: delete the `producer-execute` section and term, keep the hermetic one-shot behaviour in NFR-003, and leave pass/fail policy for exit `1` to the caller. | FR-009:52-68 |

## Verdict

FAIL as written. The underlying intent is consistent with FR-005: the runner is spawned by
a caller and Quire and Quoin only consume the published report. That intent is better
stated as the runner's own interface, with no new term.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in 9e07114 |
| FND-002 | fixed | verified in 9e07114 |
| FND-003 | fixed | verified in 9e07114 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | 'The earlier boundary that Quire and Quoin do not execute a producer' refers to text that #46 deleted from this repository. Nothing live states it. Fix: delete the sentence. | spec/functional/FR-009-package-runner.md Producer-execute |

## Correction (disposition pass 2)

Correction to FND-003 (review pass): `producer-execute` is an existing command, specified by engineering-assurance FR-029 and FR-019 (src/producer_execution.rs, src/producer_execute_host.rs). The finding was right that the PR's self-defined term had no grounded content, but its proposed fix (delete the term and leave pass/fail to the caller) was wrong. The correct fix is the one 720b33c makes: cite the engineering-assurance contract by reference. The round-1 'fixed' outcome stands; no new row is added.
