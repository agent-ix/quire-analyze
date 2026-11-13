---
id: FR-020
title: "Lower enums, records and options to SMT datatypes"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
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
  name. An `EnumLiteral` whose enumeration or variant, a `RecordLiteral` or `FieldAccess` whose record
  or field, or a value declaration whose type name does not resolve in the environment is refused with
  a stable `InvalidBinding` or `UnsupportedConstruct` diagnostic before any solver runs.
- Only types reachable from a value declaration or expression of a selected statement are declared in
  the query, so an unused environment declaration does not change the query.

### Encoding

- An enum lowers to one datatype with one nullary constructor per declared variant. Two enums with
  the same variant names are distinct sorts.
- A record lowers to one datatype with one constructor and one selector per declared field. A field
  type is Boolean, an enum, a record or an option over a permitted type, and an integer only when the
  selected capability contract lists integers as exact. A field of any other type (text, rational,
  collection) makes the record Unsupported.
- An option over payload type `T` lowers to one datatype with a nullary `none` constructor and a
  `some` constructor with one selector. There is one option datatype per distinct payload sort, and
  two options over the same payload share it.
- Sort, constructor and selector symbols encode the complete semantic identity of the owning type
  and, for a constructor or selector, the variant or field name, injectively and independently of host
  maps and paths. A user-chosen name equal to an SMT-LIB reserved word or to a name used by another
  encoded symbol cannot collide.
- Datatype declarations appear before every use, in dependency order with ties broken by the encoded
  symbol, and variants and fields are emitted in ascending name order. Order of declaration in the
  environment does not change the query bytes.
- The query logic is the one the capability contract declares for the datatypes the query uses
  (`QF_DT` when only datatypes and Boolean structure occur). The logic is never chosen from a solver
  or host.

### Expressions

- `EnumLiteral` lowers to its constructor. Enum equality and inequality (`Compare` with `Equal` or
  `NotEqual`) lower to `=` and `distinct` over the enum sort. Ordering comparison of enums is
  Unsupported.
- Membership of an enum-typed term in a set of literals ("role is one of the permitted roles") lowers
  to the total disjunction of equalities of the term with each literal, in ascending variant order.
  An empty set lowers to `false`. A set that repeats a literal lowers as the set, so repetition does
  not change the query.
- `RecordLiteral` lowers to the constructor applied to the field values in declared field order, and
  `FieldAccess` lowers to the selector. Record equality and inequality lower to `=` and `distinct`.
- `OptionNone` lowers to the `none` constructor of the payload sort, `OptionSome` to `some`, and
  `IsPresent` to the `some` tester. Option equality and inequality lower to `=` and `distinct`.
- `Unwrap` lowers to the `some` selector and is defined only where its presence proof holds. The
  upstream check (`check_expression`) admits `Unwrap` only under a presence proof (an `IsPresent`
  fact or an `OptionSome` operand). A statement that reaches lowering with an `Unwrap` that carries no
  presence proof is refused as `UnsupportedConstruct` naming the unwrap, and is never given a value
  for the absent case. The selector's value on `none` is unconstrained and unobservable under the
  proof.
- A `Compare` whose operands are not both Boolean, both of the same enum, both of the same record, or
  both of the same option sort is refused. A mixed-sort comparison is never coerced.

### Capability contract

- `CAPABILITY_CONTRACT` shall list `enum_literal`, `enum_equality`, `enum_membership`, `record`,
  `record_equality`, `field_access`, `option`, `option_equality`, `option_presence` and `option_access`
  as exact constructs, and shall remove `enum`, `record`, `option`, `field_access` and `option_access`
  from its unsupported list. `collection`, `collection_access` and `text` remain unsupported.
- The exhaustive `ExpressionKind` match keeps an upstream variant addition a compile-time review
  point. Every construct not listed exact returns `unsupported` with no approximate query.

### Refusals

- A record or option that nests a collection or text at any depth is Unsupported, naming the
  declaration and the nested field path, and the whole request returns `unsupported`. It is never
  weakened by dropping the field or by abstracting it to an uninterpreted sort.
- A record type that contains itself, directly or through options or other records, is Unsupported,
  because the bounded encoding is non-recursive.
- A datatype bound of NFR-010 that is exceeded is refused as a limit breach and never produces a
  conclusive result.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-020-AC-1 | A satisfiable and an unsatisfiable enum-membership fixture ("role is one of the permitted roles") each lower to a query that both Z3 and cvc5 decide correctly (`sat`/`unsat`). | Test (TC-020; real solvers where installed) |
| FR-020-AC-2 | Every enum variant, record field and type name in a query traces to a declaration in the environment, and an unresolved enum, variant, record, field or type name is refused before solver execution. | Test (TC-020) |
| FR-020-AC-3 | A record-field constraint fixture and an option-access fixture (`IsPresent` guard then `Unwrap`) each lower and are decided correctly by both solvers. | Test (TC-020; real solvers where installed) |
| FR-020-AC-4 | An `Unwrap` with no presence proof is refused as `UnsupportedConstruct`, and an `Unwrap` under `IsPresent` is lowered to the selector. | Test (TC-020) |
| FR-020-AC-5 | A record or option nesting a collection or text, a self-containing record, an ordering comparison of enums, and a mixed-sort comparison each return `unsupported` before solver execution, with no query. | Test (TC-020) |
| FR-020-AC-6 | `CAPABILITY_CONTRACT` lists the constructs above as exact and `collection`, `collection_access` and `text` as unsupported, and no construct appears in both lists. | Test (TC-020) |
| FR-020-AC-7 | Permuting environment type order, variant order, field order and statement order yields byte-identical queries and maps, and two enums with identical variant names produce distinct sorts. | Test (TC-020) |
| FR-020-AC-8 | Enum membership over a set with a repeated literal yields the same query as the set without the repeat, and membership over an empty set yields `false`. | Test (TC-020) |

## Dependencies

FR-002 and the integer capability contract change that lands first. Encoded integer fields are
exact only where that contract lists integers as exact.
