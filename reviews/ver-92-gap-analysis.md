---
id: SR-17069
title: "Gap analysis of VER-92 quantified replay"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; spec/functional/FR-033-quantified-assertion-replay.md, spec/nonfunctional/NFR-031-quantifier-replay-bound.md, spec/test/TC-030-bounded-quantifiers.md, src/analysis.rs, src/smt.rs, src/datatype_replay.rs, src/report.rs, tests/range_quantifier_lowering.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-033", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-031", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-030", type: reviews }
---
# Gap Analysis of VER-92 Quantified Replay

## Summary

Ticket: VER-92. This planless audit checked the FR-033/NFR-031 requirement-to-test-to-code chain and
reverse coverage for the five changed code/test files. Plan completion was not assessed. The
replay-limit behavior is implemented, but its public classification behavior and several required
quantifier/domain test combinations are not asserted end to end.

## Verdict

**CONDITIONAL (two medium test gaps).** Every changed path maps to FR-033/NFR-031 and no new code
behavior without an owning requirement was found. Matrix output was not usable for coverage: installed
`quire 0.44.0 (engine 0.59.1)` `quire matrix --scope . --format json` exited 0 but labeled every
criterion untagged, including criteria with valid Rust `#[trace(...)]` test attributes. The same
installed `quire trace --scope . --id FR-033-AC-1 --json` resolved four such test claims. Per the
project's known PLAT-1672 limitation, targeted traces and test-body inspection were used for this
scope; the all-untagged matrix result is not treated as an implementation gap. Optional semantic
review was skipped; no opt-in was supplied.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-033-AC-3 and NFR-031-AC-1/AC-2 are tested only through the private evaluator's `ReplayFault::QuantifierLimitExceeded` and internal counter. No test drives an over-limit `QueryBundle` through `classify_analysis` to assert `AnalysisStatus` remains logical, explanation becomes incomplete, `ReplayFailureReason::ReplayLimitExceeded`/`replay_limit_exceeded` is exposed, and `verified_model()` is absent. A caller-level mapping regression could therefore pass the new boundary tests. | `src/smt.rs:4998-5136`, `src/analysis.rs:744-760,928-956`, `spec/functional/FR-033-quantified-assertion-replay.md:67` |
| FND-002 | medium | FR-033-AC-4 requires passing and failing models for both quantifier kinds over each domain. The enum-domain and finite-collection test loops cover only a failing universal and a passing existential; they omit a passing universal and a failing existential for each. The integer-range cases cover both outcomes, but do not fill the enum and collection combinations. | `tests/range_quantifier_lowering.rs:1431-1535`, `spec/functional/FR-033-quantified-assertion-replay.md:68` |

## Coverage

- Reconciliation: `quire matrix` returned all criteria as untagged (exit 0), contradicted by targeted `quire trace` results; PLAT-1672 limitation, so matrix is not used as authoritative for this scope. Targeted `quire trace` reports Rust test claims for FR-033-AC-1; raw trace attributes were also inspected for the remaining new cases.
- Plan completion: not assessed
- Criteria: targeted trace/test inspection found direct claims for FR-033-AC-1 through AC-9 and NFR-031-AC-1/AC-2; completeness gaps are FND-001/002. No run evidence read.
- Untraced behaviors / stubs: 0 new reverse gaps or stubs in changed paths.
- Semantic review: skipped (not opted in).

+++

## Reviewer Data

