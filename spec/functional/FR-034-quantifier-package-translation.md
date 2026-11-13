---
id: FR-034
title: "Translate bounded quantifiers from QSL contract packages"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-009
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-012
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-022
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-034: Translate bounded quantifiers from QSL contract packages

## Description

The `run-package` translation of [FR-009](./FR-009-package-runner.md) shall translate quantifier
expressions of an admitted `quire.contract-package/v1` document into the analysis model's bounded
quantifiers, so that `run-package` analyses quantified QSL clauses without a separate quantifier
request. This amends FR-009, which lists quantifiers among the terms that exit `2`, for the domains
that [FR-030](./FR-030-bounded-quantifier-domains.md) admits.

## Behavior

- **Terms added.** A quantifier term of an `analysis_claim` body translates to a universal or
  existential quantifier with its domain classified by FR-030, its bound variable, its Boolean body
  and its source span. The quantifier is a Boolean term that Boolean operations may combine. Its
  operation row extends the Terms table of FR-009 and uses the same operation-identity rule.
- **Types.** An enum domain uses the enum type mapping of [FR-022](./FR-022-datatype-package-translation.md);
  a range domain uses the `integer_range` mapping of
  [FR-012](./FR-012-integer-claim-translation.md), whose bounds and overflow rule it inherits: a range
  bound outside the signed 64-bit range is refused by the IR, and the saturating overflow policy
  stays unsupported. A bound variable of an integer range has the integer type whose minimum and
  maximum are the range bounds.
- **Quantifier identities.** The checked operation catalog (`quire-verification-contracts`) defines
  `quire.op.collection.forall` and `quire.op.collection.exists`, each with a collection-kind first
  operand and a Boolean-bodied binder as second operand. They are the identities of the universal
  and existential quantifier. The catalog names no separate enum-domain or range-domain quantifier
  identity: an enum or `integer_range` domain is the first operand of the same two identities when the
  checked reading of the package admits it as a domain operand, and a package whose first operand is
  not admitted never reaches the translation.
- **Collection domains.** A first operand that is an `Application` of `quire.op.collection.set`,
  `quire.op.collection.ordered_set`, `quire.op.collection.sequence` or `quire.op.collection.bag`
  whose arguments are all `Literal` terms of Boolean, enum or integer-range type translates to a
  finite-collection domain with those elements and no collection value or sort; the collection exists
  only as the quantifier's domain. A first operand that is a `Reference` to a parameter, a
  collection with a non-literal element or an element of another type (text, rational, record,
  option, collection), or any other collection operation, exits `2` naming the node. A collection
  term in any other position exits `2` as FR-009 and
  [FR-022](./FR-022-datatype-package-translation.md) state; the collection refusals of FR-020 and
  FR-022 are unchanged except for this domain position.
- The domain is taken from the package's declared types and bounds only; the translation does not
  infer a bound from the body and does not widen a missing bound.
- A quantifier whose domain is not admitted makes the translation stop at its node and the runner
  exit `2` naming `semantic_graph.nodes[N]`, with the reason class and span of FR-030 on stderr and
  no report, which is the FR-009 behavior for a term the translation cannot lower. The runner never
  reports a conclusive result for it.
- Bound variables are scoped to their quantifier; two quantifiers that reuse a variable name do not
  share a symbol.
- Node bounds: the depth, node-count and shared-subterm bounds of FR-009 apply to the translated
  `Expression` tree, in which a quantifier body appears once. Expansion happens in lowering, after
  translation, and is bounded by [NFR-030](../nonfunctional/NFR-030-quantifier-expansion-bound.md) and by
  the `MAX_EXPRESSION_NODES` and query byte limits of the lowering, so a quantifier cannot bypass the
  FR-009 bounds by expansion.
- The translation of a package that holds no quantifier node shall be unchanged by this requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-034-AC-1 | A package holding an enum-domain, a collection-domain (one fixture per constructor `set`, `ordered_set`, `sequence` and `bag`, with Boolean, enum and integer elements) and a range-domain quantifier translates to the matching bounded quantifiers with source spans preserved, and the quantifier map of its lowered query holds one entry for each. | Test (TC-030) |
| FR-034-AC-2 | A package holding a quantifier over an unbounded or text domain exits `2` naming `semantic_graph.nodes[N]` of the quantifier node, with the reason class and span on stderr and no report. | Test (TC-030) |
| FR-034-AC-3 | Two quantifiers reusing one variable name in one claim produce distinct, non-colliding symbols. | Test (TC-030) |
| FR-034-AC-4 | A quantified package run through `run-package` yields the same report bytes, except `elapsedMs`, on repeated runs. | Test (TC-030) |
| FR-034-AC-5 | For the fixtures of FR-009-AC-21 and FR-012-AC-4, which hold no quantifier node, the lowered queries equal their golden queries. | Test (TC-030) |
| FR-034-AC-6 | A quantifier whose body is a 10,000-node tree is not refused for size and one whose body is 10,001 nodes exits `2`, as FR-009-AC-23 states for other trees; a literal range of width 1,024 translates, and its 1,024 expanded instances do not count toward that node bound. | Test (TC-030) |
| FR-034-AC-7 | A collection with a non-literal element, a collection of text, rational, record or nested-collection elements, a `Reference` to a collection parameter, `quire.op.collection.contains`, `size` or `index` applied to a literal collection, and a collection literal outside a quantifier domain each exit `2` naming `semantic_graph.nodes[N]`, with no report. | Test (TC-030) |

## Dependencies

[FR-030](./FR-030-bounded-quantifier-domains.md), [FR-009](./FR-009-package-runner.md),
[FR-012](./FR-012-integer-claim-translation.md) and
[FR-022](./FR-022-datatype-package-translation.md).
