---
id: FR-030
title: "Refuse quantifiers over unbounded domains"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-020
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
  - target: ix://agent-ix/quire-analyze/interface_001
    type: implements
---
# FR-030: Refuse quantifiers over unbounded domains

## Description

If the domain of a universal or existential quantifier is not an enum domain, a finite collection or
a bounded integer range, the analyzer shall return `unsupported` for that quantifier. "Bounded" means
expanded or enumerated over a finite bound; the analyzer never approximates an unbounded quantifier.

## Behavior

- Admitted domains (amending [FR-002](./FR-002-smt-lowering.md), which lists quantifiers as
  unsupported, under the same profile `quire.smtlib2/v2`):
  - an enum domain: every declared member of the enum, in declaration order. Enum sorts are those of
    [FR-020](./FR-020-datatype-lowering.md);
  - a finite collection: a collection literal whose elements are literals fixed at translation time
    (the catalog constructors `quire.op.collection.set`, `quire.op.collection.ordered_set`,
    `quire.op.collection.sequence` and `quire.op.collection.bag`), with element sorts limited to
    Boolean, enum and bounded integer. It has no SMT collection sort and is consumed only as a
    quantifier domain. Its members are the elements in the order and with the duplicate rule of
    [FR-031](./FR-031-enum-and-collection-quantifier-lowering.md). A collection that is a reference to
    a parameter, has a non-literal element, or has any other element sort is not a finite collection;
  - a bounded integer range: an inclusive range whose lower and upper bounds are integer literals or
    declared bounds of bounded integer inputs of [FR-010](./FR-010-linear-integer-arithmetic.md). When
    the lower bound exceeds the upper bound the range is empty.
- Closed set of reason classes for `unsupported` on a quantifier: `unbounded-domain` (an integer with
  no finite declared bounds), `non-finite-bound`, `text-domain`, `rational-domain`,
  `unfixed-collection`, `unencodable-element` (a collection element type with no exact encoding under
  the selected profile, for example a nested collection) and `expansion-ceiling` (an enum or
  collection expansion beyond the budget of [NFR-030](../nonfunctional/NFR-030-quantifier-expansion-bound.md)).
- `unsupported` names the quantifier's source identity and its reason class.
- A refused quantifier never yields a query, a `satisfied` result or a `refuted` result, and is never
  replaced by a sampled, truncated or relaxed domain.
- For every admitted quantifier the query carries one quantifier-map entry holding its source
  identity, domain class (`enum`, `collection` or `range`), encoding (`expanded` or `bounded`) and
  instance count. The map is the observable of admission; entries appear in canonical quantifier
  order, independent of host maps and paths.
- A universal quantifier over an empty domain is true and an existential quantifier over an empty
  domain is false.
- Nested admitted quantifiers are admitted; one refused quantifier at any depth refuses the whole
  request.
- The collection domain is the only admitted use of a collection. The collection refusals of
  [FR-020](./FR-020-datatype-lowering.md) and [FR-022](./FR-022-datatype-package-translation.md)
  (including FR-022-AC-3) hold unchanged for every other use of a collection term.
- Admission is decided from the analysis model alone, before any solver is invoked.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-030-AC-1 | For a universal and an existential quantifier over each of an enum domain, a finite collection and a bounded integer range, the query's quantifier map holds one entry with the expected source identity, domain class and encoding, and no `unsupported` status is returned. | Test (TC-030) |
| FR-030-AC-2 | Each of an unbounded integer, a text domain, a rational domain, an unfixed-size collection, a collection with an unencodable element type and a range with a non-finite bound returns `unsupported` with its matching reason class and the quantifier's source identity, and no solver is invoked. | Test (TC-030) |
| FR-030-AC-3 | An unbounded quantifier nested inside admitted quantifiers refuses the whole request; no partial query is emitted. | Test (TC-030) |
| FR-030-AC-4 | A universal quantifier over an empty domain evaluates to true and an existential quantifier over an empty domain to false in the lowered query, in replay and in the controlled-solver result. | Test (TC-030) |
| FR-030-AC-5 | An enum or collection quantifier whose expansion exceeds the budget returns `unsupported` with reason `expansion-ceiling`. | Test (TC-030) |

## Dependencies

[FR-002](./FR-002-smt-lowering.md), [FR-010](./FR-010-linear-integer-arithmetic.md) and
[FR-020](./FR-020-datatype-lowering.md) (enum sorts).