```yaml
clean: false
scope:
  - {id: FR-033-AC-1, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "For a failing universal model, replay evaluates the domain elements in order up to the first failing element, reports that element and the source identity, and evaluates no later element; for a passing universal model it evaluates exactly as many instances as the domain has elements."}
  - {id: FR-033-AC-2, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "Replay of an existential quantifier stops at and reports its first holding instance with its element and source identity, does not evaluate later elements, and completes the assertion successfully; when none holds, it reports no holding instance and the count enumerated."}
  - {id: FR-033-AC-3, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A bound whose next instance would exceed the limit in NFR-031 yields `incomplete` with reason `replay_limit_exceeded`; that instance is not evaluated and the model is never `verified`."}
  - {id: FR-033-AC-4, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "For each of an enum domain, a finite collection and an integer range, and for each of universal and existential, one passing model and one failing model are replayed to the expected outcome."}
  - {id: FR-033-AC-5, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A solver model that satisfies the lowered query but fails one instance of the original quantifier is not published as verified."}
  - {id: FR-033-AC-6, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A declared range bound that is missing, repeated, malformed or outside its declared type yields `incomplete` with reason `bound_undecodable`; a model with lower bound above upper bound replays as an empty range."}
  - {id: FR-033-AC-7, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "A body whose evaluation overflows yields `incomplete` with reason `evaluation_overflow`, never a wrapped value."}
  - {id: FR-033-AC-8, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "Where Z3 and cvc5 are installed, each satisfiable quantified fixture in TC-030 is decided `sat` by both solvers with agreement, and each returned model is published as `verified` only after replay passes."}
  - {id: FR-033-AC-9, path: spec/functional/FR-033-quantified-assertion-replay.md, role: examined, excerpt: "When replay evaluates every assertion for a returned model and each holds, the model is published as `verified`; if an assertion fails or replay is incomplete, it is not."}
  - {id: NFR-031-AC-1, path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md, role: examined, excerpt: "A request needing exactly 1,048,576 instances replays fully; one needing 1,048,577 yields `incomplete` with reason `replay_limit_exceeded`, has evaluated no more than 1,048,576 instances, and is never `verified`."}
  - {id: NFR-031-AC-2, path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md, role: examined, excerpt: "A 2^64-wide range and nested ranges whose instance-count product overflows yield `incomplete` with reason `replay_limit_exceeded`, without wrapping or panicking."}
  - {id: TC-030-replay, path: spec/test/TC-030-bounded-quantifiers.md, role: examined, excerpt: "Replay: one passing and one failing model per domain form and quantifier form; ... test bounds exactly at and one above the replay limit, and a model that satisfies the lowered query but fails one original instance."}
  - {id: TC-030-resources, path: spec/test/TC-030-bounded-quantifiers.md, role: examined, excerpt: "Resources: instance budget and replay limit at and one above their values, a 2^64-wide range, the nested width-100 fixture, a stalled solver and a solver answering `unknown` on a quantified query."}
  - {id: FR-033, path: spec/functional/FR-033-quantified-assertion-replay.md, role: context_only, excerpt: "The analyzer shall check a solver model against every bounded quantified assertion by enumerating the quantifier's bound."}
  - {id: NFR-031, path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md, role: context_only, excerpt: "The analyzer shall evaluate at most 1,048,576 quantifier instances per request during replay."}
findings:
  - fnd: FND-001
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-033-AC-3
    related_ids: [NFR-031-AC-1, NFR-031-AC-2]
    path: tests/range_quantifier_lowering.rs
    lines: "4998-5136 (src/smt.rs tests)"
    excerpt: |-
      A bound whose next instance would exceed the limit in NFR-031 yields `incomplete` with reason `replay_limit_exceeded`; that instance is not evaluated and the model is never `verified`.
    finding: "The new limit test exercises only the private ReplayExpression evaluator and observes ReplayFault::QuantifierLimitExceeded. No caller-level test proves that classify_analysis preserves the solver status, marks explanation incomplete, exposes the public ReplayFailureReason::ReplayLimitExceeded stable code, and publishes no verified model on overflow. The NFR-031 boundary tests likewise do not verify the public conclusion contract."
  - fnd: FND-002
    severity: medium
    confidence: high
    method: gap-analysis
    check: coverage
    artifact_id: FR-033-AC-4
    related_ids: [TC-030]
    path: tests/range_quantifier_lowering.rs
    lines: "1431-1535"
    excerpt: |-
      For each of an enum domain, a finite collection and an integer range, and for each of universal and existential, one passing model and one failing model are replayed to the expected outcome.
    finding: "The enum-domain and finite-collection model tests each cover only two of the four required outcomes: a failing universal and a passing existential. The passing-universal and failing-existential cases are absent for both domains, so the claimed domain-by-quantifier coverage is incomplete."
bindings: []
```

+++

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | A new classify_analysis regression test sends an over-limit query through the public classifier and checks the preserved logical status, incomplete explanation, stable replay_limit_exceeded reason, and absence of a verified model. |
| FND-002 | fixed | New TC-030 cases assert a passing universal and failing existential for both the enum and finite-collection domains, completing the previously missing combinations. |
