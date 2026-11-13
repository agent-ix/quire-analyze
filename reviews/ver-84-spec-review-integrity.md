---
id: SR-17038
title: "Integrity review of VER-84 datatype lowering, replay and translation (FR-020..022, NFR-010, TC-020)"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze; quire-analyze#88 diff origin/main...HEAD: spec/functional/FR-020-datatype-lowering.md, spec/functional/FR-021-datatype-witness-replay.md, spec/functional/FR-022-datatype-package-translation.md, spec/nonfunctional/NFR-010-datatype-bounds.md, spec/test/TC-020-enum-datatype-analysis.md; context src/smt.rs, src/package_translation.rs, quire-contract-model at the locked quire-contract-ir rev, sibling PRs #87 #89 #90"
review_set: subset
---

## Summary

Ticket: VER-84. Consistency and completeness check of the five new artifacts against the code on
`main` (`src/smt.rs` `CAPABILITY_CONTRACT`, `lower_*`; `src/package_translation.rs` term table),
against `quire-contract-model` `ExpressionKind`, `DeclarationEnvironment` and `check_expression` at
the commit `Cargo.lock` resolves, and against the open sibling spec PRs that touch the same
contract (#87 quantifiers, #89 hybrid, #90 linear integer arithmetic).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-020 specifies lowering for "membership of an enum-typed term in a set of literals", but the analysis algebra has no membership expression: `ExpressionKind` has no `In`/`Contains` variant. In QSL, membership is `quire.op.collection.contains` over a `quire.op.collection.set` literal. That is a collection construct, and FR-020 and FR-022 both keep it unsupported (FR-020-AC-6, FR-022-AC-3). In the algebra, the only forms are `Quantifier` over a `CollectionLiteral`, which #87 FR-031 owns, or an explicit `Boolean`/`Compare` disjunction, which already lowers. So FR-020-AC-1, FR-020-AC-8 and FR-022-AC-1 rest on a source form that does not exist. Fix: name the source form, and either admit enum-element `collection.contains` over a literal set as an exact special case (and say so in the unsupported rule), or drop membership and point to #87. | spec/functional/FR-020-datatype-lowering.md:60-63, FR-020-AC-1, FR-020-AC-8 |
| FND-002 | high | The logic rule cannot be satisfied with the contract's shape, and it conflicts with the integer contract it depends on. `CapabilityContract.logic` is one `&'static str`, and `smt.rs` hard-codes `(set-logic QF_UF)`. #90 FR-010-AC-1 fixes that one logic to `QF_LIA`. FR-020 wants a logic "for the datatypes the query uses" and names only `QF_DT` (datatypes and Boolean). It does not name the logic for datatypes with integer fields (for example `QF_UFDTLIA`, `QF_DTLIA` or `ALL`). It does not say whether a Boolean-only or integer-only query keeps its current logic and bytes, or how the contract carries more than one logic. No AC tests the logic. | spec/functional/FR-020-datatype-lowering.md:51-53 |
| FND-003 | medium | FR-022 lists the operations to translate by meaning ("each catalog operation that denotes … record construction, … option construction … and option access"). It names no identity, but FR-009's term table is keyed by identity (`quire.op.boolean.*` in `src/package_translation.rs:35-40`). At the locked quire-contract-ir rev, the catalog has `quire.op.enum.eq`/`ne`, `quire.op.structural.eq`/`ne`, `quire.op.record.project`, `quire.op.option.present` and `quire.op.collection.contains`. No record-construction, option-construction or unwrap identity was found. An absent option arrives as `CheckedLiteralValue::None`, a `Literal`, which the term table does not map. Fix: list the exact identities (including `structural.eq` for record and option equality) and the `Literal`/`Aggregate` forms that carry record and option construction. | spec/functional/FR-022-datatype-package-translation.md:32-38 |
| FND-004 | low | The new ACs write their Verification cell as `Test (TC-020)` (and `TC-015` in FR-022-AC-1). Repo convention is `Test`, with TC-to-FR linkage in the TC frontmatter. `quire validate --okf` emits 24 `dangling-trace-reference` warnings on exactly these cells. TC-015 also does not declare `verifies` FR-022. Fix: use `Test`, and keep the linkage in TC-020's `relationships`. | spec/functional/FR-020-datatype-lowering.md:100-107, spec/functional/FR-022-datatype-package-translation.md:51 |
| FND-005 | low | The id block is inconsistent. The functional and test ids use the 020 block (FR-020..022, TC-020), matching the sibling blocks 010 (#90), 030 (#87) and 040 (#89), but the NFR is NFR-010. That id sits in the integer block, and #87 and #89 use NFR-030 and NFR-040. Fix: rename it NFR-020. | spec/nonfunctional/NFR-010-datatype-bounds.md:2 |

## Verdict

FAIL. FND-001 and FND-002 need spec changes before implementation: the membership source form
does not exist in the algebra as specified, and the logic rule conflicts with the single-logic
contract that #90 fixes to `QF_LIA`. The rest checks out against the code. `DeclarationEnvironment`
is the source of declarations, `CheckedApplicationOperation::identity()` exists and is how FR-009
already identifies operations (`src/package_translation.rs:568`), the contract construct names
match `CAPABILITY_CONTRACT`, and integer record fields are correctly made conditional on the LIA
contract. The new ids do not collide with #87, #89 or #90. `spec/index.md` and
`spec/test-matrix.md` correctly do not enumerate the new ids. No hash, SHA, pin or version
tracking, no compatibility layer, no roadmap content and no paywalled standard text appear in
the diff.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | high | FR-022 states that "the catalog has no identity for … unwrap", so every option read exits `2`. The locked `quire-verification-contracts` catalog (rev 1bf72e6, `contracts/checked-operation-catalog-v1.json:180`) defines `quire.op.option.value`, with operator `value`, one `option` operand and result `inner:0`. That is the unwrap. FR-022 therefore drops the option access that VER-84 scopes, on a false premise. SR-17038 FND-003 searched only quire-contract-ir and was wrong on this point. Fix: add `quire.op.option.value` → `ExpressionKind::Unwrap` to the term table, under the implies-guard rule, and drop the "if a later catalog adds an unwrap" clause. | spec/functional/FR-022-datatype-package-translation.md:45-56 |
| FND-007 | low | FR-020 links `./FR-010-lia-lowering.md`, but #90 names the file `FR-010-linear-integer-arithmetic.md`. That link will dangle even after #90 lands. `quire validate --okf` reports it as one of the three FR-010 dangling references, and it is the only one of the three that #90 landing will not clear. Fix: point the link at `./FR-010-linear-integer-arithmetic.md`. | spec/functional/FR-020-datatype-lowering.md:65 |
| FND-008 | low | FR-020-AC-8 requires `quire.op.collection.contains` over `quire.op.collection.set` to return `UnsupportedConstruct`. Those are package catalog identities that FR-020's lowering never sees: FR-022 refuses them at translation with exit `2`. The analysis-algebra counterpart is `CollectionLiteral`. Fix: state the clause on `CollectionLiteral` in FR-020, and keep the catalog-identity case in FR-022-AC-3. | FR-020-AC-8 |
| FND-009 | low | The fix extends the hand-maintained id inventory in `spec/index.md` (Requirements Architecture) with FR-020..022, NFR-020 and TC-020. That prose is a manual inventory of ids, which is ceremony. `quire validate --okf` does not require it and does not flag its absence. Fix: revert the added sentence. | spec/index.md:54 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Membership is now authored and lowered as a disjunction of enum equalities. `enum_membership` is no longer in the exact list, and `collection.contains` stays unsupported. |
| FND-002 | still-open | `QF_UFDTLIA` is a valid logic name: it is an SMT-COMP division, z3 5.4.1 and cvc5 1.4.2 both accept it, and both reject a bogus name. But FR-020 still conflicts with #90 at head f1ad0f5. #90's FR-002 says v2 replaces v1, every query (including Boolean-only) declares `QF_LIA`, and v1 is not selectable. #90's FR-005-AC-6 requires reports to carry `quire.smtlib2/v2` and `QF_LIA`, and refuses `QF_UF`. FR-020 instead keeps v1/`QF_UF` for Boolean-only queries, selects profiles per query, and does not amend the report schema or validator for `QF_DT` and `QF_UFDTLIA`. FR-020-AC-6 also still names one `CAPABILITY_CONTRACT` across several profiles. |
| FND-003 | fixed | FR-022 now has an identity-keyed term table, and the `none` literal maps to `OptionNone`. The false unwrap claim the fix relied on is recorded as new FND-006. |
| FND-004 | fixed | The Verification cells now read `Test`, the TC-015 citation is gone, and the dangling-trace-reference warnings are cleared. |
| FND-005 | fixed | Renamed NFR-020, and TC-020 now verifies NFR-020. |
