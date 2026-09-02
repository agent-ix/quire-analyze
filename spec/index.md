---
type: master-requirements
name: quire-analyze
org: agent-ix
component_type: rust-library
implementation_language: rust
tags: [contract-analysis, smt, z3, cvc5, assurance]
depends_on:
  - ix://agent-ix/quire-contract-ir/PGM-01
  - ix://agent-ix/quire-contract-ir/issues/8
  - ix://agent-ix/quire-contract-ir/issues/10
standards_alignment: [iso-iec-ieee-29148]
relationships:
  - target: ix://agent-ix/quire-contract-ir/PGM-01
    type: depends_on
    cardinality: "1:1"
  - target: ix://agent-ix/quire-contract-ir/issues/10
    type: depends_on
    cardinality: "1:1"
---
# Master Requirements Specification

## Purpose

This specification defines deterministic, bounded SMT-backed consistency and implication analysis
for authoritative contract IR. It preserves exact query, solver, configuration, result, and source
identities so a reviewer can reproduce a conclusion without treating the analyzer as a release or
qualification authority.

## Scope

### In Scope

- A closed analysis algebra for consistency, implication, and counterexample requests.
- Deterministic SMT-LIB2 lowering with explicit capability and approximation contracts.
- Bounded Z3 and cvc5 process adapters with typed non-conclusive outcomes.
- Source-mapped conclusions, counterexamples, differential checks, library API, CLI, and evidence.

### Out of Scope

- Parsing or redefining the authoritative IR, implementing an SMT solver, or proving an engine sound.
- Unbounded execution, silent approximation, automatic release approval, certification, or accreditation.
- Source tags and publication; those remain a human PGM-02 Wave 4 decision.

## System Overview

The crate validates a pinned contract-analysis request, derives one engine-neutral semantic model,
lowers exact canonical SMT-LIB2, invokes bounded external solver adapters, checks conclusions and
counterexamples, and emits source-mapped derivation evidence. External engines remain untrusted,
versioned dependencies and a named human retains release authority.

## Requirements Architecture

StR-001 is refined by FR-001 through FR-006 and constrained by NFR-001 and NFR-002.
`interface-001` defines the request, response, outcome, diagnostic, and evidence boundary. TC-001
through TC-012 form the verification matrix. FR-006 adopts the shared Engineering Assurance, Quire
and Quoin contracts and owns no local evidence machinery. AP-001, AD-001, CAC-001, MP-001, and AA-001 define the
assurance boundary. PLAN-001 maps this foundation and native issues #6, #7, #3, #4, and #5.

## Authoritative Inputs

- PGM-01: `ix://agent-ix/quire-contract-ir/PGM-01`, merged revision
  `7dac9d8c19952412b56a0347387666e2ca81e01d` and inherited without local weakening.
- Accepted contract-IR Wave 1 head: `bb5d30cbb1519b7ac286250114c96ba967661cba`.
- Implemented schema/corpus merge: `5c49ebfd1c87415f74420ad047392bd03b1bd202`.
- Package schema SHA-256: `748d98def7c0a67e3e12f882cd9ef7d0948c8eacbff1e5f6135faa7fd29d642d`.
- Conformance schema SHA-256: `63fe642ebe7e7f49acf59094a8edaa488b96b13806886f0af2779629900bdb75`.
- Corpus manifest SHA-256: `aed86fa6fd5e88412b3a771b594011884ef6df1e8256827ccf87bc9ae53fced4`.

These are development pins, not source-release tags. A semantic implementation must reconcile its
dependency declaration and lockfile to an accepted exact source revision before merge.

## References

- [Program umbrella](https://github.com/agent-ix/quire-contract-ir/issues/1).
- [Contract-IR expression gate](https://github.com/agent-ix/quire-contract-ir/issues/8).
- [Contract-IR schema and corpus gate](https://github.com/agent-ix/quire-contract-ir/issues/10).
- [Contract analysis epic](https://github.com/agent-ix/quire-analyze/issues/8).
