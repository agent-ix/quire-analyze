---
id: SR-17053
title: "Gap analysis of enum datatype lowering"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; FR-020 (enum part), NFR-020, FR-010, FR-005-AC-6, TC-020 against src/smt.rs, src/report.rs, schemas/differential-report-v2.schema.json, tests/enum_lowering.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-005", type: reviews }
---
# Gap Analysis of Enum Datatype Lowering

## Summary

Ticket: VER-85. PR agent-ix/quire-analyze#94. Plan completion: not assessed. The
analysis is limited to the enum part of FR-020. Records and options belong to
VER-86, model replay to VER-87 and package translation to VER-88. The record and
option clauses of FR-020-AC-3, AC-4 and AC-5 are out of scope, and so are the
quantifier logics `LIA` and `UFDTLIA` of AC-9 (FR-032). The traces come from
`quire trace --id`. `quoin matrix --json` (quoin 0.30.1) reports every criterion
in this repo as `untagged`, including ones tagged on `main` before this PR, so it
does not read the `#[trace]` attribute here. This is a tooling observation, not a
PR finding.

## Verdict

**FAIL (one high, one medium, two low).** The enum-ordering and mixed-sort claims
of FR-020-AC-5 do not match the code path any statement can reach, and the AC-2
test asserts no refusal code. The other in-scope criteria are implemented and
covered by tests that can fail: AC-1 in its controlled-response form, AC-6, AC-7,
AC-8 and the enum part of AC-9, plus NFR-020-AC-1 (variants and datatypes) and
NFR-020-AC-2, and FR-005-AC-6 (logic). Depth is trivially 1 for an enum, so the
depth bound has no enum case. The FR-020-AC-1 real-solver test is gated by
`#[ignore]` (z3 and cvc5 are not installed here), and its assertion defect is
SR-17052 FND-003.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-020-AC-5 and the case table say an enum ordering comparison and a mixed-sort comparison return `UnsupportedConstruct` from expression lowering. Every `StatementInput` passes `check_expression`, and upstream `check_compare` refuses both as `IllTypedExpression`, which `from_clause` maps to `InvalidStatement` at path `statement.expression`. So the observable code is `InvalidStatement`. The two lowering branches cannot be reached, and the AC's only test accepts any error. Either the spec is amended so these cases are refused at statement build as `InvalidStatement` (an owner decision) or the AC stays unmet. | `spec/functional/FR-020-datatype-lowering.md` AC-5, `src/smt.rs:241-258`, `src/smt.rs:1035-1053`, `tests/enum_lowering.rs:485-500` |
| FND-002 | medium | The only FR-020-AC-2 refusal test asserts `.is_err()` on the helper's own `check_expression` / `DeclarationEnvironment::new` result, stringified, so no `LoweringErrorCode` is ever checked. The AC asks for "the code of the case table". The unresolved-variant and unresolved-enumeration cases are reachable through `StatementInput::from_clause` as `InvalidStatement` / `statement.expression`, for example by passing a clause checked under a wider environment, and are never asserted there. | `tests/enum_lowering.rs:446-459` |
| FND-003 | low | Spec gap in the FR-020 case table. The row "a value declaration or binding whose type name does not resolve → `InvalidBinding`, path `binding.type`, by binding validation" cannot be reached for a value declaration: `DeclarationEnvironment::new` refuses it as `OrphanedTypeReference` (`validate_type`), so no environment or statement exists. That is the same situation as the `RecursiveType` row, which the table already records as an environment-build refusal. | `spec/functional/FR-020-datatype-lowering.md` (Declarations come from the environment) |
| FND-004 | low | Spec gap. The FR-020 capability-contract bullet lists `enum_literal` and `enum_equality` but not `enum_inequality`. The code lists `enum_inequality` as exact, which is correct because FR-020 lowers `!=` to `distinct`, but the bullet and AC-6 do not name it. | `spec/functional/FR-020-datatype-lowering.md` (Capability contract), `src/smt.rs:459-461` |

## Coverage

Examined, with no finding:

- FR-020 enum encoding: sorts come from the environment, only referenced enums are declared, and constructor symbols are injective (a hex-encoded variant under a digest of package, enum name and sorted variants).
- Deterministic ordering: a `BTreeMap` keyed by sort symbol, with variants sorted. Membership is the existing `or` with no separate encoding, and `=`/`!=` lower to `=`/`distinct`.
- Logic: `QF_LIA` without datatypes, `QF_UFDTLIA` with them, both under v2. The schema allows exactly those two logics.
- FR-010-AC-1: the contract logic is still `QF_LIA` for datatype-free queries.
- NFR-020 counting: variants are counted per enum, and datatypes as distinct emitted sorts. 256 is accepted and 257 is `ResourceLimit` before any solver runs.
- FR-021 consistency: an enum model is never verified (SR-17052 Summary).
- The new `QueryBundle::enum_sorts()` / `EnumSortMap` API.
- TC-020: the lowering, permutation and bound parts for enums.

The edits in `src/report.rs` were judged only for this PR's change (the logic check), not the concurrent VER-82 work.

## Dispositions

Round 1, verified against PR head 2c7a7a1 (fix commit 1fe1917, spec commit 2c7a7a1).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | Per the ruling, FR-020's case table and AC-5 now say enum ordering and mixed-sort comparisons are refused at statement build as `InvalidStatement`, with `UnsupportedConstruct` as a defensive lowering check over an unchecked expression. Both paths are tested. |
| FND-002 | fixed | `unresolved_names_are_refused_before_lowering` feeds `StatementInput::from_clause` a clause checked under a wider environment and asserts `InvalidStatement` at path `statement.expression` for an unresolved variant and an unresolved enumeration. It also asserts that an orphaned value type fails at `DeclarationEnvironment::new`. |
| FND-003 | fixed | The case-table row now reads "refused by `DeclarationEnvironment::new` as `OrphanedTypeReference`, so no environment and no statement exists". |
| FND-004 | fixed | The capability bullet lists `enum_inequality`. |
