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
