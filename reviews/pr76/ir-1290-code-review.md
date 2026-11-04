---
id: SR-17023
title: "IR-1290 PR #76 code and Rust review"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; IR-1290 PR #76; Cargo.toml, Cargo.lock, README.md, schemas/differential-report-v2.schema.json, src/report.rs, src/smt.rs, src/solver.rs, tests/analysis_semantics.rs, tests/golden/boolean-v1.smt2, tests/smt_lowering.rs, tests/solver_adapters.rs, tests/foundation.rs, planning/smt-lowering-gap-analysis.md; context spec/interface/interface-001-analysis-api.md"
review_set: subset
---
# SR-17023: IR-1290 PR #76 code and Rust review

## Summary

The solver pin and redundant hashes are removed in the changed code and schema. Initial review found two test defects; the coder fixed both in the same PR.

## Verdict

**PASS** — both findings have final fixed dispositions on the reviewed PR head. The final aggregate gate belongs to the code lead.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The golden SMT fixture now contains literal `<digest>` placeholders, so it is no longer a valid query for independent review, while README links it as a sample. Restore a valid golden query and compare exact bytes. | tests/golden/boolean-v1.smt2:8, tests/smt_lowering.rs:239, README.md:59 |
| FND-002 | medium | The new clause digest has no content-mutation control: a constant digest over same-ID clauses would pass the current tests because the golden normalizes all digest bytes and the only digest assertion compares the result with itself. Add a test changing canonical clause content under the same identity and assert distinct digest and binding behavior. | src/smt.rs:232, tests/smt_lowering.rs:252 |

## Scope

Examined FR-003-AC-4, FR-005-AC-6, FR-002-AC-3 and AC-5, NFR-002-AC-1 and AC-3, TC-005, TC-007, the closed report schema, solver adapter, direct model dependency, Cargo lockfile, README, and every changed test/fixture. The planning golden description was updated; no CI workflow file changed. The clause digest is computed from canonical clause bytes and has a real proof-content binding role; the removed executable and duplicate stream/report hashes have no demonstrated feature role. The historical v1 report fixture remains a version-refusal input and is outside the new v2 contract.

## Checks

`git diff --check` passed. The coder reported passing focused `cargo check --tests`, `solver_adapters`, `analysis_semantics`, `smt_lowering`, Clippy and format checks; I inspected their assertions. The final aggregate gate was not run in this review.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | still-open | The new all-zero suffix makes the fixture syntactically valid, but it is not the emitted clause identity. Normalizing both sides still skips the exact identity bytes, so the golden cannot independently show the actual query. |
| FND-002 | fixed | A same-ID, changed-content clause now has a different digest, and a changed report clauseDigest is refused by authoritative validation. |
| FND-001 | fixed | The golden now contains the generated clause digests, and the test compares production query bytes exactly with the committed valid SMT-LIB file. |

## Final review

At PR head `ecef279e1cccb5d756d5544dd7cc13bf34f55237`, FND-001 and FND-002 are fixed. The diff adds no CI workflow edits or new solver pins.

## Rebase recheck

At PR head `813eb7ee5f19ca864325ec97ad2d504e8ea8c477` on base `9ba5014d1c36bd4ca1d2c0223cc6e96c428bd806`, the production diff remains reviewed. The newly changed `tests/foundation.rs` parses the merged interface YAML and value-asserts its normalized statuses and raw solver examples against the public enums. The previous string search for unsupported status labels is gone. The merged interface owns the updated model dependency and solver record contract. This scoped recheck found no new defect; the code lead runs the final aggregate gate.
