---
id: SR-17060
title: "Code review of enum-domain and finite-collection quantifier lowering"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/quantifier.rs, src/smt.rs, tests/range_quantifier_lowering.rs, tests/golden/enum-domain-v2.smt2, tests/golden/collection-domain-v2.smt2, tests/golden/domain-capture-v2.smt2, spec/functional/FR-030-bounded-quantifier-domains.md, spec/functional/FR-031-enum-and-collection-quantifier-lowering.md"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-031", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/FR-032", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/NFR-030", type: reviews }
  - { target: "ix://agent-ix/quire-analyze/TC-030", type: reviews }
---
# Code Review of Enum-Domain and Finite-Collection Quantifier Lowering

## Summary

Ticket: VER-90. PR agent-ix/quire-analyze#100, diff `origin/main...HEAD` (1 commit, d9d503e, 8
files). Includes the rust-review lane (rust-style loaded). Focused runs with toolchain 1.98.1 and
`--include-ignored`: lib 34 passed; `range_quantifier_lowering` 42 passed (real-solver tests
included); `smt_lowering` 12, `enum_lowering` 16, `integer_lowering` 23 and
`record_option_lowering` 23 passed. Real solvers observed: Z3 5.4.1 and cvc5 1.4.2.
`clippy --all-targets --all-features -D warnings` is clean.

## Verdict

**FAIL (one medium, two low). No code soundness defect.** The expansion is sound. `lower_instances` joins
instances with `and` for a universal and `or` for an existential. An empty domain lowers to `true`
for forall and `false` for exists, with replay matching. One instance lowers to itself. Set and bag
sort by a declaration-position key (false=0/true=1, enum position, integer value) with a stable
sort, so a bag keeps every occurrence. Set and ordered_set collapse a repeat to its first
occurrence through a `BTreeSet` on that key. Sequence and ordered_set keep written order. The
statement identity text normalizes the same way, so two spellings with the same members share one
assertion symbol and one query, and the collection kind stays in the identity.

The budget is shared with ranges through `expansion_cost` (checked `u128` product and sum, at most
1,024). A domain that does not fit is refused as `expansion-ceiling` and is never bounded. A
domain inside a bounded range counts from its own width. Refusals fire at any depth: the body is
planned even under an empty domain, so an inner refusal is never skipped. Bound variables resolve
innermost-first, so an inner quantifier that reuses a name shadows the outer one. A
`ValueReference` of the same name is a different expression kind and cannot be captured. The
golden `domain-capture-v2.smt2` shows distinct symbols. Mis-sorted uses of a Boolean or enum
local (ordering, arithmetic, a different enum sort) fail as `unsupported` and are never coerced.
`is_integer_term` now reads the local's sort, not just its presence. The FR-030 and FR-031
amendments match the code: the map records range bounds, and for an enum or collection it records
each instance's element in instance order. No paywalled text, roadmap, pin/SHA/version ceremony,
compatibility layer or build artifact is in the diff. The golden symbol hashes are canonical
symbol names, not pins.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Not all the spec was amended for the new public analysis-model input. In interface-001, `range_quantifier_input` lists only `AnalysisFormula, RangeQuantifier, RangeBound`, `quantification_note` says only "the range quantifier input above is admitted", and the quantifier map is `..._instances_and_bounds` with no elements. FR-032 says the formula nodes are `Not`, `And`, `Or`, `Implies` and `Range`. `DomainQuantifier`, `QuantifierDomain`, `DomainElement`, `CollectionKind`, `UnboundedDomain`, the new `QuantifierEntry` shape (`bounds` + `elements`) and the `with_formula` domain checks are specified nowhere. | `spec/interface/interface-001-analysis-api.md:73-75`, `spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md:37`, `src/quantifier.rs:133-184` |
| FND-002 | low | `validate_domain` checks the bound variable's type only against the collection's elements, so an empty collection is admitted whatever its `local_type` (text, record, option). FR-030 limits finite-collection element sorts to Boolean, enum and bounded integer, and a Text-typed empty collection should be refused, not lowered to `true`. | `src/smt.rs:3455-3480` |
| FND-003 | low | `QuantifierEntry` stores a two-way choice as two independent fields: `bounds: Option<(RangeBound, RangeBound)>` and `elements: Vec<String>`. A range with elements, or an enum with bounds, can be represented. `elements` is also a string list even though the typed public `DomainElement` exists. One `enum` (range bounds, or `Vec<DomainElement>` members) would make the invalid states unrepresentable and keep element types. | `src/quantifier.rs:330-342` |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | - |
| FND-002 | fixed | - |
| FND-003 | fixed | - |
