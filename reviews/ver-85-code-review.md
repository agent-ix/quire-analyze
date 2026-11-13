---
id: SR-17052
title: "Code review of enum sorts and membership lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/smt.rs, src/report.rs, schemas/differential-report-v2.schema.json, tests/enum_lowering.rs, tests/golden/enum-sat-v2.smt2"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
---
# Code Review of Enum Sorts and Membership Lowering

## Summary

Ticket: VER-85. PR agent-ix/quire-analyze#94, diff `origin/main...HEAD` (one commit,
five files). Includes the rust-review lane (rust-style loaded). Enum sorts are read
from the `DeclarationEnvironment`, declared only when referenced, keyed in a
`BTreeMap` by the encoded sort symbol, and their variants are sorted. Constructor
symbols hex-encode the variant name under a digest of package, enum name and sorted
variants. Membership is the existing Boolean `or` over enum `=`. Enum `!=` lowers to
`distinct`. The variant bound refuses 257 variants and the datatype bound refuses
the 257th distinct sort, both as `ResourceLimit` before any solver runs.

Enum terms replay as `Unreplayed`. `verify_model` requires every replay assertion
to evaluate to `Some(true)`, and `parse_boolean_model` rejects any define-fun that is
not `Bool`, so a model that assigns an enum variable is never verified. A short
circuit such as `(or b <enum term>)` with `b` true can still verify, but that result
is sound because the disjunction holds whatever the enum term is. The logical status
comes from the solver outcome, and a sat agreement requires a verified model on both
engines (`src/report.rs:185-193`). So no enum model can be reported verified and no
verdict can be wrong. This matches the FR-021 deferral to VER-87.

## Verdict

**FAIL (three medium, three low).** The report validator derives the required
logic by searching the query for a substring, which a hand-made report can fool.
The enum-ordering test cannot fail, and nothing tests the new mixed-sort branch.
The gated real-solver test would fail on any host with both solvers installed.
The lowering itself is correct and deterministic.

Gates run with toolchain 1.98.1: `cargo test --test enum_lowering` (14 passed,
1 ignored), plus `integer_lowering`, `smt_lowering` and `analysis_semantics` (all
pass, with only the real-solver tests ignored). The full gate was not run, as the
brief directs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The validator works out the required logic by searching the query bytes for `(declare-datatypes ` anywhere, then accepts `(set-logic <logic>)\n` anywhere in the bytes. It never parses the top-level commands. A comment, a quoted `\|...\|` symbol or a string literal holding either text switches the result. The singular `(declare-datatype ` (valid SMT-LIB 2.6) is not counted, and more than one `set-logic` is not refused. So a standalone-validated report can name `QF_LIA` for a query whose real `set-logic` is `QF_UFDTLIA`. Example: queryHex holds `; (set-logic QF_LIA)` on its own line, then `(set-logic QF_UFDTLIA)` and a `declare-datatype`, and its queryDigest is recomputed to match. Generated queries cannot trigger this, because their symbols are hex and their set-info is fixed. | `src/smt.rs:1846-1856`, `src/report.rs:368-378` |
| FND-002 | medium | In `ordering_and_unsupported_sorts_are_refused`, the enum-ordering arm `Err(_) => {}` accepts any statement-build error. Upstream `check_compare` refuses enum ordering as `IllTypedExpression`, so that arm always runs and the `UnsupportedConstruct` assertion never executes. The new mixed-sort branch ("comparison operands must both be Boolean or both of one enum") has no test: the text case is refused by the text-literal path before that branch, and a Role-against-Level comparison is never built. | `tests/enum_lowering.rs:485-500`, `src/smt.rs:1047-1053` |
| FND-003 | medium | The real-solver test `real_z3_cvc5_decide_membership_fixtures` (ignored until z3 and cvc5 are installed) asserts `Agreement` with status `Satisfied` for the sat fixture. A real sat answer carries an enum-sorted model, which `parse_boolean_model` rejects, so the run is `Inconclusive`. The PR's own fake-solver test asserts exactly that at line 355. On a host with both solvers this test fails, so the FR-020-AC-1 real-solver evidence can never be produced. It should assert that each engine returns `sat` and that the comparison is not an agreement, which is how FR-010-AC-7 states the same case for integers. | `tests/enum_lowering.rs:604-614` |
| FND-004 | low | The README still says queries use logic `QF_LIA` and that enums are refused. Both statements are false after this change. | `README.md:47-58` |
| FND-005 | low | The `ReplayExpression::Unreplayed` doc says "An integer-valued term". It now also marks enum literals and enum comparisons. | `src/smt.rs:538` |
| FND-006 | low | The new public `EnumSortMap::constructors` is a `Vec<(String, String)>` of variant name and SMT symbol. These are two confusable strings in a public field that VER-87 will consume (rust-style: use identity newtypes when two IDs could be confused). | `src/smt.rs:518` |

