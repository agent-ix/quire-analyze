---
id: SR-17068
title: "Code review of quantified assertion replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/analysis.rs, src/datatype_replay.rs, src/report.rs, src/smt.rs, tests/range_quantifier_lowering.rs; FR-033, NFR-031, TC-030"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-033", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-031", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-030", type: reviews }
---
# Code Review of Quantified Assertion Replay

## Summary

Ticket: VER-92. PR agent-ix/quire-analyze#102, frozen at `17b4909`. The Rust lane was reviewed
with rust-style and repository conventions; the diff changes typed quantifier replay, failure and
success reporting, and replay tests. Static review found a replay-result reporting defect and an
acceptance-criterion test gap. No aggregate build gate was run during review; the code lead owns the
final gate. No workflow files changed.

## Assurance Context

AP-001 (`spec/assurance/AP-001-analyze-release.md`) applies to this v0.1 source candidate. Its
material impact scenario is a semantic or evidence failure reported as a conclusive analysis. I
reviewed the changed typed replay and report path against AD-001's model-replay boundary and
CAC-001's exact-encoding, bounded-execution, explicit-abstention controls. AD-001's architecture
and human release boundary were available; the implemented replay flow retains a logical solver
status while marking failed replay evidence incomplete. No retained MP-001 measurement collection
or release-candidate evidence was available, and MP-001's enumerated population predates TC-030, so
it does not establish this change's conformance. AP-001 has no standing exception; no exception was
found for this change. This code review and its findings do not make a release decision.

## Verdict

**CONDITIONAL (one medium finding).** Quantifier evaluation stops correctly at the first decisive
instance and shares a checked request counter. However, distinct quantifier results can be
collapsed when they have the same accepted source identity, losing replay evidence from the
verified-model API.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Replay results are keyed by caller-supplied source text and overwritten on a match, although formula validation permits repeated non-empty identities. A model containing two distinct quantified nodes with the same identity therefore retains only the later node's result; the verified-model API promises a result for each quantified assertion, so one node's count/outcome disappears. | `src/smt.rs:1124-1131`, `src/smt.rs:3697-3750`, `src/analysis.rs:487-515` |

## Review Coverage

- Rust implementation: checked bounded/lazy range iteration, checked request-wide counting, checked arithmetic propagation, local binding restoration and shadowing, model-bound decoding classification, finite-domain typed substitution, solver/replay outcome separation, public API documentation, and report diagnostics.
- Tests: checked new trace attributes and assertions for range, Boolean collection, enum, bound decode, and solver-backed cases. The matrix command and targeted trace discrepancy are recorded in the separate gap-analysis artifact.
- Duplicate/vendor and integrity checks: no copied dependencies, repo-file pins, narrowed workflow gates, or changes to CI were found.
- Scope units examined: FR-033-AC-1 through FR-033-AC-9; NFR-031 statement and AC-1/AC-2; TC-030 replay and resource procedures; AP-001; AD-001; CAC-001; MP-001.

+++ [reviewer data]

