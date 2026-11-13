---
id: SR-17059
title: "Gap analysis of VER-91 range quantifier lowering against FR-032, FR-030, NFR-030, NFR-032, FR-005 and TC-030"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; src/quantifier.rs, src/smt.rs, tests/range_quantifier_lowering.rs, spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md, spec/functional/FR-030-bounded-quantifier-domains.md, spec/functional/FR-031-enum-and-collection-quantifier-lowering.md, spec/nonfunctional/NFR-030-quantifier-expansion-bound.md, spec/nonfunctional/NFR-032-quantifier-exhaustion.md, spec/functional/FR-005-evidence-cli.md, spec/interface/interface-001-analysis-api.md, spec/test/TC-030-bounded-quantifiers.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-032", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-032", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-030", type: reviews }
---
# Gap Analysis of VER-91 Range Quantifier Lowering

## Summary

Ticket: VER-91. PR agent-ix/quire-analyze#99. Plan completion: not assessed. `quoin matrix`
(quoin 0.30.1) lists every FR-030/031/032, NFR-030/031/032 and FR-005-AC-6 criterion as
`untagged`, while `quire trace --id FR-032-AC-1` resolves the `#[trace]` claims in
`tests/range_quantifier_lowering.rs`. The check below was therefore done by hand from
`quire trace` and the source. Real-solver tests were run with Z3 5.4.1 and cvc5 1.4.2 and
passed.

## Verdict

**FAIL (three medium).** Each of FR-032-AC-2, AC-3, AC-4, AC-5 and AC-6, NFR-030-AC-1 (range
part: 1,024/1,025 single, 1,000+24/25 across clauses, 32+992/1,056 nested), NFR-030-AC-2,
NFR-030-AC-3, FR-030-AC-2 (non-finite-bound part), FR-030-AC-3, FR-030-AC-4 (lowered-query
part), NFR-032-AC-1, NFR-032-AC-2 and FR-005-AC-6 (LIA/UFDTLIA part) has a tagged test that
asserts it and can fail. NFR-030-AC-4 holds by inspection: expansion loops only after
`expansion_cost` admits a count of at most 1,024, and the quantifier map gets one entry per
quantifier, not one per instance. The enum and collection parts of FR-030 and NFR-030-AC-1
belong to FR-031, and replay to FR-033, so they are outside this PR.

The mediums: the oracle half of FR-032-AC-1 is not built, the analysis-model input form is
specified nowhere, and FR-031's per-instance identity is not carried by range expansion.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-032-AC-1 requires the status from the FR-033 enumeration oracle to equal the recorded expected status. TC-030 step 3 says this runs everywhere, so the case can fail without solvers. No oracle exists: the tagged tests feed canned `sat`/`unsat` from a fake script and assert the script's own answer. Only the `#[ignore]`d real-solver test checks a hand-derived status, so without solvers the status half of AC-1 cannot fail. | `tests/range_quantifier_lowering.rs:841-867`, `tests/range_quantifier_lowering.rs:931-998` |
| FND-002 | medium | Spec gap: no artifact specifies the analysis-model range-quantifier input this PR makes public (`AnalysisFormula`, `RangeQuantifier`, `RangeBound` including `Unbounded`, `local_type`, `StatementInput::with_formula`, `LocalReference` resolution) or the `QueryBundle::quantifiers()` map. interface-001 `lowering_v2` still lists `quantification` as unsupported, names no quantifier map, and gives the logic as `QF_LIA` or "the feature set". FR-030 calls the range form only "the target design for the analysis model". `with_formula` replaces a checked clause body with an unchecked formula, while the assertion map and report still attribute the assertion to that clause and its `clauseDigest`. | `spec/interface/interface-001-analysis-api.md:68-75`, `src/smt.rs:314-322`, `src/quantifier.rs:1-80` |
| FND-003 | medium | FR-032 says a literal range is "expanded exactly as in FR-031". FR-031 requires each instance to keep the quantifier's source identity and the element, so a counterexample names the failing instance (FR-031-AC-5). Range expansion folds instances into plain `And`/`Or` replay, and a failure names only the assertion. Nothing records this as deferred to FR-033. | `src/smt.rs:1553-1604`, `spec/functional/FR-031-enum-and-collection-quantifier-lowering.md` |
| FND-004 | low | Spec ambiguity: NFR-030 "outermost first in canonical order" and FR-030 "canonical quantifier order" are not defined. The code plans depth-first pre-order per statement, in assertion-name order, so in `And(A{B}, C)` it charges B before C. A breadth-first reading charges C first, and the two readings give different encodings when the budget is contested. | `spec/nonfunctional/NFR-030-quantifier-expansion-bound.md`, `src/smt.rs:1366-1447` |
| FND-005 | low | Spec ambiguity: FR-030 "declared bounds of bounded integer inputs" can mean the input type's min/max constants. The code, and the FR-032 rule that a declared bound forces the bounded form, read it as the input's value, finite because its type is bounded. The spec should state that reading. No spec reason exists for an interim unreplayed bounded quantifier, so the code borrows `sort_not_replayable` (SR-17058 FND-001). | `spec/functional/FR-030-bounded-quantifier-domains.md`, `spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md`, `src/smt.rs:1451-1483` |
| FND-006 | low | NFR-030-AC-3's nested-product case never reaches the overflow branch of `checked_mul` in `expansion_cost`. The test's outer range is already bounded on its own, so the enclosing count stays 1. A width near 2^128 nested under an expanded outer of width 2 or more is untested. | `tests/range_quantifier_lowering.rs:561-591`, `src/quantifier.rs:183-187` |

