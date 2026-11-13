---
id: SR-17057
title: "Gap analysis of VER-86 record and option lowering against FR-020, NFR-020, FR-010, FR-005 and TC-020"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; src/smt.rs, src/analysis.rs, tests/record_option_lowering.rs, spec/functional/FR-020-datatype-lowering.md, spec/nonfunctional/NFR-020-datatype-bounds.md, spec/functional/FR-010-linear-integer-arithmetic.md, spec/functional/FR-005-evidence-cli.md, spec/test/TC-020-enum-datatype-analysis.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-010", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-005", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
---
# Gap Analysis of VER-86 Record and Option Lowering

## Summary

Ticket: VER-86. PR agent-ix/quire-analyze#97. Plan completion: not assessed. `quoin matrix`
(quoin 0.30.1) lists every FR-020 and NFR-020 criterion as `untagged`, enum ones on main
included, while `quire trace --id FR-020-AC-4` resolves the `#[trace]` claims in
`tests/record_option_lowering.rs`. So the criterion-to-test check was done by hand from
`quire trace` and the source. Each of FR-020-AC-3, AC-4, AC-5, AC-6, AC-7, AC-9 and AC-10
(record and option parts), NFR-020-AC-1 (64/65 fields, 8/9 depth over records, options and a
mixed chain, 256/257 datatypes over records and a mixed record/option set) and NFR-020-AC-2 has at
least one tagged test that asserts it and can fail. The real-solver half of AC-3 is in an
`#[ignore]`d test, as TC-020 Limitations allows.

## Verdict

**FAIL (one medium).** The code has behavior that the spec does not state: unnamed range
assertions over the integer parts of datatype-sorted variables. They are needed for soundness:
without them a solver can choose `level = -5` and return a false counterexample. But FR-020 and
FR-010 do not specify them, and their position does not follow the FR-010 block rule. The two
low findings are spec text that has fallen behind the code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The spec does not cover the datatype range assertions. FR-010 requires a range "for every integer symbol" in one fixed block "in ascending symbol order". The PR also asserts ranges over record-field and option-payload selectors, with the payload guarded by the `some` tester. These are emitted after the integer block, not merged into it. A probe saw datatype variable `v_s_7746...` emitted after integer `v_s_b0ff...`. FR-020 has no text for any of this. Missing text for FR-020 Encoding: "For every datatype-sorted variable the query asserts the declared range of each integer part, as an unnamed assertion outside the assertion map and report `assertions[]`: a record field through its selector, and an option payload through the `some` selector guarded by `(=> ((_ is some) v) ...)`. These assertions follow the FR-010 integer range block, ordered by ascending variable symbol and, within a variable, by ascending field name, depth first." Update the README sentence on declared ranges to match. | `src/smt.rs:1068-1081`, `spec/functional/FR-020-datatype-lowering.md:56-62`, `spec/functional/FR-010-linear-integer-arithmetic.md:54-60` |
| FND-002 | low | FR-020-AC-10 still covers only enums: "a query declaring an enum datatype ... naming the first enum variable". The PR widened the Replay-until-FR-021 parenthetical to every datatype and tags its record and option replay tests FR-020-AC-10. So AC-10 does not state the behavior those tests claim. Change it to "a datatype (enum, record or option)" and "the first datatype-sorted variable". | `spec/functional/FR-020-datatype-lowering.md:149` |
| FND-003 | low | FR-020 says `RecordLiteral` "lowers to the constructor applied to the field values in declared field order", and Encoding says fields are "emitted in ascending name order". "Declared field order" can be read as the environment's order, which the permutation rule (AC-7) forbids. The code uses ascending name order. The Expressions bullet should say "in ascending field-name order". | `spec/functional/FR-020-datatype-lowering.md:93` |

## Coverage

Examined FR-020 (all Behavior sections, AC-2 to AC-7, AC-9, AC-10), NFR-020 (Statement,
Definitions, AC-1, AC-2), FR-010 Declared domain, FR-005-AC-6 (unnamed ranges stay out of
`assertions[]`: they are emitted as plain `assert`s and the record and option reports validate)
and TC-020. Context only: FR-020-AC-1 and AC-8 (enum and collection, on main), NFR-020-AC-3
(model parsing, VER-87).
