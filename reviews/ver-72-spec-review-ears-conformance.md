---
id: SR-17030
title: "EARS conformance review of VER-72 FR-009 behavior and NFR-003"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-analyze; quire-analyze#80 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md (Behavior, Producer-execute), spec/nonfunctional/NFR-003-runner-invocation.md (Statement)"
review_set: subset
---

## Summary

Ticket: VER-72. EARS review of the FR-009 Behavior bullets, the producer-execute bullets
and the NFR-003 statement. quire flags the NFR-003 statement (`ears:unclassifiable`,
`ears:missing-subject`, line 15). A manual pass found compound and subject-less statements
in FR-009.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The NFR-003 statement is two sentences and fails EARS classification. "Terminate within its configured bounds on every package" names a bound that does not exist: NFR-001 limits are per solver run, and there is no configured whole-run bound. Fix: write one ubiquitous statement, for example "The run-package runner shall complete every solver run within the time and output limits of NFR-001", and move the input-dependence sentence to its own statement or delete it as a duplicate of FR-009. | NFR-003:14-15 |
| FND-002 | medium | The FR-009 Behavior bullet "Two runs over identical package content and solver configuration shall produce byte-identical reports" has no system subject. Fix: "The runner shall produce byte-identical report bytes for identical package content and solver configuration." | FR-009:47 |
| FND-003 | low | The first FR-009 Behavior bullet is compound (load, derive, run, render, and "shall not hand-build"), and the exit bullet combines the `0` and `1` conditions in one statement. Fix: split them into one statement per response. | FR-009:39-44 |
| FND-004 | low | The producer-execute bullets "The party that executes is always..." and "The runner is a non-interactive, one-shot process" are descriptive, with no `shall`. "Any developer-owned file" in the Behavior bullets is undefined here. Fix: use `shall` statements and cite FR-005's definition of developer-owned. | FR-009:48, FR-009:58-62 |

## Verdict

FAIL on FND-001 and FND-002, both of which are quick rewordings. The non-conclusive bullet
(an unwanted-behaviour `If ... shall`) and the load-failure bullet (an event `When ...
shall`) conform.
