---
id: SR-17056
title: "Code review of record and option SMT datatype lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/smt.rs, src/analysis.rs, tests/record_option_lowering.rs, tests/smt_lowering.rs, tests/golden/record-sat-v2.smt2, tests/golden/option-sat-v2.smt2, README.md, spec/functional/FR-020-datatype-lowering.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
---
# Code Review of Record and Option SMT Datatype Lowering

## Summary

Ticket: VER-86. PR agent-ix/quire-analyze#97, diff `origin/main...HEAD` (4 commits, 8 files).
Includes the rust-review lane (rust-style loaded). Focused runs with toolchain 1.98.1:
lib 29 passed; `record_option_lowering` 20 passed, 1 ignored (real solvers, z3/cvc5 not
installed); `smt_lowering` 12 passed; `package_translation` 17 passed;
`clippy --all-targets --all-features -D warnings` clean.

Probes run on a scratch copy (not the branch): a record `A` of 64 fields of record `B`, of 64
fields of record `C`, of 64 integer fields (3 datatypes, depth 3, 64 fields, all inside NFR-020)
is refused `ResourceLimit` at `type.ranges`; `A` of 64 fields of `C` of 64 integer fields (2
datatypes) lowers to a 1,947,463-byte query. The depth-128 `package_translation` test overflows
at `RUST_MIN_STACK` 1.25 MiB and passes at 1.5 MiB on both this branch and `origin/main`, so the
helper split restores the main headroom.

## Verdict

**FAIL (one medium).** Record and option lowering matches FR-020: one-constructor records with
selectors in ascending field-name order, one option datatype per payload sort, permitted field
types, nested text/collection/rational refused as `UnsupportedType`, `=`/`distinct` over records
and options, dependency-ordered declarations with ties by encoded symbol, and byte-identical
output under permutation. `Unwrap` as the bare `some` selector is sound: `check_expression`
admits it only where the `some` tester is the antecedent of an implication or the left operand of
a short-circuit `and`/`or`, and SMT `=>`, `and` and `or` take the same value as the
short-circuit form whatever the selector holds on `none`. Unguarded `Unwrap` is refused upstream
(tested). The guarded payload range `(=> ((_ is some) v) range)` is correct and composes through
nested records and options. The symbol digests use the same domain-separated, length-prefixed
`hash_fields` identity encoding as the merged enum symbols; they are identity encodings, not
pins or tracking records. `record_sorts()`/`option_sorts()` are ascending by sort symbol.
`sort_not_replayable` covers every datatype sort with the stated subject rule, and
`query_logic` still derives `QF_UFDTLIA` from `declare-datatypes`. The
`capability_contract_is_explicit` change (`record` to `text`) is legitimate. The `lower()` split
does not change behavior. The real-solver test is `#[ignore]`d and was not run. No paywalled
text, roadmap, ceremony, compatibility layer or build artifact is in the diff.

The medium: the range expansion grows with the product of field counts, so in-bound types are
refused under a bound NFR-020 does not define.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Range constraints for a datatype-sorted variable get one assertion per integer leaf, so the count is the product of field counts along each path. A type inside every NFR-020 bound (3 records x 64 fields, depth 3) is refused `ResourceLimit` `type.ranges`, a bound NFR-020 does not list. Two records of 64x64 integer leaves emit about 1.9 MB of range text, which contradicts the NFR-020 claim that worst-case declaration text stays well under 16 MiB. | `src/smt.rs:1817-1880`, `src/smt.rs:2056-2064` |
| FND-002 | low | `reference()` rebuilds a variable's range constraints on every reference and overwrites the map entry, walking up to 100,000 nodes each time, though the result depends only on the symbol's type. | `src/smt.rs:2056-2064` |
| FND-003 | low | An integer `FieldAccess` or `Unwrap` inside an integer term is counted twice against `MAX_EXPRESSION_NODES`: once by `fold_integer`'s `count_node` and again when `emit` calls `lower` on the stored expression. The bound is over-counted (conservative, never unsound). | `src/smt.rs:2390-2405`, `src/smt.rs:1926-1933` |
| FND-004 | low | For an option-typed value the `UnsupportedType` path is `type.option.some`, which names neither the value declaration nor any type. FR-020 Refusals requires the path to name the declaration and the nested field path. | `src/smt.rs:1770` |

## Coverage

Examined: `DatatypeSet::declarations`/`sort_symbols`, `RecordSortMap`, `OptionSortMap`,
`record_sort`, `option_sort`, `sort_of`, `range_constraints`, `option_map`,
`lower_record_literal`, `lower_field_access`, `lower_option_*`, `lower_comparison`,
`comparable_sort`, `lower_term`, `term_type`, `datatype_integer_type`, the `fold_integer`
datatype arm, `IntegerTree::Datatype` emission, `reference`, `verify_model` in `src/analysis.rs`,
`query_logic`, the capability contract, both goldens, and every test in
`tests/record_option_lowering.rs` and the two new unit tests. Rust lane: no `unwrap` outside
tests, indexing `records[&sort]` is guarded by a preceding insert, errors are typed, the
exhaustive `ExpressionKind` match is kept.
