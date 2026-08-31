---
id: AP-001
title: "Contract analysis v0.1 decision profile"
type: AssuranceProfile
status: proposed
owner: human-release-owner
profile_version: 0.2
profile_kind: general
scope: one identified quire-analyze v0.1 source candidate and pinned dependency and solver set
impact_assessments:
  - id: impact-false-conclusion
    scenario: a semantic adapter or evidence failure is reported as a conclusive analysis
    severity: material
    verifiability:
      class: cheap-conclusive
      stochastic_dependency: none
    detect_before_harm:
      expected: true
      control_ref: ix://agent-ix/quire-analyze/CAC-001
review_policy:
  mode: require
  operations: [code-review, gap-analysis]
relationships:
  - target: ix://agent-ix/quire-contract-ir/PGM-01
    type: references
---
# Contract analysis v0.1 decision profile

## Decision Boundary

The decision boundary is one source revision, pinned IR/schema/corpus, encoding, solver executables,
configuration, toolchain, and platform profile. The profile supplies evidence and confers no
consuming-project qualification, accreditation, certification, or release authority.

## Intended Use

`quire-analyze` is an analysis/evidence tool that assists reviewers by producing bounded,
reproducible consistency and implication evidence for one pinned contract package. It does not
replace source review, solver qualification, system verification, or a human release decision.

## Boundary and Dependencies

The owned boundary validates an analysis request, constructs the semantic model, lowers exact
SMT-LIB2, invokes a bounded adapter, classifies the response, checks source mapping, and emits
evidence. Contract IR and schemas, Z3, cvc5, the operating system, Rust toolchain, Quire validator,
and the human decision are external and exactly identified.

## Assurance Activities

1. Validate requirements, interface, matrix, reviews, and plan with Quire.
2. Reconcile exact contract-IR, schema, corpus, toolchain, solver, and configuration pins.
3. Require independent finite-model truth checks for the algebra and counterexamples.
4. Exercise exact/unsupported capability partitions for every public IR construct.
5. Fault-inject process, protocol, resource, evidence, and publication boundaries.
6. Differentially execute the supported corpus on pinned Z3 and cvc5.
7. Retain source-bound results, failures, skipped lanes, limitations, and checksums.
8. Perform code review, requirements-test review, gap analysis, and correction before merge.

## Failure States

Invalid input, unsupported semantics, unknown response, timeout, cancellation, tool absence, process
failure, malformed or excessive output, contradictory protocol, undecodable model, differential
disagreement, incomplete evidence, and internal error are explicit. Only satisfied and refuted are
conclusive, and neither is a source-release or project-qualification decision.

## Human Owner

`@kreneskyp` owns v0.1 source-release sufficiency under PGM-01. Automation may prepare candidates and
evidence but leaves that claim open for PGM-02 Wave 4.

## Impact Scenarios

Material scenarios include semantic drift, identity aliasing, silent approximation, false
conclusions after process/protocol failure, unreaped solver processes, invalid counterexamples,
differential disagreement hidden as agreement, and incomplete or altered evidence.

## Evidence Policy

Every material identity and individual outcome is retained. Failed, unavailable, unsupported,
unknown, timed-out, cancelled, discrepant, skipped, and inconclusive states remain visible.

## Exceptions

No standing exception exists. Any exception requires scope, rationale, affected requirements,
expiry, evidence effect, and explicit human acceptance under PGM-01.
