---
id: MP-001
title: "Contract analysis measurement plan"
type: MeasurementPlan
status: proposed
owner: analyze-maintainers
metric: analysis_semantic_fault_differential_and_evidence_conformance
definition_version: quire-analyze.measurement-v1
stage: gate
ground_truth_kind: mechanical
statistical_design:
  population: the 12 declared TC procedures, covering public IR constructs, analysis kinds, failure states, engines, and supported platforms
  minimum_population: 12
  sampling: each declared TC procedure once per repetition, with its canonical fixtures and seeded semantic and fault variations
  repetitions: 3
  estimator: count
  error_model: encoding engine process platform configuration identity and fixture differences
  uncertainty: retain unavailable skipped inconclusive unsupported unknown and differential states
  decision_rule:
    comparator: eq
    threshold: 0
negative_controls:
  - kind: suppressed-observation
    description: Reconcile every declared fixture and repetition with retained per-run outcomes before counting adverse results.
  - kind: selective-reporting
    description: Retain all three repetitions and all Z3 and cvc5 outcomes, including unavailable and inconclusive results.
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: measures
---
# Contract analysis measurement plan

## Decision Use

Measurements inform a later human source-release decision for one candidate. They do not
approve release or confer solver qualification, validation, accreditation, or certification.

## Population

The population includes every public IR construct and boundary, consistency and implication kind,
exact and unsupported capability, solver/protocol/failure state, output artifact, mutation class,
and declared supported platform. At this definition, `spec/test/` declares 12 TC procedures:
TC-001 through TC-010, TC-013, and TC-014. Each repetition must record one outcome for each
of those 12 procedures, including unavailable or inconclusive outcomes. The collection must
state `population.examined` of at least 12 and `population.repetitions` of at least 3; a
shorter collection is refused before the zero-count rule is evaluated. The human gate
review also reconciles retained TC IDs against the declared set; the numeric floor alone
cannot detect a duplicate substituted for an omitted TC.

## Collection Procedure

Fake executables exercise
process faults; independent finite enumeration and counterexample replay exercise semantics;
Z3 and cvc5 runs exercise differential behavior; mutation probes exercise evidence integrity.

## Mechanical Decision Rule

For each declared fixture and each of the three repetitions, count every semantic-oracle
discrepancy, false conclusive classification, surviving process tree or cleanup overrun,
byte or identity-digest difference for identical material inputs, undisposed engine
discrepancy, invalid counterexample replay, and accepted evidence mutation. The count is
zero only when none of these adverse outcomes occurs. Retain each individual outcome and
the declared fixture census. Any nonzero count is escalated. A zero count with an omitted
fixture or repetition, an unavailable supported lane, or an inconclusive outcome is
incomplete evidence and cannot support a clean claim. M-01, M-03, M-09, and M-10 remain
separate review evidence for the human source-release decision.

## Measures

| ID | Measure | Required result | Owner |
|---|---|---|---|
| M-01 | Acceptance-criterion census | Every completed matrix row maps to executed named tests; zero unbacked completions | analyzer maintainer |
| M-02 | Algebra oracle | Zero discrepancies against independent finite enumeration | semantic implementer |
| M-03 | Capability census | Every public IR construct is exact-supported or explicitly unsupported | lowering implementer |
| M-04 | Reproducibility | Zero byte differences for identical material inputs | evidence owner |
| M-05 | Adapter fault campaign | Zero false conclusions and zero surviving process trees | adapter implementer |
| M-06 | Differential corpus | Zero undisposed Z3/cvc5 discrepancies | analysis implementer |
| M-07 | Counterexample replay | Every published model re-evaluates to its claimed truth condition | analysis implementer |
| M-08 | Evidence mutations | Every required-field, digest, census, and outcome contradiction is rejected | evidence owner |
| M-09 | Local quality gates | fmt, Clippy warnings-as-errors, tests, docs, licenses, unsafe audit, MSRV pass | maintainer |
| M-10 | Platform campaign | Linux, macOS, and Windows status retained; absent lanes remain limitations | release owner |

## Adapter v1 Quantitative Profile

| Quantity | Ceiling | Collection and decision rule |
|---|---:|---|
| solver wall time | 5,000 ms | monotonic spawn-to-terminal duration; timeout is non-conclusive |
| total process-group cleanup | 1,000 ms | three timeout plus three cancellation repetitions; retain all values and maximum; any survivor or overrun fails M-05 |
| graceful termination interval | 100 ms | monotonic interval before forced termination; included in total cleanup |
| monitor interval | 5 ms | code/config census; a larger interval is invalid configuration |
| query stdin | 16,777,216 bytes | exact boundary and one-byte-over rejection |
| captured stdout | 16,777,216 bytes | drain after ceiling, mark truncated, never conclusive |
| captured stderr | 1,048,576 bytes | drain after ceiling, mark truncated, never conclusive |
| parsed model | 8,388,608 bytes | exact boundary and one-byte-over response fixture |
| version output | 65,536 bytes | bounded identity probe; excess is tool error |
| executable identity input | 536,870,912 bytes | metadata check before fixed-buffer SHA-256 streaming |
| canonical executable path | 4,096 UTF-8 bytes | reject before spawn if absent, relative, non-UTF-8, or excessive |

These are v1 profile ceilings, not performance estimates. TC-005 measures cleanup against the
declared value on every supported platform run. Later release evidence retains platform-specific
observations and cannot infer an unexecuted platform result.

## Interpretation

Passing scaffold tests is not semantic coverage. Process exit zero is insufficient where structured
output reports unknown, skip, contradiction, or unavailable. Coverage percentage is supplementary;
requirements coverage and fault/differential outcomes determine claim support.
