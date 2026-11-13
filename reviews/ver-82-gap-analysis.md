---
id: SR-17053
title: "Gap analysis of VER-82 integer witness replay against FR-011, FR-010, FR-005-AC-6, NFR-004 and TC-017"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; src/analysis.rs, src/smt.rs, tests/integer_lowering.rs, spec/functional/FR-011-integer-witness-replay.md, spec/functional/FR-010-linear-integer-arithmetic.md, spec/functional/FR-005-evidence-cli.md, spec/nonfunctional/NFR-004-integer-exactness.md, spec/test/TC-017-integer-witness-replay.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-011", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-004", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-017", type: reviews }
---
# Gap Analysis of VER-82 Integer Witness Replay

## Summary

Ticket: VER-82. Plan completion: not assessed. Spec read from main (identical on
the PR branch). `quoin matrix --repo .` (quoin 0.30.1) reports every criterion of
the repository as untagged, including long-tagged ones such as FR-005-AC-6, so
it does not harvest this repository's `#[trace]` attributes; the AC-to-test
check below is manual over `#[trace(...)]` tags and test bodies.

FR-011-AC-1..11: each has at least one tagged test that can fail. AC-1 and
AC-11 have a recorded-model test plus the `#[ignore]`d real-solver test (Z3 and
cvc5 not installed; not run). AC-7 and AC-10 overflow ordering are unit tests
over directly constructed unchecked expressions, as the spec requires. AC-9
mutates the query text and checks the verdict for a verifying and a failing
value. FR-005-AC-6: the v2 report still validates for the verified integer
run (`replay_verified_models_agree`). NFR-004-AC-2 replay half is exercised by
the AC-7 test.

## Verdict

**PASS WITH GAPS.** No AC is unimplemented. One NFR-004 criterion assigned to
TC-017 has no integer test, and two spec lines need tightening.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Spec gap: FR-011 defines malformed only for numerals and sorts and says every non-verifying model gets exactly one reason naming a symbol or assertion; it does not say how a structurally unreadable model (empty, unbalanced, non-define-fun entry, quoted symbol, trailing tokens) is classified or that it names nothing. The PR's value_malformed with no subject is the only reading consistent with the closed set, so it is not a defect. Missing line: "A model text that is not a sequence of define-fun entries in the Z3 or cvc5 shape is value_malformed and names no symbol." | `spec/functional/FR-011-integer-witness-replay.md:24-28` |
| FND-002 | medium | NFR-004-AC-3 (TC-017) has no test: no integer-symbol model above the parsed-model ceiling is exercised; the existing model-bound tests in solver_adapters are Boolean and not tagged NFR-004-AC-3. | `tests/integer_lowering.rs` |
| FND-003 | low | The AC-7 overflow unit test also discharges the replay half of NFR-004-AC-2 but is not tagged NFR-004-AC-2. | `src/analysis.rs:1316` |
| FND-004 | low | FR-010-AC-7 still says the satisfiable fixture's model "stays unverified until replay exists ... and the comparison is not an agreement", while the real-solver test tagged FR-010-AC-7 now asserts Verified and Agreement; the clause is stale now that FR-011 replay exists. | `spec/functional/FR-010-linear-integer-arithmetic.md:96` |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | eed1092: FR-011 now states that a model text that is not a sequence of define-fun entries in the Z3 or cvc5 shape is value_malformed and names no symbol. Unit and integration tests assert reason value_malformed with subject None. |
| FND-002 | fixed | eed1092: `an_integer_model_above_the_model_ceiling_is_not_conclusive` (tagged NFR-004-AC-3) asserts ModelLimit, a non-conclusive ToolError and no verified model above a 64-byte ceiling, and sat within 4096. |
| FND-003 | fixed | eed1092: the overflow test is now tagged NFR-004-AC-2. |
| FND-004 | fixed | eed1092: FR-010-AC-7 and TC-016 now say the sat fixture's models replay-verify and the comparison agrees with status satisfied, matching FR-011-AC-11 and the real-solver test. |
