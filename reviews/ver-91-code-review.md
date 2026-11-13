---
id: SR-17058
title: "Code review of bounded integer range quantifier lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/quantifier.rs, src/smt.rs, src/lib.rs, schemas/differential-report-v2.schema.json, tests/range_quantifier_lowering.rs, tests/golden/range-expanded-v2.smt2, tests/golden/range-bounded-v2.smt2"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-032", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-032", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-030", type: reviews }
---
# Code Review of Bounded Integer Range Quantifier Lowering

## Summary

Ticket: VER-91. PR agent-ix/quire-analyze#99, diff `origin/main...HEAD` (2 commits, 7 files).
Includes the rust-review lane (rust-style loaded). Focused runs with toolchain 1.98.1: lib 31
passed; `range_quantifier_lowering` 19 passed, 2 ignored; `smt_lowering` 12, `enum_lowering`
15, `integer_lowering` 22, `record_option_lowering` 22 and `analysis_semantics` 9 passed, all
goldens unchanged. The two ignored real-solver tests passed with Z3 5.4.1 and cvc5 1.4.2
(`QUIRE_Z3`, `QUIRE_CVC5` set). `clippy --all-targets --all-features -D warnings` is clean.

## Verdict

**FAIL (three medium).** The encodings are sound. Literal expansion iterates `lower..=upper`
inclusive (correct at `i128::MAX`). An empty range gives `true` for forall and `false` for
exists. The bounded form is `(forall ((bv_k Int)) (=> (and (<= lo bv_k) (<= bv_k hi)) body))` and
`(exists ((bv_k Int)) (and lo<=bv_k bv_k<=hi body))`, with `bv_k` numbered by bounded nesting
depth. That prefix collides with no other symbol family (`v_`, `s_`, `c_`, `r_`, `o_`, `a_`).
Locals resolve innermost-first, so shadowing works, and `LocalReference` and `ValueReference`
cannot capture each other. Budget accounting follows NFR-030. Instance count is width x the
enclosing expanded count, checked `u128` in `expansion_cost`, and `used + count <= 1024`. A
bounded quantifier is charged zero. `literal_width` returns `None` for a 2^128 width. Statements
are planned in sorted assertion-name order. That changes no golden and no assertion-map order:
the map and the errors were already sorted, and `variables`/`datatypes` are BTreeMaps with sorted
origins. The declared-bound check admits only integer inputs, and every `IntegerType` has finite
bounds. One refused quantifier at any depth refuses the request. The logic is chosen from
`quantifiers.bounded`, which is set only when a `forall`/`exists` is actually emitted, so a bounded
entry under an empty expanded range keeps `QF_LIA`. The `query_logic` scanner detects only an atom
right after `(`, skipping comments, `|...|` symbols and strings, and the validator plus the schema
enum accept `LIA`/`UFDTLIA` exactly as FR-005-AC-6 states. Boolean short-circuit replay over an
unreplayed quantifier is sound. A timeout or `unknown` on a quantified query stays inconclusive.
No paywalled text, roadmap, pin/SHA ceremony, compat layer or build artifact is in the diff.

The mediums: the interim unreplayed-quantifier path reuses the datatype fault, and with it the
`sort_not_replayable` reason, which PR #98 deletes. The real-solver timeout test accepts
`ToolError`. A test is tagged to FR-031-AC-6 without testing it.

PR #98 (VER-87) conflict zones in `src/smt.rs`, for FND-001:
1. `ReplayFault` (line 738): #98 removes `DatatypeUnreplayed` and adds `AbsentOption` and
   `IllTyped`. This PR needs its own fault variant there, for example `QuantifierUnreplayed`.
2. `ReplayExpression` (line 794-810): #98 replaces `DatatypeUnreplayed` with
   `Datum`/`Integer`/`IsPresent`/`DatumEqual`. This PR's `QuantifierUnreplayed` sits next to it,
   a textual conflict.
3. `ReplayExpression::evaluate` (line 834): the arm
   `Self::DatatypeUnreplayed | Self::QuantifierUnreplayed => Err(ReplayFault::DatatypeUnreplayed)`
   must become `Self::QuantifierUnreplayed => Err(ReplayFault::<own variant>)`.
4. `emit` (line 2424): `IntegerTree::Bound(symbol) => (symbol.clone(), ReplayTerm::DatatypeUnreplayed)`.
   #98 removes `ReplayTerm::DatatypeUnreplayed` and rewrites the adjacent `Datatype` arm. `Bound`
   needs a replacement `ReplayTerm` variant or the quantifier fault.
5. `lower_term` (line 1958) and `lower_integer_comparison` (line 2355): this PR adds `Resolvers`
   and #98 changes the replay these functions return, an overlapping textual edit.
6. Outside smt.rs: the `src/analysis.rs:772` mapping `DatatypeUnreplayed => SortNotReplayable`
   and the `verify_model` datatype guard are both removed by #98. The new fault needs its own
   `ReplayFailureReason` mapping, and no spec defines one (see SR-17059). After #98 a `UFDTLIA`
   bounded query is no longer caught by the datatype guard, so it depends on that new fault alone.
   `tests/range_quantifier_lowering.rs:925-928` asserts `SortNotReplayable`, which #98 deletes.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `ReplayExpression::QuantifierUnreplayed` evaluates to `ReplayFault::DatatypeUnreplayed`, so a datatype-free `LIA` bounded query reports `sort_not_replayable` naming an assertion. FR-020 defines that reason only for datatype sorts, naming a datatype symbol. `IntegerTree::Bound` also emits `ReplayTerm::DatatypeUnreplayed`. Both names, the reason and the test that asserts it are removed by PR #98, so whichever PR lands second fails to compile. | `src/smt.rs:834`, `src/smt.rs:2424`, `src/analysis.rs:772`, `tests/range_quantifier_lowering.rs:925-928` |
