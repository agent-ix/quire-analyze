---
id: FR-020
title: "Lower enums, records and options to SMT datatypes"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-010
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface_001
    type: implements
---
# FR-020: Lower enums, records and options to SMT datatypes

## Description

The analyzer shall lower enum, record and option values, and the expressions over them, to
SMT-LIB 2.6 algebraic datatypes, so Z3 and cvc5 decide equality, membership, field access and option
access over data-classification and role rules. `quire-analyze` is a QSL-fed runner: Z3 and cvc5 stay
external solvers, and this requirement adds an encoding, not a prover. Collections and text stay
unsupported.

## Behavior

### Declarations come from the environment

- Every enum, record and option sort in a query is derived from the `DeclarationEnvironment` of the
  statements being lowered. The lowering shall not invent an enum variant, a record field or a type
  name.
- Only types reachable from a value declaration or expression of a selected statement are declared in
  the query, so an unused environment declaration does not change the query.
- Refusal codes (`LoweringErrorCode` of `src/smt.rs`) are fixed by case:

| Case | Code | Raised by |
|---|---|---|
| An `EnumLiteral`, `RecordLiteral`, `FieldAccess` or `Unwrap` whose enumeration, variant, record, field or presence proof does not hold | `InvalidStatement`, path `statement.expression` | `check_expression` at statement build, before lowering |
| An enum ordering comparison, or a comparison whose operands differ in sort | `InvalidStatement`, path `statement.expression` | `check_expression` at statement build, which refuses it as ill-typed, before lowering |
| A value declaration whose type name does not resolve in the environment | refused by `DeclarationEnvironment::new` as `OrphanedTypeReference`, so no environment and no statement exists | environment build |
| A construct outside the exact list (collection, text, rational), and, defensively, an enum ordering or mixed-sort comparison that reaches lowering unchecked | `UnsupportedConstruct` | expression lowering |
| A declared type that nests a collection or text | `UnsupportedType` | type lowering |
| A bound of NFR-020 exceeded | `ResourceLimit` | type lowering |
| A record type that contains itself | refused by `DeclarationEnvironment::new` as `RecursiveType`, so no environment and no statement exists | environment build |

### Encoding

- An enum lowers to one datatype with one nullary constructor per declared variant. Two enums with
  the same variant names are distinct sorts.
- A record lowers to one datatype with one constructor and one selector per declared field. A field
  type is Boolean, an enum, a record or an option over a permitted type, and an integer only when the
  selected profile lists integers as exact. A field of any other type (text, rational, collection)
  makes the record Unsupported.
- An option over payload type `T` lowers to one datatype with a nullary `none` constructor and a
  `some` constructor with one selector. There is one option datatype per distinct payload sort, and
  two options over the same payload share it.
- Sort, constructor and selector symbols encode the complete semantic identity of the owning type
  and, for a constructor or selector, the variant or field name, injectively and independently of host
  maps and paths. The encoded symbols are recorded in the query's symbol map, which
  [FR-021](./FR-021-datatype-witness-replay.md) uses to read models. A user-chosen name equal to an
  SMT-LIB reserved word or to a name used by another encoded symbol cannot collide.
- For every datatype-sorted variable the query asserts the declared range of each integer part
  through one non-recursive range predicate (define-fun) per typed shape, a datatype sort together
  with the integer ranges of its parts (two options over integer payloads of different ranges
  share a sort but not a predicate). A record predicate is the conjunction of its integer field
  ranges and of the predicates of its record and option fields; an option predicate is `(=> ((_ is
  some) x) <payload range or payload predicate>)`. The definitions follow the datatype
  declarations in dependency order, ties broken by the predicate symbol. Each variable gets one
  unnamed assertion of its predicate, outside the assertion map and report assertions[], following
  the FR-010 integer range block, ordered by ascending variable symbol.
- Datatype declarations appear before every use, in dependency order with ties broken by the encoded
  symbol, and variants and fields are emitted in ascending name order. Order of declaration in the
  environment does not change the query bytes.

