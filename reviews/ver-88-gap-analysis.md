---
id: SR-17055
title: "Gap analysis of the FR-022 datatype package translation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; src/package_translation.rs, tests/datatype_translation.rs, tests/package_runner.rs, tests/common/mod.rs against FR-022, FR-020 (Replay until FR-021, AC-10), FR-009, FR-012, NFR-020, TC-020"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-022", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
---
# Gap Analysis of the FR-022 Datatype Package Translation

## Summary

Ticket: VER-88. PR agent-ix/quire-analyze#96. Plan completion: not assessed.
`quoin matrix --repo . --json` (quoin 0.30.1) reports every one of the
repository's 234 criteria `untagged` or `method-without-symbol`, including the
pre-existing FR-009 rows, so it does not read this repository's `#[trace(...)]`
attributes; the AC-to-test mapping below is manual from the trace attributes.

Manual mapping: FR-022-AC-1 has non-ignored tests for fixture 2 (unsat, exit `1`),
fixture 1's satisfiable model (exit `3`) and fixture 6 (exit `2`, `claim-N`);
fixtures 3, 4 and 5 have only `#[ignore]` translation tests and no `run-package`
test. AC-2: enum declarations and the unsupported-payload refusal are tested; the
record half is ignored. AC-3: text, rational and sequence nesting, `enum.lt`,
`text.eq` and `collection.contains` are tested at both levels. AC-4 and AC-5 are
tested. FR-012 style (one translation table extended, named constants, one
`Operation` enum) is followed.

Coder choices judged against the spec: `enum_value` member nodes as enum literals
(spec-silent, sound: the node body is the enum literal); declarations only from
reached `semantic_type` nodes (required by FR-022 Types); type symbols `t{N}`
(spec-silent; FR-020 requires distinct sorts, which node-indexed names give, and
AC-4 determinism holds); field and payload types Boolean, integer, enum, record or
option, else exit `2` at the type node (matches FR-020 Encoding and FR-022
Refusals); record cycle refused at the type node (FR-020 assigns this to
`DeclarationEnvironment::new` as `RecursiveType`; refusing earlier is consistent
with FR-022 "naming the node"); type nesting guarded at 128, not NFR-020's 8
(spec-silent for translation: NFR-020 constrains FR-020 lowering, so a nesting of
9 to 128 passes translation and is refused by the VER-86 lowering as `claim-N`; the
fields 64, datatypes 256 and variants 256 bounds are likewise left to lowering).

## Verdict

**FAIL (one high, one medium).** FR-022-AC-1 contradicts the later FR-020 ruling
and cannot pass until VER-87; three of its six fixtures do not run through
`run-package` in this PR. The translation code itself conforms to FR-022.

Amendment for FR-022-AC-1 (replace the row's Criteria text):

> Six built contract-package fixtures run end to end through `run-package`, with
> a stub `producer-execute` and with real solvers where installed, each with a
> fixed expected exit: an enum-membership disjunction (`role = Admin` or
> `role = Auditor`) with `role != Admin`; the same disjunction with `role = Guest`,
> which exits `1`; a record-field claim `record.project(user)` with field `role`
> equal to `Admin`; and the same with a second claim that field equal to `Guest`,
> which exits `1`. A fifth fixture is a single claim, `quire.op.boolean.implies`
> with antecedent `quire.op.option.present` of an option-of-`Role` parameter `o`
> and consequent `quire.op.enum.eq` of `quire.op.option.value` of `o` with
> `Admin`; it translates to an `Unwrap` and the request is satisfiable (`o`
> absent, or `o` present as `Admin`). The first, third and fifth fixtures are
> satisfiable: while the "Replay until FR-021" rule of FR-020 applies to a
> datatype kind a fixture declares, it exits `3` with both engines `satisfied`,
> the witness incomplete with reason `sort_not_replayable` and the differential
> run inconclusive; once FR-021 replay of every kind it declares lands, it exits
> `0`. A sixth fixture is the same consequent alone, without the antecedent, and
> exits `2` naming the clause `claim-N` and the validator's path.

Amendment for TC-020 Expected Results (append):

> A satisfiable datatype fixture run through `run-package` exits `3` (witness
> incomplete with `sort_not_replayable`, differential inconclusive) while the
> FR-020 "Replay until FR-021" rule applies to a kind it declares, and `0` once
> FR-021 replay of that kind lands; an unsatisfiable one exits `1`.

and in TC-020 Test Procedure, add "an option-access claim under a
`quire.op.option.present` guard" to the `run-package` fixture list.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-022-AC-1 requires exit `0` for satisfiable fixtures 1, 3 and 5, but FR-020 "Replay until FR-021" (and FR-020-AC-10) makes every model of a datatype query unverified, so they exit `3` until VER-87; the test traced to AC-1 asserts `3`, the opposite of the AC text. Amend FR-022-AC-1 and TC-020 as in the Verdict. | `spec/functional/FR-022-datatype-package-translation.md:72`, `tests/package_runner.rs:1525-1541` |
| FND-002 | medium | FR-022-AC-1 fixtures 3, 4 and 5 (record field sat, record field with `Guest`, guarded option) have no `run-package` test even as ignored, and their translation tests are `#[ignore]` pending VER-86; fixture 4's exit `1` is asserted nowhere. AC-1 and the ticket's record-field acceptance criterion are not met by this PR. | `tests/datatype_translation.rs:205-270`, `tests/package_runner.rs:1494-1541` |
| FND-003 | low | The record-cycle refusal (`declaring` set) and the type-nesting guard have no test, though both are new refusal paths naming a type node. | `src/package_translation.rs:569-574`, `src/package_translation.rs:677-679` |
| FND-004 | low | FR-022 is silent on two translation rules the code applies: a `Reference` to an enum `value/enum_value` member node translates to `EnumLiteral`, and enum and record declarations are named `t{N}` after the type node index. Add both to the FR-022 Terms table and Types bullet. | `src/package_translation.rs:777-779`, `src/package_translation.rs:636-639` |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 7e067c9: FR-022-AC-1 and TC-020 carry the amendment text verbatim; consistent with FR-020 "Replay until FR-021" and FR-020-AC-10. |
| FND-002 | deferred | 7e067c9 adds `record_and_option_fixtures_have_their_fixed_exits_through_the_runner` for fixtures 3-5 (exits 3, 1, 3); it and the three translation tests are ignored and fail only in the src/smt.rs lowering ("record expressions are unsupported"). Un-ignore all four when VER-86 lands; VER-88 stays open until they pass. |
| FND-003 | fixed | 7e067c9: record-cycle test (byte-level mutation adding a self field and recursion group, permitted by FR-009-AC-27, the shape of a QSL recursive record) and a 130-level nesting test. |
| FND-004 | fixed | 7e067c9: FR-022 Terms row adds the `value/enum_value` reference and Types states the `t{N}` name. |
