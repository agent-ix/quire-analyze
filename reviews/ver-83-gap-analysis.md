---
id: SR-17053
title: "Gap analysis of FR-012 integer analysis_claim translation"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze#93; spec/functional/FR-012-integer-claim-translation.md, spec/functional/FR-009-package-runner.md, spec/functional/FR-010-linear-integer-arithmetic.md, spec/nonfunctional/NFR-004-integer-exactness.md, spec/test/TC-018-integer-package-claims.md, src/package_translation.rs, tests/integer_translation.rs, tests/package_runner.rs"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-012", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-009", type: references }
  - { target: "ix://agent-ix/quire-analyze/NFR-004", type: references }
  - { target: "ix://agent-ix/quire-analyze/TC-018", type: references }
---
# Gap analysis of FR-012 integer analysis_claim translation

## Summary

Ticket: VER-83. This gap analysis covers the PR #93 diff against FR-012, FR-009 as amended,
NFR-004 and TC-018 on main, and checks the upstream IR rules in quire-contract-model at the
locked revision. The headline is a spec gap that is owned upstream. The IR gives every numeric
operand one complete integer type, and the reject policy proves each result inside that type's
bounds. Under that rule, FR-012's mapping cannot check any addition, subtraction or non-unit
product over a parameter whose range is not a single point. FR-012-AC-1's own example
`dose <= 5 * weight` is one such product.

## Verdict

**FAIL**. One high spec gap, plus medium gaps in the spec and in test coverage. The PR's code is
faithful to the FR-012 Terms table, the linearity rule, the refusals, the bounds and the
determinism rule. Its signed-domain and reject-policy choice is sound. The gaps sit mostly in
FR-012 and upstream, not in this PR's code.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-012 does not say how integer terms of different package ranges get one IR type. The IR requires equal integer types for numeric and comparison operands, returns the operand type, and under reject refuses a result that may leave its bounds. So `x + c`, `x + y` and `c * x` (with the absolute value of `c` above 1) over any range other than a point are refused in `check_expression` and named as `claim-N`, not as a node. FR-012-AC-1's example cannot be built. The PR's fixtures avoid this with point ranges, factor 1, and `dose <= max`. | spec/functional/FR-012-integer-claim-translation.md; src/package_translation.rs:515-532 |
| FND-002 | medium | The FR-012 Parameters bullet takes "domain, minimum, maximum and overflow policy" from the `integer_range`, but an `integer_range` carries only `min` and `max`. FR-012 never names the domain or the policy. The PR fixes Signed and Reject, which is sound, but the spec does not say so. | spec/functional/FR-012-integer-claim-translation.md; src/package_translation.rs:526-532 |
| FND-003 | medium | A literal of the bare `scalar_type/integer` type is given the full 128-bit signed range. No requirement owns that rule, upstream `type_range` treats a bare integer as unbounded, no test exercises it, and such a literal can never be combined with an `integer_range` operand. | src/package_translation.rs:521 |
| FND-004 | medium | FR-009-AC-25 states that no admitted package reaches `check_expression` errors. That is no longer true for integer claims: an admitted `a + 1` over `[0,100]` exits 2 naming `claim-1`. | spec/functional/FR-009-package-runner.md:323 |
| FND-005 | medium | FR-012-AC-8 has no test at all, not even one skipped where the solvers are absent, although TC-018 asks for a real-solver repeat that is skipped visibly. | spec/functional/FR-012-integer-claim-translation.md; tests/package_runner.rs |
| FND-006 | medium | The FR-012-AC-1 fixture is `dose <= max` over two parameters, with no weight and no arithmetic. It does not bound a dose by a maximum for a weight, and it cannot, because of FND-001. | tests/package_runner.rs:1366-1400 |
| FND-007 | low | FR-012-AC-3 requires exit 2 with no report. The product and cycle refusals are asserted only on the translation library, and only `div` goes through the spawned runner. | tests/integer_translation.rs:233-330; tests/package_runner.rs:1023-1041 |

## Coverage

- Plan completion: not assessed
- Matrix: `quire matrix --scope .` (quire 0.44.0) reports every criterion in the repository as
  `untagged`, FR-009 and FR-010 included, with a `criterion-tagged-outside-row` warning on each
  `#[trace]` tag. This is a repo-wide model condition that predates this PR, so the PR scope
  falls back to a grep index of `#[trace]` tags. By that index, FR-012-AC-1 through AC-7 are
  tagged and AC-8 is untagged.
- Run evidence: on HEAD, 8 tests pass and 6 are ignored in integer_translation, and 2 are
  ignored in package_runner. On a throwaway local merge of PR #92, 13 of 14 pass in
  integer_translation (FND-001 of SR-17052 is the failure). `contradictory_dose_claims_are_refuted`
  passes, and `a_dose_bounded_by_a_maximum_runs_end_to_end` exits 3.
- Semantic review: run, scoped to FR-012 at the caller's request.
- Real solvers: Z3 and cvc5 are absent, so AC-8 is not assessable.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-012 gains an "Integer types of terms" bullet covering parameter, range literal, bare-integer literal and Application typing, with `result_type` used only to tell integer from Boolean. It also states the upstream limitation plainly: wider-range arithmetic exits 2 naming `claim-N`, and the fix needs a quire-contract-ir capability. The AC-1 example is now admissible: `max <= 1 * weight` passed `check_expression` and reached both solvers with #92 merged. |
| FND-002 | fixed | The bullet fixes Signed, `min`/`max` and Reject, and notes that an `integer_range` carries only `min` and `max`. |
| FND-003 | fixed | The new FR-012-AC-9 owns the rule for bare-integer literals, and `a_literal_of_the_bare_integer_type_has_the_signed_128_bit_range` passes with #92 merged. |
| FND-004 | fixed | FR-009-AC-25 now states that an integer claim can reach IR errors and exits 2 naming `claim-N`. |
| FND-005 | fixed | A gated FR-012-AC-8 test was added and skips with an eprintln when the executables are absent, as the FR-009-AC-14 test does. It cannot yet pass where solvers exist: see SR-17052 FND-004. |
| FND-006 | fixed | The fixture now bounds dose by max and max by `1 * weight`, the form the limitation admits, and AC-1 is reworded to match. |
| FND-007 | fixed | `integer_refusals_exit_two_naming_the_node_through_the_runner` spawns the runner for the product, div/rem/mod and cycle cases and asserts exit 2 and no report. Its node naming is weak: see SR-17052 FND-005. |
