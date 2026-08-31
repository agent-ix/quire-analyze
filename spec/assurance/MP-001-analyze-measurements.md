---
id: MP-001
title: "Contract analysis measurement plan"
type: MeasurementPlan
status: proposed
owner: analyze-maintainers
metric: analysis_semantic_fault_differential_and_evidence_conformance
definition_version: quire-analyze.measurement-v1
stage: gate
statistical_design:
  population: every public IR construct analysis kind failure state engine and supported platform
  sampling: exhaustive canonical fixtures plus seeded generated semantic and fault variations
  repetitions: 3
  estimator: exact discrepancies digest differences classifications cleanup and evidence mutation outcomes
  error_model: encoding engine process platform configuration identity and fixture differences
  uncertainty: retain unavailable skipped inconclusive unsupported unknown and differential states
  decision_rule: escalate any semantic discrepancy false conclusion leaked process digest drift or evidence mutation pass
relationships:
  - target: ix://agent-ix/quire-analyze/AP-001
    type: measures
---
# Contract analysis measurement plan

## Decision Use

Measurements inform a later human source-release decision for one pinned candidate. They do not
approve release or confer solver qualification, validation, accreditation, or certification.

## Population

The population includes every public IR construct and boundary, consistency and implication kind,
exact and unsupported capability, solver/protocol/failure state, output artifact, mutation class,
and declared supported platform.

## Collection Procedure

Each native implementation task extends a source-bound local evidence runner. The runner executes
the requirement-tagged test census, retains stdout, stderr, numeric exit and structured outcome,
records exact pins, and derives rather than authors aggregate status. Fake executables exercise
process faults; independent finite enumeration and counterexample replay exercise semantics; pinned
Z3 and cvc5 runs exercise differential behavior; mutation probes exercise evidence integrity.

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

## Retention

Each candidate record identifies source revision and state, commands, toolchain, OS/target, exact IR
and solver pins, schema and corpus digests, feature set, environment, individual exit/status values,
test census, limitations, and SHA-256 checksums. Failed, skipped, and unavailable measures remain
visible. A rerun creates a new immutable record; it never edits history.

## Interpretation

Passing scaffold tests is not semantic coverage. Process exit zero is insufficient where structured
output reports unknown, skip, contradiction, or unavailable. Coverage percentage is supplementary;
requirements coverage and fault/differential outcomes determine claim support.
