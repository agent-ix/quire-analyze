---
id: SR-17050
title: "Code review of VER-71 linear integer arithmetic lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; quire-analyze#92 diff origin/main...HEAD: README.md, schemas/differential-report-v2.schema.json, src/report.rs, src/smt.rs, tests/analysis_semantics.rs, tests/golden/boolean-v2.smt2, tests/golden/integer-sat-v2.smt2, tests/integer_lowering.rs, tests/smt_lowering.rs; context src/analysis.rs, docs/images/figures.py, tests/adr_0010.rs"
review_set: subset
---

## Summary

Ticket: VER-71. Code review with the Rust lane (rust-style loaded) of PR #92, which lowers linear
integer arithmetic under profile `quire.smtlib2/v2` with logic `QF_LIA` (FR-010). Focused checks run on
head 1aacda2 with cargo 1.98.1: lib, integer_lowering, smt_lowering, analysis_semantics and adr_0010
tests all pass (the two real-solver tests are ignored; Z3 and cvc5 are not installed).

## Verdict

The lowering conforms to FR-010: v2 replaces v1 with no fallback, every query declares `QF_LIA`, the
exact construct list and refusal classes match, maximal constant subtrees fold with checked i128
arithmetic and the constant first, `i128::MIN` is emitted via `unsigned_abs` as `(- 170141183460469231731687303715884105728)`,
and range assertions form one unnamed block in ascending symbol order outside the AssertionMap and
the report `assertions[]`. Binding groups require an identical type-shape digest, which includes the
range, so a shared symbol cannot carry two ranges. `ReplayExpression::Unreplayed` is fail-closed: the
Boolean model parser rejects any `Int`-sorted definition, so a model for a query with an integer symbol
is never verified, and an integer-only constant comparison evaluates to `None` unless Boolean
short-circuiting makes it irrelevant. No wrong verdict follows. The report validator and schema refuse
v1 and `QF_UF` with no fallback. The deleted `arithmetic_is_explicitly_unsupported` asserted that an
integer comparison is refused, which FR-010 now makes exact, so it was obsolete. No build artifacts,
roadmap text, hashes or compat layers were added. Not mergeable as is: FND-001 is a test that can
never pass on a host with real solvers.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The ignored real-solver test asserts `Agreement` and `Satisfied` for the satisfiable integer fixture, but no integer model can be verified before FR-011 replay: `parse_boolean_model` requires sort `Bool`, so every `Int` define-fun fails, the explanation is `Incomplete`, `verified_if_required` fails and the disposition is `Inconclusive`. The test fails on every host where Z3 and cvc5 exist. Assert the sat half as `Inconclusive` with an incomplete explanation pending FR-011, or test only the unsat half. | tests/integer_lowering.rs:694-716, src/analysis.rs:617-623, src/report.rs:185-193 |
| FND-002 | medium | `unsat_fixture_agrees_and_integer_models_stay_unverified` uses controlled solver responses but is tagged FR-010-AC-7, which is the real-solver criterion. TC-016 says the real-solver row is not complete where the solvers are absent; this tag backs it anyway. Tag it with the criterion it verifies (FR-010-AC-8, or NFR-004 replay) and leave AC-7 only on the real-solver test. | tests/integer_lowering.rs:632-666 |
| FND-003 | medium | The path of an unsupported integer construct is relative to the enclosing comparison (`expression.left`, `expression.left.right`), not to the statement root, so a refused product nested inside `and` reports a path that names a different node. FR-010 requires the path of the offending expression. The test only checks `starts_with("expression")`. | src/smt.rs:457-464, tests/integer_lowering.rs:533 |
| FND-004 | medium | The refusal class (`non_linear_arithmetic` and the others) exists only as a prefix of the human-readable message, and the tests branch on `message().starts_with(class)`. FR-010 requires a stable diagnostic naming the class; rust-style requires a typed condition and forbids branching on error text. Add a typed class to `LoweringError` (or a `Copy` enum) and assert on it. | src/smt.rs:646-648, tests/integer_lowering.rs:528-532, src/smt.rs:886 |
| FND-005 | low | User-facing docs still describe the Boolean-only profile: the README hero alt text says the query is "over Booleans", and the figure source draws "QF_UF · Booleans" and "non-Boolean constructs are refused when lowering" (rendered into the hero SVGs). The README Outcomes section does not say that an integer sat model stays `Incomplete`, so a differential run never agrees on an integer sat until replay lands. ADR-0010 and tests/adr_0010.rs naming v1 are a historical decision record and are fine. | README.md:18, docs/images/figures.py:33-49, README.md:79-84 |
