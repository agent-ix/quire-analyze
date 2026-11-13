---
id: SR-17028
title: "Integrity review of VER-72 package runner against FR-005 and the report contract"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; quire-analyze#80 diff origin/main...HEAD: spec/functional/FR-009-package-runner.md, spec/nonfunctional/NFR-003-runner-invocation.md; context spec/functional/FR-005-evidence-cli.md, src/report.rs, src/main.rs"
review_set: subset
---

## Summary

Ticket: VER-72. Consistency check of FR-009 against FR-005 (on which it depends), the
implemented report contract and the QSL package code it cites. FR-009 asks for a report
covering many requests, which the single-request report contract cannot hold, and CON-1
forbids changing that contract. FR-009 loads packages and solver configuration from files,
which FR-005 says the CLI does not do. It also does not say which package fields declare an
analysis request, and `qsl-package/src/` on main has no such concept.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-009 derives several "analysis requests" from a package and publishes "one differential report" whose exit is `0` only when "every declared request is satisfied". The rendered report holds exactly one request: one `requestDigest`, one `queryHex` and one `differential.agreedStatus`. FR-009-CON-1 forbids changing the schema, so the requirement cannot be met. The precedence between "any refuted, so `1`" and "any non-conclusive, so `3`" is also undefined. Fix: write one report per request (an output directory, or a `--request` selector), or restrict the package to exactly one declared request, and state the aggregate exit precedence. | FR-009:39-46, FR-009-CON-1, src/report.rs:216-256 |
| FND-002 | high | Loading is not grounded. FR-009 does not say which QSL artefact or API it loads, or which field declares an analysable request and its kind (consistency, contradiction, implication, redundancy or dead antecedent). `qsl-package/src/` on QSL main has no analysis-request declaration, and quire-analyze depends on `quire-contract-model`, not on `qsl-package`. FR-005 also states "The v1 CLI ... does not reconstruct authoritative contract packages or solver configurations from JSON", which `--package DIR` and `--solver-config FILE` contradict. Fix: name the loader and the declaring field, or the derivation rule per `AnalysisKind`, and amend FR-005's CLI sentence in this PR to scope it to `publish-report`. | FR-009:39-41, FR-005:59-61, Cargo.toml:30 |
| FND-003 | medium | FR-009-CON-1 points at "the differential report contract named by FR-005", but FR-005 names both `quire.analysis-report/v1` and `quire.differential-report/v2`, and speaks of the "v1 library" and "v1 CLI". The only schema rendered and validated today is `quire.differential-report/v2` (`DIFFERENTIAL_REPORT_SCHEMA`, `schemas/differential-report-v2.schema.json`), and v1 documents are rejected. The PR body's "report schema v2 ... out of scope" does not match the code. Fix: have CON-1 name `quire.differential-report/v2` as the emitted contract, by reference and without restating it. | FR-009-CON-1, FR-005:41-55, src/report.rs:16 |

## Verdict

FAIL: FND-001 and FND-002 are contradictions that a coder cannot resolve without a spec
decision. The exit classes (`0` satisfied, `1` refuted, `2` usage or input, `3`
non-conclusive, `4` publication) match `publish-report` in `src/main.rs`, and the
"retain both engine records" and publication rules match FR-005.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | FR-009 now runs over a quire-contract-ir executable projection and states that QSL does not emit it. VER-72's outcome is an analysis runner from the QSL package, and VER-73's ACs ask for a fixture QSL package and invocation through producer-execute; neither is testable from this spec. Report for a fixture package and exit status reflects pass/fail are testable (FR-009-AC-1 to AC-3, TC-015). Fix: an owner decision, either accepting the re-scope and amending VER-73, or specifying a QSL-to-projection path. | spec/functional/FR-009-package-runner.md Dependencies |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in 9e07114 |
| FND-002 | fixed | verified in 9e07114 |
| FND-003 | fixed | verified in 9e07114 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | medium | ContractPackageV1::read does not exist on quire-contract-ir main. The reader is ContractPackageWireV1::read(bytes, limits, dependencies), which returns ContractPackageReadResult::Admitted(Box<ContractPackageV1>), Refused, Incomplete or CatalogUnavailable (crates/quire-contract-model/src/checked_package/v2/contract_package.rs:1702, 1071). It needs caller-supplied CheckedPackageReadLimits and a ContractDependencies map of admitted dependency packages, and FR-009 supplies neither, with only one --package. Fix: name the real reader; state that dependencies are empty (a package with dependencies exits 2), that limits are CheckedPackageReadLimits::bounded(), and how Incomplete and CatalogUnavailable map to exit classes. | spec/functional/FR-009-package-runner.md Input / AC-6 |
| FND-006 | medium | examples/checked-handoff/model.semantic-ir.json is a semantic-IR domain model (keys constructs, types, populations; format semantic-ir/2.0.0). QSL's checked_handoff::write takes it as a model input; it is not a checked package. So the TC-015 step that lowers it through quire-contract-model cannot run, and FR-009's input section calls it a checked package. Gap 1 also says QSL tests do not call the IR lowering, but the cfg(test) module qsl-package/src/emit/extent_agreement.rs calls CheckedPackageV2::lower. Fix: produce the checked package at test time with QSL's checked_handoff::write over the examples/checked-handoff units and model, then lower it with CheckedPackageV2::lower to CompleteContractPackageV2::canonical_bytes; correct the Gap 1 wording. | spec/test/TC-015-package-runner.md Test Procedure |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-004 | fixed | verified in 720b33c |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | Analysable clauses are claim nodes with form analysis_claim, but on quire-spec-language main no non-test source refers to ClaimForm::AnalysisClaim or the string analysis_claim; the only use is the admission corpus test qsl-package/src/emit/tests/admission_corpus.rs:774. No committed fixture contains one. A package lowered from the examples/checked-handoff units therefore has no analysable clause and exits 2. That makes the QSL-derived fixture of TC-015 unable to produce the exit 0 or 1 of AC-14 and case 15, and Gap 1 does not say so. Fix: state in Gap 1 that QSL emits no analysis_claim today, expect exit 2 for the QSL-lowered fixture, and run AC-14 on a tests/fixtures contract package. | spec/functional/FR-009-package-runner.md AC-14 / Gaps 1 |
| FND-008 | low | The TC-015 lowering step names no requested node keys and no CompleteLoweringProfileV2, both required by CheckedPackageV2::lower. FR-009 also gives no exit class for a Boolean analysis_claim whose term the (still missing, Gap 2) translation cannot lower. Fix: name the lowering inputs, and state that an untranslatable term exits 2. | spec/test/TC-015-package-runner.md Test Procedure |

## Dispositions (round 3)

| FND | outcome | reason |
| --- | --- | --- |
| FND-005 | fixed | verified in 665567a against code |
| FND-006 | fixed | verified in 665567a against code |
