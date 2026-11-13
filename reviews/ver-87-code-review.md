---
id: SR-17058
title: "Code review of enum, record and option witness replay"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/datatype_replay.rs, src/analysis.rs, src/smt.rs, src/lib.rs, tests/datatype_replay.rs, tests/enum_lowering.rs, tests/record_option_lowering.rs, spec/functional/FR-011-integer-witness-replay.md, spec/functional/FR-020-datatype-lowering.md, spec/functional/FR-021-datatype-witness-replay.md, README.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-021", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-011", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-020", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-020", type: reviews }
---
# Code Review of Enum, Record and Option Witness Replay

## Summary

Ticket: VER-87. PR agent-ix/quire-analyze#98, diff `origin/main...HEAD` (4 commits, 10 files).
Includes the rust-review lane (rust-style loaded). Focused runs with toolchain 1.98.1: lib 32
passed; `datatype_replay` 8 passed, 1 ignored; `enum_lowering` 15 passed, 1 ignored;
`record_option_lowering` 22 passed, 1 ignored; the three ignored real-solver tests passed with
Z3 5.4.1 and cvc5 1.4.2; `clippy --all-targets --all-features -D warnings` clean; `cargo fmt
--check` clean.

A probe query (enum, option and record over a non-parametric option sort, with the `some`
selector of a `none` value asserted equal to 7) was run on both solvers. Both print
`(define-fun x () R (k c_2 n))` style models: no `(model` keyword, no `(as ...)`
qualification, no parametric `(Option ...)` sort, and no selector interpretation.

## Verdict

**FAIL (three medium).** Soundness holds: a model is verified only after parse defects,
`symbol_missing`, integer and datatype range checks (the replacement for the unnamed FR-010
range and FR-020 range-predicate assertions, matched leaf for leaf, with `none` exempt as in
`(=> ((_ is some) x) ...)`) and every named assertion pass. `Unwrap` of `none` returns
`AbsentOption` and never reads a selector value (the assignment holds none), and the
implication, short-circuit `and` and `or` evaluators skip the guarded operand. Total `and`/`or`
also evaluate lazily, which gives the same verdict for every admitted expression because
`check_expression` admits no `Unwrap` whose proof comes from a total operator. Constructor
membership is checked against the query's symbol map. A foreign constructor, wrong arity, wrong
qualifying sort, abstract element, unknown or repeated or omitted symbol all fail closed with an
FR-011 reason in FR-011 precedence. `skip_model_term` is iterative and the 40,000-deep model is
rejected without stack overflow. Removal of `SortNotReplayable`, `DatatypeUnreplayed`,
`sort_symbols`, `datatype_sorts` and `datatype_variables` leaves no live reference (only
historical review files mention them). No paywalled text, roadmap, pin or SHA tracking,
compatibility layer or build artifact is in the diff.

The mediums: the parser accepts an invented `(Option S)` sort spelling that is not in the
query's symbol map; scalar positions map a declared constructor to `value_malformed` where the
new FR-021 bullet says `sort_mismatch`; and the depth-bound reason is stated three different ways
(AC-6, the new bullet, and the code/test), with the code's guard unreachable from model text.

Likely conflict zones with VER-91's concurrent `src/smt.rs` edits: the `ReplayExpression` enum
and its `evaluate` (about lines 770-830), the `QueryBundle` fields and accessors (about 860 and
915), the `QueryBundle` construction in `lower_statement_specs` (about 1178), `ValueReference`
lowering in `lower` (about 1308), `lower_comparison` (about 1540-1580) and `emit` (about
2100).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `sort_matches` accepts `ModelSort::Applied("Option", P)` for an option shape whenever `P` equals the payload sort, so `(as n_x (Option Int))` and a define-fun sort `(Option Int)` verify. The query declares only the non-parametric encoded sort, so `Option` is a name guessed from the source language, which FR-021 forbids ("never guessed from a source name"); neither Z3 5.4.1 nor cvc5 1.4.2 prints it. The test `nested_record_replays_from_both_spellings` asserts this spelling verifies. A side effect: an unknown symbol printed with an applied sort reports `value_malformed`, not `symbol_unknown`. Remove the `Applied` arm (and `ModelSort::Applied` if nothing else needs it) and make the test expect a failure. | `src/datatype_replay.rs:155-178`, `src/datatype_replay.rs:250-265`, `tests/datatype_replay.rs:416` |
| FND-002 | medium | In a Bool or Int position (a record field or an option payload), `parse_value` reads through `take_model_scalar`, so a declared constructor there (an enum constructor, `none`, or `(m_x 5)`) is `value_malformed`. The new FR-021 bullet says a constructor of another datatype is `sort_mismatch`. Because `value_malformed` is first in FR-011 precedence, the misclassification also changes which reason and symbol a multi-defect model reports. Classify a known constructor in a scalar position as `SortMismatch` and add a case to `values_that_do_not_match_the_declared_datatype_fail_replay`. | `src/datatype_replay.rs:276-291`, `spec/functional/FR-021-datatype-witness-replay.md:48-53` |
| FND-003 | medium | The over-deep reason is stated three ways. FR-021-AC-6 says "rejected as a limit breach". The new bullet says `value_out_of_domain` for "a value nested beyond the datatype depth bound". The code parses by declared shape, which lowering already bounds at depth 8, so the `depth > MAX_DATATYPE_DEPTH` guard cannot fire from any model text, and the 40,000-deep test asserts `value_malformed`. The only test of the guard calls the private `parse_datatype` with a depth argument. Make spec and code agree: state that a value deeper than its declared shape is `value_malformed` (AC-6 and the bullet), and either delete the unreachable guard and its unit test or label it defensive in the spec. | `src/datatype_replay.rs:303-305`, `src/datatype_replay.rs:580-597`, `tests/datatype_replay.rs:778-791`, `spec/functional/FR-021-datatype-witness-replay.md:74` |
| FND-004 | low | `verify_model` builds `datatype_values` with `filter_map`, and `DatatypeContext::witness` returns `None` on any shape/value mismatch, so a failed conversion silently drops the value from a model that is still published as verified. It cannot happen today (parse already validated the value), but it fails open. Return an error (or make `witness` infallible) instead of dropping. | `src/analysis.rs:790-803`, `src/datatype_replay.rs:482-522` |
| FND-005 | low | Stale doc comments: `datatype_shapes` is documented as "The symbols of datatype-sorted variables, in ascending order", and the deleted `datatype_sorts` left its line "Every datatype sort symbol the query declares, in ascending order." stacked on `enum_sorts`'s doc. | `src/smt.rs:917-923` |
| FND-006 | low | README still says "Models of datatype-sorted queries are not replayed yet and stay unverified.", which this PR makes false. | `README.md:57` |
| FND-007 | low | `literal_only_record_and_option_queries_are_replayed` expects both models (`flag` true and false) to verify, so no case shows replay rejecting a model through a record or option literal; an evaluator that wrongly returned true on those literals would still pass. Add a falsifying model, as the enum literal-only test does. | `tests/record_option_lowering.rs:621-630` |