## New findings (disposition pass 1)

Found in fix commit df9923b.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | low | FR-032-AC-8 is compound: it joins the interim replay reason and the statement-order independence of a contested budget, two unrelated behaviours. Its clause "a datatype-free query never reports a datatype reason" is vacuous after PR #98, which removed `sort_not_replayable`; the merge rightly dropped the matching test assert, so the clause has no test and can never fail. | `spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md:113` |

## Dispositions

Round 1, verified against fix commit df9923b and merge 225bf9a on the PR branch.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | `enumeration_oracle_decides_expanded_fixtures_without_a_solver` (src/smt.rs tests) runs seven expanded fixtures with no solver: forall, exists, both refuted, both empty, nested. An independent evaluator enumerates each range from the formula and is compared, at every x in -1000..=1000, with the lowered query's replay and with a hand-recorded expected status. It runs everywhere and can fail. |
| FND-002 | fixed | FR-032 now specifies the analysis-model input (`AnalysisFormula`, `RangeQuantifier`, `RangeBound`, `local_type`, `with_formula` and its refusals, `QueryBundle::quantifiers`) and the attribution to the host clause, with new FR-032-AC-7. interface-001 `lowering_v2` names the input, the quantifier map and `LIA`/`UFDTLIA`, and notes that only contract-IR quantifier nodes stay unsupported. |
| FND-003 | deferred | Deferred to FR-033 (VER-92). FR-032 now states that the per-instance identity and element of FR-031 and the counterexample naming a failing instance (FR-031-AC-5) come from FR-033 quantified replay. Until then the assertion map names the failing assertion and the quantifier map records the source identity. |
| FND-004 | fixed | FR-030, FR-032 and NFR-030 define canonical order: statements in ascending assertion-symbol order and, within a statement, depth-first pre-order with siblings left to right. A test pins the pre-order case. |
| FND-005 | fixed | FR-032 defines a declared bound as the input's value at the current observation, not its min or max. FR-011 adds the ninth reason `quantifier_not_replayable` as interim until FR-033. The reason counts agree: "nine" in FR-011 (twice), FR-020 and FR-021, with the list's precedence text updated to "the last three reasons". |
| FND-006 | fixed | `a_nested_count_that_overflows_is_bounded_not_wrapped` nests a width-(2^128 - 1) range under an expanded width-2 outer. The `expansion_cost_is_checked` unit test also covers the product and sum overflow branches directly. |
