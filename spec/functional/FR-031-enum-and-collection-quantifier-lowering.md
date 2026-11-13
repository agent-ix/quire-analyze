---
id: FR-031
title: "Lower quantifiers over enum domains and finite collections by expansion"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-020
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-031: Lower quantifiers over enum domains and finite collections by expansion

## Description

The analyzer shall lower a universal or existential quantifier over an enum domain or a finite
collection by expansion into one instance of the body per domain element, joined by conjunction for a
universal and by disjunction for an existential quantifier.

## Behavior

- Encoding choice: these forms use expansion and never emit a solver quantifier. Decidability
  argument: the expanded query is quantifier-free, so it is decidable exactly when the logic of the
  body alone is decidable. The bodies admitted here lie in linear integer arithmetic
  ([FR-010](./FR-010-linear-integer-arithmetic.md)), datatypes ([FR-020](./FR-020-datatype-lowering.md))
  and Boolean structure, and non-linear integer bodies are `unsupported` before expansion, so the
  expanded query stays in a decidable quantifier-free fragment and expansion adds no quantifier
  instantiation.
- Each instance substitutes the element for the bound variable. The substitution is capture-avoiding:
  an element never binds a free variable of the body, and an inner quantifier that reuses the bound
  variable's name shadows it.
- The analysis model exposes a closed `DomainQuantifier` with `identity: String`, `kind:
  QuantifierKind`, `local: SymbolName`, `local_type: ValueType`, `domain: QuantifierDomain` and
  `body: AnalysisFormula`. `QuantifierDomain` is `Enum(SymbolName)`, `Collection { kind:
  CollectionKind, elements: Vec<DomainElement> }` or `Unbounded(UnboundedDomain)`.
  `DomainElement` is `Boolean(bool)`, `Enum { enumeration: SymbolName, variant: SymbolName }` or
  `Integer(i128)`. `CollectionKind` is the closed set `Set`, `OrderedSet`, `Sequence` and `Bag`.
  `UnboundedDomain` is the closed set `Integer`, `Text`, `Rational`, `UnfixedCollection` and
  `UnencodableElement`. The `AnalysisFormula` tree admits `Domain` nodes alongside terms, Boolean
  operators and `Range` quantifiers. `StatementInput.with_formula` rejects an empty source
  identity, an undeclared or mismatched enum, and a collection member that does not fit its local
  type; [FR-030-AC-6](./FR-030-bounded-quantifier-domains.md) defines the permitted empty collection
  element sorts.
- Instances appear in domain order. An enum domain uses declaration order. For a collection, a
  `quire.op.collection.set` and a `quire.op.collection.bag` are enumerated in ascending canonical
  element order (false before true, enum declaration order, ascending integer), and a
  `quire.op.collection.ordered_set` and a `quire.op.collection.sequence` in written order. In a
  `set` and an `ordered_set` a repeated literal collapses to its first occurrence; in a `bag` and a
  `sequence` every occurrence is an instance. The instance count is the number of elements after this
  rule, and an empty collection is an empty domain.
- Assertion names, symbol names and metadata are stable and independent of host maps and paths.
- Each quantifier-map entry retains the source identity and domain data. Range data is a lower and
  upper bound pair; enum and collection data is a typed list of elements in instance order. This
  map describes lowering and does not claim to identify which expanded instance failed. Replay
  reports the first failing instance, its element and source identity under
  [FR-033](./FR-033-quantified-assertion-replay.md).
- Expansion consumes the instance budget defined in
  [NFR-030](../nonfunctional/NFR-030-quantifier-expansion-bound.md); this requirement states no
  separate ceiling.
- The lowered query declares the logic the expanded body requires and never a quantified logic.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-031-AC-1 | For a universal and an existential fixture over an enum domain, the lowered query equals its golden query, and the status derived by the enumeration oracle of [FR-033](./FR-033-quantified-assertion-replay.md) equals the fixture's recorded expected status. Where Z3 and cvc5 are installed, both also answer the matching `sat`/`unsat`. | Test (TC-030; real solvers where installed) |
| FR-031-AC-2 | The same holds for a universal and an existential fixture over a finite collection literal; a `set` with a repeated literal yields the same query as the `set` without the repeat, a `sequence` with a repeated literal yields one instance per occurrence, and each of `set`, `ordered_set`, `sequence` and `bag` enumerates in the order stated above. | Test (TC-030; real solvers where installed) |
| FR-031-AC-3 | The query for a quantified fixture contains no `forall` or `exists` token and one body instance per domain element, in domain order. | Test (TC-030) |
| FR-031-AC-4 | Lowering the same quantified package twice, in separate processes, yields byte-identical SMT-LIB2 output. | Test (TC-030) |
| FR-031-AC-5 | The quantifier-map entry for an enum or collection quantifier records its source identity and typed elements in instance order. It does not identify a failed instance; replay does that under FR-033. | Test (TC-030) |
| FR-031-AC-6 | A body that mentions a free variable whose name equals the bound variable of a nested inner quantifier lowers so that the free variable is not captured; the golden query shows distinct symbols. | Test (TC-030) |

## Dependencies

[FR-030](./FR-030-bounded-quantifier-domains.md), [FR-002](./FR-002-smt-lowering.md) and
[FR-020](./FR-020-datatype-lowering.md).