## Coverage

Examined: every hunk of `src/smt.rs` (`ReplayAssignment::datums`, `ReplayFault::{AbsentOption,
IllTyped}`, `ReplayTerm::Datum`, `ReplayExpression::{Datum, Integer, IsPresent, DatumEqual}`,
`into_datum`, `into_item`, `replay_shape`, `QueryBundle::datatype_shapes`, the lowering arms
for value references, literals, field access, options, comparisons and `emit`), all of
`src/datatype_replay.rs`, the `src/analysis.rs` changes (`verify_model`, `parse_model`,
`record_value`, `take_datatype_value`, `take_model_scalar`, iterative `skip_model_term`,
`replay_assertions`, `MappedDatatypeValue`, `datatype_values()`), the public `WitnessValue`
export and `Display`, the three spec amendments, and every new or changed test. Rust lane: no
`unwrap`/`expect`/indexing on untrusted input outside tests, typed errors, no unsafe, no new
dependencies, exhaustive matches kept; `Field` evaluation clones the whole record per access,
acceptable under the NFR-020 bounds.

The stray-selector rule ("never reads the solver's value of the `some` selector on `none`") needs
no separate test: the assignment has no channel for a selector value, both solvers were observed
not to print one, and the unit test asserts `Unwrap` of `none` is `AbsentOption` rather than a
default.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | medium | The amended FR-021 bullet lists "a parametric spelling such as `(Option S)`" and any sort "that is not the symbol of the query's symbol map" under `sort_mismatch`, but `read_sort` returns `None` for any non-atom sort and `read_head` maps that to `value_malformed`, and a non-atom define-fun sort fails the whole model as `value_malformed`. The new test `invented_sort_spellings_fail_closed` accepts either reason, so it cannot catch the disagreement. Make the bullet say that a sort term that is not a single symbol is `value_malformed` (or map it to `sort_mismatch`), and assert the one exact reason. | `src/datatype_replay.rs:158-167`, `src/datatype_replay.rs:362-397`, `spec/functional/FR-021-datatype-witness-replay.md:52-54`, `tests/datatype_replay.rs:473-497` |

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `ModelSort` is now the atom only and `sort_matches` compares it with the shape's encoded sort symbol; the parametric arm is gone and the old positive test is replaced by a failing one (cf4c9e6). |
| FND-002 | fixed | A bare or `as`-qualified declared nullary constructor at a Bool/Int position is `SortMismatch`, tested in a record field and an option payload. The coder's limit (a constructor application there stays `value_malformed`) agrees with the amended bullet's "nests a constructor application beyond its declared shape" clause and is what makes over-deep text `value_malformed`. |
| FND-003 | fixed | The unreachable depth guard and its private-call unit test are deleted; FR-021-AC-6 and the bullet now state shape-driven `value_malformed`, matching the 40,000-deep and new depth-boundary tests. |
| FND-004 | fixed | `verify_model` returns `sort_mismatch` for a value `witness` cannot publish instead of dropping it. No test is acceptable: parse validates the value against the same shape, so no model text reaches the branch. |
| FND-005 | fixed | `datatype_shapes` doc corrected and the orphaned `datatype_sorts` line removed. |
| FND-006 | fixed | README now says datatype models are replayed (the stale integer sentence at line 87 was also removed). |
| FND-007 | still-open | The fix adds a bare `flag` assertion, so the `false` model now fails on that Boolean, not through a record or option literal. The literal assertions still hold for every `flag`, so an evaluator that returned true on them would still pass. Needs an assertion whose truth depends on a literal, for example `field(record(.. active: flag ..), "active")` alone. |
| FND-008 | still-open | New this round; not yet addressed. |