### Logic

[FR-010](./FR-010-linear-integer-arithmetic.md) replaces the Boolean-only profile with
`quire.smtlib2/v2`, which is the only profile; `quire.smtlib2/v1` and `QF_UF` are no longer emitted or
accepted, there is no fallback, and a query is never emitted under two profiles. The profile's logic
follows the feature set of the query, and this requirement owns the datatype case:

| Query | Logic |
|---|---|
| No datatype declaration (including a Boolean-only query) | `QF_LIA` (FR-010) |
| Declares at least one datatype (enum, record or option), with or without integer terms | `QF_UFDTLIA` |
| Holds a bounded solver quantifier ([FR-032](./FR-032-bounded-integer-range-quantifier-lowering.md)) and declares no datatype | `LIA` |
| Holds a bounded solver quantifier and declares at least one datatype | `UFDTLIA` |

The logic is determined by whether the query declares a datatype and whether it holds a bounded solver quantifier, never by a solver or host. A report
for a datatype query without a bounded solver quantifier names `quire.smtlib2/v2` and `QF_UFDTLIA`, and report validation
([FR-005](./FR-005-evidence-cli.md)) accepts exactly the logic the query's feature set requires and
refuses `quire.smtlib2/v1` and `QF_UF`.

### Expressions

- `EnumLiteral` lowers to its constructor. Enum equality and inequality (`Compare` with `Equal` or
  `NotEqual`) lower to `=` and `distinct` over the enum sort. Ordering comparison of enums is
  Unsupported.
- There is no membership construct in `ExpressionKind`. "Role is one of the permitted roles" is
  authored, and lowered, as the total disjunction of enum equalities of the term with each permitted
  literal. The disjunction is built from the exact Boolean `or` and enum equality constructs, and no
  separate membership encoding exists. Membership over a collection value and membership through a quantifier
  stay Unsupported here, and the quantifier forms are owned by the quantification requirements.
