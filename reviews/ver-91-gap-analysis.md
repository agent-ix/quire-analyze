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
