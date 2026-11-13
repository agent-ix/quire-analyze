---
id: SR-17059
title: "Gap analysis of VER-87 datatype witness replay against FR-021, FR-011, FR-020, NFR-020 and TC-020"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; src/datatype_replay.rs, src/analysis.rs, src/smt.rs, tests/datatype_replay.rs, tests/enum_lowering.rs, tests/record_option_lowering.rs, spec/functional/FR-021-datatype-witness-replay.md, spec/functional/FR-011-integer-witness-replay.md, spec/functional/FR-020-datatype-lowering.md, spec/nonfunctional/NFR-020-datatype-bounds.md, spec/test/TC-020-enum-datatype-analysis.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-021", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-011", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
---
# Gap Analysis of VER-87 Datatype Witness Replay

## Summary

Ticket: VER-87. PR agent-ix/quire-analyze#98. Plan completion: not assessed. `quoin matrix`
(quoin 0.30.1) lists every FR-021, FR-011, FR-020-AC-10 and NFR-020-AC-3 criterion as
`untagged`, including FR-011 criteria whose tagged tests are already on main, so the matrix does
not read this repository's `#[trace(...)]` tags. The mapping below was made by reading the tags.

## Verdict

**FAIL (one medium).** Every FR-021 criterion has a tagged test that can fail:
AC-1 (`nested_record_replays_from_both_spellings`, `recorded_real_solver_layouts_replay`,
`real_z3_cvc5_models_replay`, the enum and record replay tests), AC-2
(`values_that_do_not_match_the_declared_datatype_fail_replay`, Z3 and cvc5 layouts), AC-3
(enum, record, option present and absent, record nesting option and enum), AC-4 (three
assertion failures and two out-of-range parts, differential inconclusive), AC-5 (omitted,
unknown, repeated), AC-6 (40,000-deep model and model ceiling), AC-7 (unit and integration
laziness tests). FR-020-AC-10 is covered by the enum, record, option and literal-only tests;
FR-011's closed set is preserved (no new reason; `sort_not_replayable` removed everywhere). The
real-solver tests passed with Z3 5.4.1 and cvc5 1.4.2. The reason-mapping and depth
disagreements are filed in SR-17058 (FND-002, FND-003) and not repeated here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FR-021 bullet makes "an integer beyond 128 bits" inside a datatype value `value_out_of_domain`, and `parse_value` maps `ModelScalar::OutOfDomain` to it, but no test puts a numeral beyond the 128-bit range (for example 2^127) in a record field or option payload. Add one case to the AC-2 or AC-4 test. | `src/datatype_replay.rs:284-290`, `spec/functional/FR-021-datatype-witness-replay.md:51-52`, `tests/datatype_replay.rs:532-679` |
| FND-002 | low | NFR-020's metric table requires an exact boundary test for nesting depth "in lowering and in model parsing". Lowering has depth-8 tests; model parsing has none: no model whose value is nested to depth 8 is shown to verify. Add a depth-8 model value that replays. | `spec/nonfunctional/NFR-020-datatype-bounds.md:27`, `tests/datatype_replay.rs:773-804` |

## Coverage

Examined criteria: FR-021-AC-1 through AC-7 and every FR-021 Behavior bullet (including the new
reason-mapping bullet), FR-011 Typed failure and precedence and the amended Datatype-sorted
symbols bullet, FR-020 "Replay of datatype models" and FR-020-AC-10, NFR-020-AC-3 and the depth
metric, and the TC-020 procedure. Code with no owning requirement: none; the new public items
(`WitnessValue`, `MappedDatatypeValue`, `VerifiedBooleanModel::datatype_values`) implement the
FR-021 published-values bullet. No stubs, no `todo!`, no coverage inflation: each changed test
asserts a reason or a verified value that a wrong implementation would change, except the
literal-only record/option test filed as SR-17058 FND-007.