```yaml
clean: false
scope:
  - {id: FR-033-AC-1, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "For a failing universal model, replay evaluates the domain elements in order up to the first failing element, reports that element and the source identity, and evaluates no later element; for a passing universal model it evaluates exactly as many instances as the domain has elements."}
  - {id: FR-033-AC-2, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "Replay of an existential quantifier stops at and reports its first holding instance with its element and source identity, does not evaluate later elements, and completes the assertion successfully; when none holds, it reports no holding instance and the count enumerated."}
  - {id: FR-033-AC-3, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A bound whose next instance would exceed the limit in NFR-031 yields `incomplete` with reason `replay_limit_exceeded`; that instance is not evaluated and the model is never `verified`."}
  - {id: FR-033-AC-4, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "For each of an enum domain, a finite collection and an integer range, and for each of universal and existential, one passing model and one failing model are replayed to the expected outcome."}
  - {id: FR-033-AC-5, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A solver model that satisfies the lowered query but fails one instance of the original quantifier is not published as verified."}
  - {id: FR-033-AC-6, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A declared range bound that is missing from the model, repeated, malformed or outside its declared type yields `incomplete` with reason `bound_undecodable`; a model with lower bound above upper bound replays as an empty range."}
  - {id: FR-033-AC-7, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A body whose evaluation overflows yields `incomplete` with reason `evaluation_overflow`, never a wrapped value."}
  - {id: FR-033-AC-8, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "Where Z3 and cvc5 are installed, each satisfiable quantified fixture in TC-030 is decided `sat` by both solvers with agreement, and each returned model is published as `verified` only after replay passes."}
  - {id: FR-033-AC-9, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "When replay evaluates every assertion for a returned model and each holds, the model is published as `verified`; if an assertion fails or replay is incomplete, it is not."}
  - {id: NFR-031, path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md, role: examined, excerpt: "The analyzer shall evaluate at most 1,048,576 quantifier instances per request during replay, counted in the instance unit of NFR-030, and shall report a request that needs more as `incomplete` with reason `replay_limit_exceeded`. An over-limit replay is never reported as `verified`."}
  - {id: NFR-031-AC-1, path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md, role: examined, excerpt: "A request needing exactly 1,048,576 instances replays fully; one needing 1,048,577 yields `incomplete` with reason `replay_limit_exceeded`, has evaluated no more than 1,048,576 instances, and is never `verified`."}
  - {id: NFR-031-AC-2, path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md, role: examined, excerpt: "A 2^64-wide range and nested ranges whose instance-count product overflows yield `incomplete` with reason `replay_limit_exceeded`, without wrapping or panicking."}
  - {id: TC-030-replay, path: spec/test/TC-030-bounded-quantifiers.md, role: examined, excerpt: "Replay: one passing and one failing model per domain form and quantifier form; ... test bounds exactly at and one above the replay limit, and a model that satisfies the lowered query but fails one original instance."}
  - {id: TC-030-resources, path: spec/test/TC-030-bounded-quantifiers.md, role: examined, excerpt: "Resources: instance budget and replay limit at and one above their values, a 2^64-wide range, the nested width-100 fixture, a stalled solver and a solver answering `unknown` on a quantified query."}
  - {id: AP-001, path: spec/assurance/AP-001-analyze-release.md, role: context_only, excerpt: "A semantic adapter or evidence failure is reported as a conclusive analysis."}
  - {id: AD-001, path: spec/assurance/AD-001-analyze-architecture.md, role: context_only, excerpt: "On replay error it retains that logical status and records `Incomplete` with a diagnostic, without representing a model or source value as verified."}
  - {id: CAC-001, path: spec/assurance/CAC-001-analyze-controls.md, role: context_only, excerpt: "All resource and evidence boundaries fail closed."}
  - {id: MP-001, path: spec/assurance/MP-001-analyze-measurements.md, role: context_only, excerpt: "Every completed matrix row maps to executed named tests; zero unbacked completions."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: code-review
    check: code-bug
    artifact_id: FR-033-AC-1
    related_ids: [FR-033-AC-2, FR-033-AC-9]
    path: src/smt.rs
    lines: "1124-1131"
    excerpt: |-
      if let Some(previous) = context
          .quantifier_results
          .iter_mut()
          .find(|previous| previous.source == source)
      {
          *previous = result;
      } else {
          context.quantifier_results.push(result);
      }
    finding: "Formula validation only rejects an empty quantifier identity, so two distinct nodes may legally share a source string. This code then replaces the earlier node's result instead of retaining one result per quantified assertion. The public VerifiedBooleanModel documentation promises results for each quantified assertion, but its source, selected element, count, and outcome become incomplete for such a valid request."
bindings: []
```

+++

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Replay results now use a stable per-node identity instead of source text as the map key. Lowering assigns a distinct node ID to each quantifier, and a regression test verifies that two valid nodes sharing one source identity both remain in the verified model's results. |
