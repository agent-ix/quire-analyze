---
id: AD-001
title: "Contract analysis architecture"
type: ArchitectureDescription
status: proposed
owner: analyze-maintainers
system: quire-analyze v0.1
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: realizes
---
# Contract analysis architecture

## System Boundary

The Rust library consumes typed expressions and clause identities from `quire-contract-ir`. It owns
closed-request validation, Boolean-v1 lowering, bounded solver execution, checked conclusions,
source mapping, and differential report rendering. The solver engines and operating-system process
behavior remain external. The CLI currently validates and publishes an existing library report;
it does not run the analysis pipeline. Human reviewers retain release and consuming-project
qualification authority. This description is proposed; it does not accept AD-001 or qualify an
engine.

## Views

The implemented Boolean-v1 derivation path is typed contract IR → closed `AnalysisRequest` →
canonical `QueryBundle` with assertion and variable maps → bounded Z3/cvc5 child process →
`SolverRecord` → `classify_analysis` with checked model and source mapping → differential report
bytes. Only the conclusion layer can promote a recognized, exact solver result to a conclusive
analysis status. The report layer retains both records and makes disagreement non-conclusive.

| Boundary | Owner and current entry point | Responsibility |
|---|---|---|
| Typed contract input | `quire-contract-ir` dependency; `src/smt.rs` `StatementInput`, `BindingGroup` | Consume authoritative expressions, clause identity, and explicit cross-requirement bindings; refuse unsupported shapes. |
| Request and exact lowering | `src/analysis.rs` `AnalysisRequest`, `lower_analysis_request`; `src/smt.rs` `lower_boolean_statements` | Validate closed analysis kinds and produce the Boolean-v1 SMT-LIB2 query plus source maps. |
| Bounded engine execution | `src/solver.rs` `execute_solver` | Run the external engine under resource bounds and retain a normalized record; never certify its answer. |
| Checked conclusion | `src/analysis.rs` `classify_analysis` | Apply analysis-kind-specific sat/unsat rules and replay-check mapped Boolean models before a conclusive status. |
| Differential evidence and publication | `src/report.rs` `execute_differential`, `render_differential_report`, `validate_report_document`, `publish_report_new`; `src/main.rs` `publish-report` | Compare and retain both engine records, render and validate report bytes, and atomically publish a pre-existing report through the CLI. |
| Separate method providers | `src/hybrid_synthesis.rs` `reach`, `synthesize`, `validate_candidate`; `crates/method-contract` `AnalyzeOutcome` | Return bounded hybrid reachability and synthesis outcomes through their own method-result seam; these do not become SMT proof classifications. |

The proof identity view binds selected clause content, explicit bindings, query bytes and maps,
engine execution record, checked conclusion, and canonical report content. These content digests
bind a proof to what it analyzed. Cargo.toml declares `quire-contract-ir` and Cargo.lock records
its resolved source; they are the dependency identity authorities. The current implementation also
duplicates that revision as `CONTRACT_IR_REVISION` in `src/smt.rs`, validates the same literal in
`src/report.rs`, and text-scans manifests and lockfile in `tests/smt_lowering.rs`. That repetition
is a code follow-up, not an additional architecture authority. Report producer identity is the
application name and version.

The failure view routes every invalid, unsupported, unknown, timed-out, cancelled, unavailable,
malformed, truncated, contradictory, undecodable, discrepant, or internal state to a non-conclusive
variant. Adapters cannot construct conclusions directly.

The evidence view is incomplete: `src/report.rs` explicitly marks its PGM-01 envelope
`unavailable` pending `quire-contract-ir#20`. FR-005-AC-2 and TC-007 cannot be claimed complete
from that placeholder. Source publication and release qualification remain a human PGM-02 Wave 4
decision.

## Decisions

- One engine-neutral Boolean query bundle prevents adapters from reinterpreting clauses.
- Complete semantic identity, not display name, controls variable sharing and source mapping.
- Canonical SMT-LIB2 and explicit assertion names make derivation reviewable.
- External engines run as bounded child processes with argv, never a shell command.
- Retained raw records are inputs to normalization and differential comparison.
- Exact encoding is required for conclusive v0.1 results; approximation is fail-closed.
- Hybrid reachability and synthesis have separate bounded providers and method outcomes; their
  results do not imply a Boolean SMT proof.

## Interfaces and Trust

Solver answers are evidence, not self-authenticating truth: finite-model and cross-engine campaigns
reduce risk but do not qualify engines. The OS process API, filesystem atomicity, content-digest
implementation, and schema validator remain named external dependencies. The interface's
prepare/lower/execute/conclude/compare sequence describes callable library steps; the current CLI
exposes only `publish-report`.

## Risks

Encoding drift, solver defects, platform process differences, unbounded output, identity collisions,
model-decoding errors, and unavailable cross-platform execution can weaken evidence. The measurement
plan retains these outcomes and no automated control may waive them into success. AD-001 acceptance,
the upstream PGM-01 envelope, engine qualification, and source-release authority remain open
decisions; this proposed description does not resolve them.
