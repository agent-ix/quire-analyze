---
id: interface_001
title: "Contract analysis API"
type: interface
---
# [interface_001] Contract analysis API

## Contract

```yaml
name: ContractAnalysis
version: current-library-v1
status: proposed acceptance candidate under IR-1294; callable library stages and publication-only CLI
input:
  package: typed statements and clause identities from quire-contract-ir
  request: closed analysis kind, kind-specific ordered statement groups and explicit bindings; statements carry execution points
  profiles_and_limits: lowering uses fixed Boolean-v1 profiles and ceilings; execution receives caller-supplied SolverConfig
analysis_kinds: [consistency, contradiction, implication, redundancy, dead-antecedent]
operations:
  - name: prepare
    entry_point: src/analysis.rs AnalysisRequest constructors
    current: constructors check required groups and duplicate statement identities; remaining validation occurs during lowering
    output: "Result<AnalysisRequest, Vec<AnalysisRequestError>>"
    semantics: constructors validate required groups and duplicate identities; lowering performs remaining checks before solver execution
  - name: lower
    entry_point: src/analysis.rs lower_analysis_request; src/smt.rs lower_boolean_statements
    current: implemented library Boolean-v1 lowering
    output: "Result<QueryBundle, Vec<LoweringError>>"
    semantics: canonical exact SMT-LIB2 plus assertion and symbol maps; approximation is non-conclusive
  - name: execute
    entry_point: src/solver.rs execute_solver
    current: implemented library execution with caller-supplied query and solver configuration
    output: SolverRecord
    semantics: bounded argv-based process execution with exact engine/configuration identity
  - name: conclude
    entry_point: src/analysis.rs classify_analysis
    current: classifies the logical solver outcome; failed SAT replay retains that status and records Incomplete explanation without a verified model
    output: AnalysisConclusion
    semantics: FR-004 preserves logical status while requiring verified evidence before publishing a mapped model or counterexample
  - name: compare
    entry_point: src/report.rs compare_solver_records
    current: implemented library comparison; PGM-01 envelope unavailable
    output: DifferentialRun
    semantics: retains both engine records; disagreement is non-conclusive until human-reviewed adjudication
  - name: cli_publish
    entry_point: src/main.rs publish-report; src/report.rs validate_report_document and publish_report_new
    current: sole implemented CLI command; consumes existing library report bytes
    input: authoritative canonical report rendered by the library
    output: byte-identical report file, stable exit class, and stderr diagnostics
    semantics: validates the closed report contract and atomically publishes without reconstructing trusted inputs
outcome:
  conclusive: [satisfied, refuted]
  non_conclusive_library_status: [unknown, unsupported, timeout, tool-error]
  detail: SolverRecord retains original outcomes; cancellation maps to timeout, unsupported-platform to unsupported, other failures to tool-error
  rejected_requests: constructor and lowering errors return Result errors before an AnalysisConclusion exists
  rule: complete recognized sat/unsat under exact encoding determines logical status; SAT replay determines whether model evidence is verified or incomplete
  differential_agreement: FR-005 requires both matching conclusive statuses and every required SAT model replay-verified
exit_classes:
  0: published differential agreement on satisfied
  1: published differential agreement on refuted
  2: usage or input-read or report-validation error
  3: published report with any non-agreement differential disposition
  4: publication or exit-code conversion error
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
  source_release: requires the human PGM-02 Wave 4 decision
```

## Features

| Feature | Kind |
|---|---|
| Boolean-v1 analysis stages | Library API |
| Differential evidence and existing-report publication | Library API and publish-only CLI |

## Invariants

The raw query and response are retained before normalization. An adapter cannot directly construct a
conclusion; `classify_analysis` owns its logical status and its verified or incomplete explanation.
A conclusive logical status follows only a complete recognized sat/unsat response under the exact
encoding profile.
FR-004 keeps that logical status when optional model evidence fails validation, without representing
the evidence as verified. `compare_solver_records` requires verified SAT evidence for same-status
differential `Agreement` under FR-005.
Missing identities invalidate the record. The library API never derives an executable path from
untrusted query content. The listed library entry points are separate calls; no CLI command
currently composes prepare through compare.
Hybrid reachability and canonical synthesis are separate providers in `src/hybrid_synthesis.rs`,
returning method outcomes through `crates/method-contract`; they are outside this Boolean SMT
classification contract. The PGM-01 shared envelope remains unavailable pending
`quire-contract-ir#20`, so a local placeholder cannot establish full evidence compliance.

Cargo.toml and Cargo.lock declare and resolve the `quire-contract-ir` dependency. The runtime
report uses `quire.differential-report/v2`, a closed contract without `contractIrRevision`. It
retains producer application name/version and canonical clause, request, query, raw-stream, and
report content digests. The standalone validator accepts v2 only; the bound validator also
reconstructs identities from the authoritative query and differential run. No v1 fallback is
provided.
Neither acceptance of this interface nor automated output grants engine qualification or human
release approval. The acceptance scope and decision dispositions are recorded in AD-001. This
interface remains an acceptance candidate until the independent IR-1294 review is resolved; the
accepted ADR-0010 alone does not accept it.
