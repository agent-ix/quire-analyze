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

StR-001 is refined by FR-001 through FR-005, FR-007 and FR-008 and constrained by NFR-001 and NFR-002.
`interface-001` defines the request, response, outcome, diagnostic, and evidence boundary. TC-001
through TC-010, TC-013 and TC-014 form the verification matrix. FR-007 and FR-008 own the bounded hybrid
reachability and canonical synthesis providers. AP-001, AD-001, CAC-001, MP-001, and AA-001 define the
assurance boundary. PLAN-001 maps this foundation and native issues #6, #7, #3, #4, #5, and #33.

## References

- [Program umbrella](https://github.com/agent-ix/quire-contract-ir/issues/1).
- [Contract-IR expression gate](https://github.com/agent-ix/quire-contract-ir/issues/8).
- [Contract-IR schema and corpus gate](https://github.com/agent-ix/quire-contract-ir/issues/10).
- [Contract analysis epic](https://github.com/agent-ix/quire-analyze/issues/8).
