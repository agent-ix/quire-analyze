---
id: interface-001
title: "Contract analysis API"
type: interface
---
# [interface-001] Contract analysis API

## Contract

```yaml
name: ContractAnalysis
version: draft-analysis-v1
input:
  package: validated package at pinned quire-contract-ir revision and schema digest
  request: closed analysis kind, ordered assumption/left/right/candidate groups, execution point, encoding profile, limits
analysis_kinds: [consistency, contradiction, implication, redundancy, dead-antecedent]
operations:
  - name: prepare
    output: AnalysisModel | DiagnosticSet
    semantics: validates identity, types, shared variables, definedness, and bounds before lowering
  - name: lower
    output: QueryBundle | Unsupported
    semantics: canonical exact SMT-LIB2 plus assertion and symbol maps; approximation is non-conclusive
  - name: execute
    output: SolverRecord
    semantics: bounded argv-based process execution with exact engine/configuration identity
  - name: conclude
    output: AnalysisReport
    semantics: analysis-kind-specific sat/unsat classification and checked source mapping
  - name: compare
    output: DifferentialReport
    semantics: retains both engine records; disagreement is non-conclusive until human-reviewed adjudication
  - name: cli_analyze
    output: deterministic JSON report, stable exit class, and stderr diagnostics
    semantics: equivalent to library operation and atomically published
outcome:
  conclusive: [satisfied, refuted]
  non_conclusive: [unknown, unsupported, timed-out, cancelled, tool-unavailable, tool-error, invalid-input, internal-error]
  rule: only a complete recognized sat/unsat response under an exact encoding can become conclusive
exit_classes:
  0: satisfied
  1: refuted
  2: invalid-input-or-unsupported
  3: unknown-or-timeout-or-cancelled
  4: tool-or-internal-error
identity_envelope:
  schema: quire.derivation-evidence/v1
  required: [producer, inputs, backend, outputs, parameters, dependencies, environment, provenance, result]
resource_limits:
  lowering: [statement_count, expression_depth, expression_nodes, query_bytes]
  adapter: [wall_time_ms, cleanup_time_ms, graceful_cleanup_ms, monitor_interval_ms, stdin_bytes, stdout_bytes, stderr_bytes, model_bytes, version_bytes, executable_bytes, path_bytes]
lowering_v1:
  request_kind: boolean_conjunction
  logic: QF_UF
  identities: [binding_set_digest, analysis_request_digest, query_digest]
  maps: [named_assertion_to_clause_and_span, variable_to_complete_origins_and_binding]
  unsupported: [arithmetic, quantification, non_boolean_data, calls, accessors]
compatibility:
  unknown_schema_major: reject
  exact_source_revision: required for development
  source_tag_and_checksum: required only for a human-selected release candidate
  publication: disabled until the human PGM-02 Wave 4 decision
```

## Invariants

The raw query and response are retained before normalization. An adapter cannot directly construct a
conclusive report; only the checked conclusion layer can do so. Missing identities invalidate the
record. The library API never derives an executable path from untrusted query content.
