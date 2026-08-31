---
id: CAC-001
title: "Contract analysis component assurance contract"
type: ComponentAssuranceContract
status: proposed
owner: analyze-maintainers
kind: deterministic
responsibility: produce bounded source-mapped consistency and implication evidence without false conclusions
inputs: [validated contract package, analysis request, encoding profile, solver configuration]
outputs: [analysis report, diagnostics, derivation evidence]
invariants: [exact encoding for conclusions, complete identity, bounded execution, explicit non-conclusive states]
failure_behaviors: [abstain from a conclusion, retain the failure, reap the process tree, publish no partial report]
version_pins:
  rust-msrv: "1.75"
  governance: agent-ix/quire-contract-ir@7dac9d8c19952412b56a0347387666e2ca81e01d
  ir-wave1: agent-ix/quire-contract-ir@bb5d30cbb1519b7ac286250114c96ba967661cba
controls:
  surfaces: [library API, CLI, semantic model, SMT lowering, solver adapters, evidence verifier]
  fallback: return a typed non-conclusive outcome and retain available raw evidence
  abstention: no approximation unknown timeout cancellation failure or discrepancy becomes conclusive
  escalation: human release owner reviews unresolved gaps dependency changes and limitations
isolation: no dependency on Quoin Quire or engineering-assurance repositories at runtime
replacement: preserve semantics outcomes identities bounds evidence and differential contracts
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: references
---
# Contract analysis component assurance contract

## Component Boundary

The component owns deterministic analysis derivation and evidence. It does not own the IR semantics,
external solver correctness, platform qualification, consuming-project approval, or release decision.

## Required Behavior

One shared semantic model feeds exact lowering. Complete identities govern variable sharing and
source maps. Only checked sat/unsat responses become conclusions. All resource and evidence
boundaries fail closed.

## Failure Handling

Every invalid, unsupported, unknown, timed-out, cancelled, unavailable, malformed, truncated,
contradictory, discrepant, incomplete, or internal state is explicit and non-conclusive.

## Controls

Exact dependency and engine pins, requirement-tagged model/property/fault/differential tests,
bounded argv-only process execution, immutable raw records, schema and mutation verification,
reproducibility measurement, code review, gap analysis, and human decision constrain the component.

## Claims and Controls

| Claim | Principal hazard | Control | Evidence |
|---|---|---|---|
| CAC-001-C1 Semantic fidelity | Encoding changes contract meaning | Closed capability table, independent finite evaluator, cross-engine corpus | TC-001, TC-003, TC-006 |
| CAC-001-C2 Identity integrity | Clauses or variables alias or drift | Complete canonical identities, injective symbols, mutation tests | TC-002, TC-004, TC-007 |
| CAC-001-C3 Result truthfulness | Failure is reported as proof | Closed outcome enum, conclusion-layer gate, protocol and fault injection | TC-005, TC-007 |
| CAC-001-C4 Resource containment | Engine hangs, floods, or survives cancellation | Finite bounds and process-tree cleanup | TC-005 |
| CAC-001-C5 Counterexample validity | A fabricated or mistranslated model misleads review | Typed decode plus independent re-evaluation | TC-006 |
| CAC-001-C6 Evidence integrity | Missing or altered bytes escape the record | Schemas, complete artifact census, digests, contradiction probes | TC-007 |
| CAC-001-C7 Authority containment | Automation makes a release/qualification claim | Open AA top claim and human-only Wave 4 decision | Inspection |

## Stop Conditions

Any false conclusive classification, semantic differential without disposition, live process after
cleanup, identity collision, unbounded owned path, evidence mutation that passes, missing exact pin,
or unresolved blocking review finding prevents implementation-epic completion.

## Replacement

A replacement must consume the same versioned request, preserve every conclusion and failure state,
pass the same model, capability, fault, differential, replay, and evidence corpora, and receive a new
human decision.
