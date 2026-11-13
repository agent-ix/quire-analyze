---
id: SR-17047
title: "Failure-domain review of VER-70 integer witness replay and lowering"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-analyze; quire-analyze#90 diff origin/main...HEAD: spec/functional/FR-002, FR-009, FR-010, FR-011, FR-012; spec/nonfunctional/NFR-004; spec/test/TC-016..018; spec/index.md; spec/test-matrix.md"
review_set: subset
---

## Summary

Ticket: VER-70. Failure modes, identity and boundary cases of FR-010, FR-011 and NFR-004 in the PR #90 diff, checked against src/analysis.rs replay on main and the quire-contract-ir checker. The witness-replay reason set is closed (eight values) and each value has an AC, but precedence between reasons is undefined, one reason may be unreachable, and the over-long boundary is open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-011 reports 'one reason' from the closed set but gives no precedence when several apply (a model that omits one symbol and puts another out of domain; a repeated symbol whose second value is malformed). Two implementations can report different reasons for the same model, and a test cannot fix the expected one. Fix: state the check order (for example parse/value_malformed, then symbol set, then sort, then domain, then assertions) or report every failing reason. | spec/functional/FR-011-integer-witness-replay.md:35-38 |
| FND-002 | medium | FR-011 says a verified model 'defines no other symbol'. The Boolean parser on main (parse_boolean_model, src/analysis.rs:603-647) accepts and skips define-fun entries that name a query assertion alias, because named assertions can appear in a returned model. FR-010 adds a named range assertion per integer symbol, so this case gets more likely, and under FR-011 such a model would fail with symbol_unknown. Fix: state that named-assertion aliases are skipped, not counted as symbols. | spec/functional/FR-011-integer-witness-replay.md:29-30 |
| FND-003 | medium | evaluation_overflow may be unreachable. quire-contract-ir's checker attaches a CheckedRange obligation to every Reject-policy integer result and refuses one whose interval may leave its named bounds (expression.rs:2682-2697, 3121-3127). Saturating results are unsupported (FR-010), and FR-011 checks each value's domain before replay. So for an admitted expression and an in-domain model, no exact i128 evaluation can overflow, and FR-011-AC-7 names no way to build such an assertion. Fix: say how the fixture is built (an unchecked hand-built expression), or keep the reason as defensive and verify it by inspection. | spec/functional/FR-011-integer-witness-replay.md:58 |
| FND-004 | medium | 'An over-long numeral' fails with value_malformed, but no length or magnitude bound is stated. A numeral outside the declared domain but inside i128 is value_out_of_domain under FR-011-AC-5, so where value_malformed starts is open. Fix: define over-long, for example a numeral that does not fit i128 is value_malformed, and one that fits is checked against the domain. | spec/functional/FR-011-integer-witness-replay.md:57 |
| FND-005 | low | FR-010 lowers over the mathematical integers and asserts ranges only for symbols, not for intermediate results. That is sound only because upstream check_expression discharges a CheckedRange obligation for each Reject-policy result. FR-002 states the matching condition for Boolean definedness; FR-010 does not. Fix: state that lowering admits only expressions whose Reject-policy range obligations upstream validation has discharged. | spec/functional/FR-010-linear-integer-arithmetic.md:43-48 |

## Scope

| Unit | Role | Path |
| --- | --- | --- |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-1 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-2 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-3 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-4 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-5 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-6 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-7 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-8 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-011-AC-9 | examined | spec/functional/FR-011-integer-witness-replay.md |
| FR-010 | examined | spec/functional/FR-010-linear-integer-arithmetic.md |
| NFR-004-AC-2 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| NFR-004-AC-3 | examined | spec/nonfunctional/NFR-004-integer-exactness.md |
| FR-012-AC-3 | examined | spec/functional/FR-012-integer-claim-translation.md |
| FR-012-AC-7 | examined | spec/functional/FR-012-integer-claim-translation.md |

## Verdict

FAIL at medium: none of these is a contradiction, but FND-001 and FND-004 leave expected test outcomes undefined. Clean: reference cycles and size/depth bounds are covered for integer terms (FR-012-AC-3, AC-7); replay purity is stated and tested (FR-011-AC-9); a failed replay keeps the logical status (FR-011-AC-8); symbol identity follows the variable map.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | verified in f1ad0f5 |
| FND-002 | fixed | verified in f1ad0f5 |
| FND-003 | still-open | f1ad0f5 gives a construction for AC-7 (a symbol over the full i128 range at its maximum, `x + 1 > 0`) and states that evaluation_overflow 'is reachable when a declared domain spans the 128-bit extremes'. That fixture cannot reach replay. StatementInput::from_clause (src/smt.rs:171-215) runs check_expression on every clause, and check_integer_operator on quire-contract-ir main (crates/quire-contract-model/src/expression.rs:2672-2678) refuses an addition whose interval exceeds the checked intermediates ('integer interval computation exceeded checked intermediates'). So x + 1 with x up to i128::MAX is refused before lowering. The reason is still unreachable for admitted statements, and AC-7 and the second clause of NFR-004-AC-2 still name no admissible fixture. Fix: verify evaluation_overflow by inspection as a defensive reason, or build the fixture from an unchecked replay expression in a unit test, and drop the reachability sentence. |
| FND-004 | fixed | verified in f1ad0f5 |
| FND-005 | fixed | verified in f1ad0f5 |
