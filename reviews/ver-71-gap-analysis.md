---
id: SR-17051
title: "Gap analysis of VER-71 linear integer arithmetic lowering"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; quire-analyze#92 diff origin/main...HEAD against FR-010, FR-001, FR-002, FR-005-AC-6, NFR-004, interface_001 and TC-016 on main; tests/integer_lowering.rs, tests/smt_lowering.rs, src/smt.rs, src/report.rs, schemas/differential-report-v2.schema.json"
review_set: subset
---

## Summary

Ticket: VER-71. Planless gap analysis. Plan completion: not assessed. `quoin matrix` and
`quire matrix --scope .` report every criterion of the repository untagged ("no declared matrix row"),
on main as well as on this branch, so the computed matrix gives no per-PR signal. The criterion-to-test
mapping below was done by reading the `#[trace]` tags and the test bodies.

## Verdict

FR-010-AC-1, AC-2, AC-3, AC-4, AC-5, AC-6, AC-8, AC-10 and AC-11, NFR-004-AC-1, AC-2 (lowering half) and
AC-4, and FR-005-AC-6 (v2 profile and logic refusal) each have a tagged test that can fail and that
asserts the criterion. FR-001 and FR-002 on main already name `quire.smtlib2/v2` and `QF_LIA`, and
interface_001 names v2, `QF_LIA` and the refusal classes; the code agrees with all three. No source
code was added without an owning requirement. Gaps: AC-9 is untagged, AC-7's satisfiable half cannot
pass before FR-011, and AC-4's "status is unsupported" is not asserted.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-010-AC-7 requires that real solvers decide the satisfiable fixture and that the differential reports agreement. Under FR-005, agreement requires every sat model to be replay-verified, and integer replay is FR-011, which FR-010 does not list as a dependency. Until FR-011 lands, AC-7's sat half is unreachable and its only test (SR-17050 FND-001) fails. Record the dependency on FR-011 in FR-010 or split AC-7 so the sat-agreement half is verified by TC-017. | spec/functional/FR-010-linear-integer-arithmetic.md:96, tests/integer_lowering.rs:694-716 |
| FND-002 | medium | FR-010-AC-9 (each TC-010 Boolean fixture lowers under v2 and decides the same status; no v1 query) has no tagged test. The renamed Boolean golden in `golden_query_is_order_independent_and_source_mapped` exercises it but is tagged only to FR-002/TC-010. Tag the test(s) that assert it with FR-010-AC-9. | tests/smt_lowering.rs:207-245 |
| FND-003 | low | FR-010-AC-4 says "the status is `unsupported`", but the test asserts only `LoweringErrorCode::UnsupportedConstruct` from `lower_boolean_statements`; no analysis status is checked. The library has no status for a refused lowering, so either assert the status where one is produced (run-package) or state in the AC that the error code is the status. | tests/integer_lowering.rs:519-586 |
