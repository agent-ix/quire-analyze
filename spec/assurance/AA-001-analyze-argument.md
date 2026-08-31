---
id: AA-001
title: "Contract analysis assurance argument"
type: AssuranceArgument
status: proposed
owner: human-release-owner
profile: ix://agent-ix/quire-analyze/AP-001
top_claim:
  id: claim-analyze-v01
  statement: the identified analysis source candidate and dependency set are acceptable for bounded v0.1 use
  subject: quire-analyze v0.1 source candidate
  status: open
reasoning:
  - id: reasoning-analysis-conformance
    statement: evaluate semantic fidelity bounded execution counterexample replay differential agreement and evidence integrity
    supports: claim-analyze-v01
    sufficiency_criteria:
      - every native issue and required gate is complete
      - exact IR and solver pins are reconciled
      - no blocking specification implementation code-review or gap-review finding remains
assumptions:
  - id: assumption-consumer-validation
    statement: consuming projects validate the pinned analyzer engines and outputs for their intended use
    owner: human-release-owner
    status: open
    review_by: "2026-12-31T00:00:00Z"
participants:
  - id: human-release-owner
    role: decision owner
    authority: accept reject or defer the bounded source candidate
    independence: reviews agent-assisted implementation pins limitations and evidence
challenges:
  - id: challenge-unimplemented-semantics
    target: claim-analyze-v01
    statement: the foundation is specified but semantic implementation and retained campaigns do not yet exist
    status: open
    owner: human-release-owner
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: references
---
# Contract analysis assurance argument

## Claim

The exact `quire-analyze` v0.1 source candidate is suitable for its declared analysis/evidence-tool
use with all limitations understood and accepted by the named human release authority.

**Status: open.** No source candidate, tag, or human sufficiency decision exists in this foundation.

## Reasoning

Independent model checks, exact capability coverage, deterministic lowering, hostile-process fault
injection, differential solvers, counterexample replay, schema mutations, local quality gates, and
human review jointly address the known failure scenarios. No single solver answer or manifest is sufficient.

## Sufficiency Decision

No automated sufficiency decision is recorded. The human release owner reviews the exact source and
dependency candidate, retained measurements, open limitations, assumptions, and challenges in Wave 4.

## Subclaims

| ID | Claim | Required support | Current status |
|---|---|---|---|
| AA-001-C1 | The analysis algebra matches authoritative contract semantics. | M-01 through M-03 | open |
| AA-001-C2 | Lowering is deterministic, exact, and source traceable. | M-03, M-04 | open |
| AA-001-C3 | Solver execution is bounded and failure preserving. | M-05 | open |
| AA-001-C4 | Conclusions and counterexamples are independently checked. | M-02, M-06, M-07 | open |
| AA-001-C5 | Evidence is complete, truthful, and immutable. | M-08, M-09 | open |
| AA-001-C6 | Residual platform, engine, and qualification limits are disclosed. | M-10 and human review | open |

## Current Foundation Claim

The foundation defines verifiable requirements, architecture, controls, measurements, tests, and a
dependency-guarded plan. It makes no semantic implementation claim. All test-matrix rows remain
planned and all subclaims remain open until native issues and independent review complete.

## Challenges

Semantic implementation, solver pins, retained campaigns, independent review, cross-platform
evidence, and the human decision are absent. Each remains explicit and open.