- `RecordLiteral` lowers to the constructor applied to the field values in ascending field-name order (the order of the datatype's constructor), and
  `FieldAccess` lowers to the selector. Record equality and inequality lower to `=` and `distinct`.
- `OptionNone` lowers to the `none` constructor of the payload sort, `OptionSome` to `some`, and
  `IsPresent` to the `some` tester. Option equality and inequality lower to `=` and `distinct`.
- `Unwrap` lowers to the `some` selector and is defined only where its presence proof holds. The
  upstream check (`check_expression`) admits `Unwrap` only under a presence proof: an `IsPresent` fact
  of the same option term established by the antecedent of an implication, by the left operand of a
  short-circuit `and`, or by the left operand of a short-circuit `or` (as the negated fact), or an
  `OptionSome` operand. Total `and` and `or` establish no presence fact. An `Unwrap` without that proof
  is refused by `check_expression` at statement build as `InvalidStatement` (path
  `statement.expression`), and the lowering adds no separate check. The selector's value on `none` is
  unconstrained and unobservable under the proof, and is never given meaning.
- A `Compare` whose operands are not both Boolean, both of the same enum, both of the same record, or
  both of the same option sort is refused at statement build by `check_expression` as `InvalidStatement`
  (path `statement.expression`). The lowering also refuses such a `Compare` as `UnsupportedConstruct`, as a
  defensive check over an unchecked expression. A mixed-sort comparison is never coerced.

### Capability contract

- `CAPABILITY_CONTRACT` shall list `enum_literal`, `enum_equality`, `enum_inequality`, `record`, `record_equality`,
  `field_access`, `option`, `option_equality`, `option_presence` and `option_access` as exact
  constructs, and shall remove `enum`, `record`, `option`, `field_access` and `option_access` from its
  unsupported list. `collection`, `collection_access` and `text` remain unsupported.
- The exhaustive `ExpressionKind` match keeps an upstream variant addition a compile-time review
  point. Every construct not listed exact returns `unsupported` with no approximate query.

### Replay of datatype models

A model for a query that declares datatype-sorted symbols is replayed by
[FR-021](./FR-021-datatype-witness-replay.md) for every datatype kind (enum, record and option). No
datatype kind is refused as unreplayable, and a datatype symbol that fails replay fails with one of
the eight [FR-011](./FR-011-integer-witness-replay.md) reasons.

### Refusals

- A record or option that nests a collection or text at any depth is `UnsupportedType`, naming the
  declaration and the nested field path, and the whole request returns `unsupported`. It is never
  weakened by dropping the field or by abstracting it to an uninterpreted sort.
- A datatype bound of [NFR-020](../nonfunctional/NFR-020-datatype-bounds.md) that is exceeded is a
  `ResourceLimit` and never produces a conclusive result.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-020-AC-1 | Two role-membership fixtures, each a single assertion `role = Admin` or `role = Auditor` (a disjunction of enum equalities) plus one further assertion, are decided by both Z3 and cvc5: with the further assertion `role != Admin` the query is `sat`, and with the further assertion `role = Guest` the query is `unsat`. | Test (real solvers where installed) |
| FR-020-AC-2 | Every enum variant, record field and type name in a query traces to a declaration in the environment, and each unresolved name is refused with the code of the case table before solver execution. | Test |
| FR-020-AC-3 | A record-field constraint fixture and an option-access fixture (`IsPresent` in the antecedent of an implication, `Unwrap` in the consequent) each lower and are decided correctly by both solvers. | Test (real solvers where installed) |
| FR-020-AC-4 | An `Unwrap` with no presence proof is refused by `check_expression` as `InvalidStatement` at statement build and nothing is lowered; an `Unwrap` under the proof lowers to the selector. | Test |
| FR-020-AC-5 | A record or option nesting a collection or text returns `UnsupportedType`, an enum ordering comparison and a mixed-sort comparison are refused at statement build as `InvalidStatement` (path `statement.expression`) and, over an unchecked expression, by the lowering as `UnsupportedConstruct`, and a self-containing record fails at `DeclarationEnvironment::new` with `RecursiveType`. None yields a query. | Test |
| FR-020-AC-6 | The capability contract of `quire.smtlib2/v2` lists the constructs above as exact and `collection`, `collection_access` and `text` as unsupported, and no construct appears in both lists. | Test |
| FR-020-AC-7 | Permuting environment type order, variant order, field order and statement order yields byte-identical queries and maps, and two enums with identical variant names produce distinct sorts. | Test |
| FR-020-AC-8 | A membership disjunction with a repeated literal is lowered as written, and a `CollectionLiteral` or any other collection expression returns `UnsupportedConstruct`, except a collection literal consumed as a quantifier domain per [FR-030](./FR-030-bounded-quantifier-domains.md) and [FR-034](./FR-034-quantifier-package-translation.md). | Test |
| FR-020-AC-9 | The `(set-logic ...)` of a query declaring no datatype is `QF_LIA` and of a query declaring at least one datatype is `QF_UFDTLIA`, in both cases under `quire.smtlib2/v2`, that of a query holding a bounded solver quantifier is `LIA` without and `UFDTLIA` with a datatype, and a report naming `quire.smtlib2/v1`, `QF_UF`, or a logic other than the one the feature set requires is refused by validation. | Test |
| FR-020-AC-10 | A satisfiable model for a query declaring any datatype (enum, record or option), including one whose terms are only datatype literals, is replayed by FR-021: it is verified when every assertion holds under the typed evaluation and is incomplete, with one of the FR-011 reasons, when it does not; no datatype kind is refused as unreplayable. | Test |

## Dependencies

FR-002 and FR-010 (the `quire.smtlib2/v2` profile and the integer capability contract). Integer
record fields are exact only where the selected profile lists integers as exact. Quantification and
collection membership are owned by other requirements and stay Unsupported here.