## Coverage

Examined every hunk of the diff and the code it calls: `lower_statement_specs`
query assembly, `LowerState::lower` (EnumLiteral, Compare), `term_sort`,
`enum_sort`, `reference`, `type_shape_digest` and `project_type` (enum case),
`variable_key` and the binding `compatible_with` check (a shared binding symbol
always has one type shape and one package, so one sort),
`ReplayExpression::evaluate`, `verify_model`, `parse_boolean_model`, the report
validator's logic check and query-digest re-derivation, the schema `logic` enum,
`StatementInput::from_clause`, and upstream `check_compare` /
`DeclarationEnvironment::validate_type` (quire-contract-model at the locked
revision). The capability contract lists `enum_literal`, `enum_equality` and
`enum_inequality` as exact and drops `enum` from the unsupported list. The schema
allows exactly `QF_LIA` and `QF_UFDTLIA`, and v1 and `QF_UF` are refused. The
public-repo hygiene check is clean: no standard text, no roadmap, no build
artifacts, no compatibility layer, and no new hash, SHA or pin ceremony. The
digests in the golden file are encoded symbol identities.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | The `sort_not_replayable` guard in `verify_model` fires only when the query has an enum-sorted *variable* (`enum_variables()`). A query that declares an enum datatype but compares only literals, for example `flag or Role.Admin = Role.Guest` (well-typed upstream), has no enum variable. Its model is parsed and replayed under the FR-011 order, and `Or(flag, EnumUnreplayed)` short-circuits to `Ok(true)` when `flag` is true, so the model is verified. FR-020 "Replay until FR-021" says a model for a query that declares any datatype-sorted symbol (constructors included) is never verified and that this reason is checked before every FR-011 reason. The verified result is sound, but it contradicts the spec, and for such queries the failure names an assertion instead of an enum symbol. Fix by guarding on `!query.enum_sorts().is_empty()` (naming the first enum variable, or the first constructor when there is none), or narrow the spec to datatype-sorted variables. | `src/analysis.rs:669-676`, `src/smt.rs:629`, `spec/functional/FR-020-datatype-lowering.md` (Replay until FR-021) |
| FND-008 | low | An empty model returns `value_malformed` (an FR-011 reason) before the `sort_not_replayable` guard, although FR-020 says `sort_not_replayable` is checked before every FR-011 reason. | `src/analysis.rs:661-663` |
| FND-009 | low | Two `query_logic` scanner edge cases. (1) The repeat guard keys on `declared.is_some()`, so `(set-logic)` with no argument followed by `(set-logic QF_LIA)` is accepted as one logic with two `set-logic` commands. (2) An unterminated quoted symbol `\|...` runs to the end of input and is accepted, while an unterminated string is refused. Neither occurs in generated queries, and both are invalid SMT-LIB that a solver rejects. | `src/smt.rs` (`query_logic`, `b'\|'` arm and `set-logic` arm) |

## Dispositions

Round 1, verified against PR head 2c7a7a1 (fix commit 1fe1917, main merge 6a8e355).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `query_logic` scans the top-level commands, skipping comments, quoted symbols and strings. It counts the singular and plural datatype and codatatype heads, requires exactly one `set-logic` before every declaration, and requires balanced parentheses. `report.rs` requires declared == required == the report's `logic` field. A unit test covers each evasion. |
| FND-002 | fixed | `ordering_and_mixed_sorts_are_refused_at_statement_build` asserts `IllTypedExpression` for ordering and for Bool-against-enum. `lowering_refuses_unchecked_enum_ordering_and_mixed_sorts` drives both defensive branches over an unchecked expression and asserts `UnsupportedConstruct`. |
| FND-003 | fixed | The real-solver test now asserts `sat` from each engine, a non-conclusive differential for the sat fixture, and Agreement/Refuted for the unsat fixture. |
| FND-004 | fixed | The README names `QF_UFDTLIA` for enum queries and lists enum literals, references and equality as supported, with only enum ordering refused. |
| FND-005 | fixed | Main (#95) removed `Unreplayed`. The new `ReplayExpression::EnumUnreplayed` doc describes enum terms. |
| FND-006 | fixed | `EnumSortMap::constructors` is `Vec<EnumConstructor>` with named fields `variant` and `symbol`. |
