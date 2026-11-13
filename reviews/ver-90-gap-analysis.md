---
id: SR-17061
title: "Gap analysis of VER-90 enum-domain and collection quantifier lowering against FR-031, FR-030, NFR-030 and TC-030"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-analyze; src/quantifier.rs, src/smt.rs, tests/range_quantifier_lowering.rs, spec/functional/FR-031-enum-and-collection-quantifier-lowering.md, spec/functional/FR-030-bounded-quantifier-domains.md, spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md, spec/functional/FR-033-quantified-assertion-replay.md, spec/nonfunctional/NFR-030-quantifier-expansion-bound.md, spec/nonfunctional/NFR-031-quantifier-replay-bound.md, spec/nonfunctional/NFR-032-quantifier-exhaustion.md, spec/test/TC-030-bounded-quantifiers.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-031", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-032", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-030", type: reviews }
---
# Gap Analysis of VER-90 Enum-Domain and Collection Quantifier Lowering

## Summary

Ticket: VER-90. PR agent-ix/quire-analyze#100. Plan completion: not assessed. Test-to-criterion
bindings were read with `quire trace --id` (quoin 0.30.1), because `quoin matrix` misreads
`#[trace]` tags (PLAT-1672). Real-solver tests passed with Z3 5.4.1 and cvc5 1.4.2.

Ticket acceptance, re-measured. (1) Universal and existential enum fixtures are decided correctly
by both real solvers, and by the controlled oracle on every assignment. (2) An unbounded integer,
text, rational, unfixed or unencodable domain is refused as `UnsupportedConstruct`, naming its
reason class and source identity. (3) The SMT-LIB output equals fixed golden bytes, so it is
byte-identical across runs.

## Verdict

**FAIL (two medium, three low).** The following are backed by tests that can fail:

- FR-031-AC-1: enum goldens, and the oracle-vs-replay agreement over 56 assignments for 8 fixtures.
- FR-031-AC-2: the integer-collection order and duplicate rules.
- FR-031-AC-3, FR-031-AC-4 and FR-031-AC-6 (golden `domain-capture-v2.smt2`).
- FR-030-AC-1, FR-030-AC-2 (domain classes), FR-030-AC-3 (refusal nested two levels deep),
  FR-030-AC-5, and NFR-030-AC-1 (1,024/1,025; 1,000+24/25 shared with a range; 600x2 nested enum).

The independent `holds` oracle enumerates domains from its own declarations, not from the
lowering. A wrong join, a dropped instance or a wrong substitution would make
`verified_model()` disagree with it.

The gaps: enum and Boolean canonical order cannot be told apart from other orderings, and
FR-031-AC-5 is met by test-side computation rather than by product output.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-031-AC-2 requires a set and a bag to enumerate in canonical order: false before true, enum declaration order. No test can tell that order from another one. The `Role` fixture declares `Admin, Guest`, so declaration order is also alphabetical order. The Boolean and enum checks compare a `set` only with another `set`. A sort by variant name, or true before false, would pass every test. No `bag` of enum or Boolean elements is lowered at all. | `tests/range_quantifier_lowering.rs:159`, `tests/range_quantifier_lowering.rs:1873-1910` |
| FND-002 | medium | FR-031-AC-5 requires a counterexample for a failed universal to name the failing element and the source identity. No product output does this. The test finds the failing position with its own `holds` oracle, then indexes `entry.elements`, so it checks only the element order in the map. FR-032 assigns per-instance counterexamples to FR-033 replay. The FR-031 amendment ("the position of a failing instance names its element") assumes a position the analyzer never reports. | `tests/range_quantifier_lowering.rs:1917-1945`, `spec/functional/FR-031-enum-and-collection-quantifier-lowering.md:43-46` |
| FND-003 | low | `an_invalid_domain_formula_is_refused_when_attached` is tagged FR-031-AC-1, but it checks `with_formula` validation errors. AC-1 is about enum goldens and oracle status. No AC states the domain validation rules, so the binding is wrong (see SR-17060 FND-001). | `tests/range_quantifier_lowering.rs:2180-2241` |
| FND-004 | low | `real_z3_cvc5_decide_domain_fixtures` is tagged NFR-032-AC-3. That AC needs a fixture that a solver cannot decide to be reported `timeout`/`unknown`. This test asserts that every expanded fixture is decided `sat`/`unsat`. | `tests/range_quantifier_lowering.rs:2274-2302` |
| FND-005 | low | FR-030-AC-4 requires the empty-domain result in the lowered query, in replay and in the controlled-solver result. `an_empty_domain_is_true_for_universal_and_false_for_existential` checks only the query text and the instance count. | `tests/range_quantifier_lowering.rs:1991-2018` |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
| FND-004 | fixed | - |
| FND-005 | fixed | - |
