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