| FND-002 | medium | `real_solver_timeout_on_a_quantified_query_is_inconclusive` accepts `AnalysisStatus::ToolError`. NFR-032-AC-3 requires `timeout` or `unknown`, so the test passes on an adapter fault. It also runs only Z3, though the AC covers both Z3 and cvc5. | `tests/range_quantifier_lowering.rs:1005-1021` |
| FND-003 | medium | `lowering_is_deterministic_and_inner_quantifiers_shadow` is tagged FR-031-AC-6. That AC needs a free variable, named like a nested quantifier's bound variable, to stay uncaptured, with distinct symbols shown in a golden. The test checks local-over-local shadowing with substring asserts and no golden, which inflates FR-031-AC-6 coverage. | `tests/range_quantifier_lowering.rs:694-722` |
| FND-004 | low | `capability_contract_lists_the_range_quantifier` is tagged FR-030-AC-1, but it only checks a string in `exact_constructs`, not a quantifier-map entry. | `tests/range_quantifier_lowering.rs:736-742` |
| FND-005 | low | The order-independence check reverses a fixture whose quantifiers both fit the budget, and `pair(25)` counts the bounded entries without saying which one. A contested budget under reordered statements, the case the canonical sort exists for, is not asserted. | `tests/range_quantifier_lowering.rs:431-456`, `tests/range_quantifier_lowering.rs:696-704` |
| FND-006 | low | Dead or misleading code. `bound_term`'s `Unbounded` arm is unreachable after `require_finite` and builds an error without the refusal. `lower_planned` has `unreachable!`. `saturating_add(count)` and `enclosing.saturating_mul(literal_width(..).unwrap_or(1))` saturate values already checked to be at most 1,024; the inner enclosing count is just `count`. | `src/smt.rs:1425`, `src/smt.rs:1435-1439`, `src/smt.rs:1496`, `src/smt.rs:1643-1652` |
| FND-007 | low | `AnalysisFormula::canonical` builds statement identity text from `{:?}` Debug output, which is not a stable encoding. It also uses `serde_json::to_string(..).unwrap_or_default()`, so a serialization failure collapses distinct terms to one identity. | `src/quantifier.rs:85-112` |
| FND-008 | low | Formula nodes count twice against `MAX_EXPRESSION_NODES`, once in `plan` and again in `lower_planned`/`lower`, so the node bound is over-counted for a range formula (conservative, never unsound). | `src/smt.rs:1366-1373`, `src/smt.rs:1485-1494` |
| FND-009 | low | `RangeQuantifier::local_type` is never checked against the range. A range of 0..300 over a local type of 0..255 lowers elements outside the bound variable's type, and the bounded `bv_k` is limited only by the guard. The field feeds only the saturating check. | `src/smt.rs:1400-1447`, `src/quantifier.rs:58-67` |

## Dispositions

Round 1, verified against fix commit df9923b and merge 225bf9a (origin/main with PR #98) on the PR branch.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `ReplayFault::QuantifierUnreplayed`, `ReplayTerm::QuantifierUnreplayed` and `ReplayExpression::QuantifierUnreplayed` are separate variants. `replay_assertions` maps the fault to the new `ReplayFailureReason::QuantifierNotReplayable` (`quantifier_not_replayable`). After the merge no `DatatypeUnreplayed` or `SortNotReplayable` name remains, and the quantifier path does not touch datatype replay. A `UFDTLIA` test checks that a replayable enum model fails only on the quantifier. |
| FND-002 | fixed | The timeout test now loops over Z3 and cvc5 and accepts only `Timeout` or `Unknown`. |
| FND-003 | fixed | The FR-031-AC-6 tag is removed from the shadowing test. The new `a_free_variable_named_like_a_bound_variable_is_not_captured` checks input `i` against bound `i` in both forms, against the golden `range-free-variable-v2.smt2`. |
| FND-004 | still-open | The capability-contract test was retagged from FR-030-AC-1 to FR-032-AC-1. It still checks only a string in `exact_constructs`, not the golden or the oracle status that FR-032-AC-1 states. The binding is still wrong; a fitting tag is FR-010-AC-1 (TC-016), or no AC tag at all. |
| FND-005 | fixed | `a_contested_budget_is_spent_in_canonical_order` lowers a 1,000 + 25 contested pair in both statement orders and asserts the same query and which quantifier is bounded. It also asserts the pre-order case A(100), B(500), C bounded. |
| FND-006 | fixed | `Planned` is split into `Expanded`/`Bounded`. `FiniteBound` removes the unreachable `Unbounded` arm and `unreachable!`. The inner enclosing count is `count`, and `used += count` carries a comment that it is bounded. |
| FND-007 | fixed | `identity_text` is a serde_json value tree with explicit tags. A serialization error now refuses `with_formula` as `InvalidStatement` instead of collapsing to an empty string. |
| FND-008 | fixed | `plan` counts only structural nodes and returns `Term` uncounted; `lower_planned` no longer counts, so each node is counted once. |
| FND-009 | fixed | `validate_formula` refuses a non-empty literal range, or a declared bound whose input type lies outside `local_type`, as `InvalidStatement`. Tested in `an_invalid_formula_is_refused_when_attached`. |
