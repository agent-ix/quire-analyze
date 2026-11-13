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
| A value declaration or binding whose type name does not resolve in the environment | `InvalidBinding`, path `binding.type` | binding validation |
| A construct outside the exact list (collection, text, rational, enum ordering) | `UnsupportedConstruct` | expression lowering |
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
- Datatype declarations appear before every use, in dependency order with ties broken by the encoded
  symbol, and variants and fields are emitted in ascending name order. Order of declaration in the
  environment does not change the query bytes.

### Logic

[FR-010](./FR-010-lia-lowering.md) fixes one logic per capability profile. This requirement adds
profiles on the same rule: a query carries exactly one profile, and `CapabilityContract.logic` of that
profile is the query logic. The logic is never chosen from a solver or host.

| Query uses | Logic |
|---|---|
| Boolean structure only (`quire.smtlib2/v1`, unchanged) | `QF_UF` |
| Datatypes (enum, record, option) and Boolean structure, no integer | `QF_DT` |
| Datatypes and integer record fields or integer comparisons | `QF_UFDTLIA` |

The profile is selected from the constructs the selected statements use, deterministically: any
datatype sort or datatype expression selects at least the datatype profile, and any integer construct
selects the integer-with-datatype profile. The integer-only logic remains the one FR-010 declares.

### Expressions

- `EnumLiteral` lowers to its constructor. Enum equality and inequality (`Compare` with `Equal` or
  `NotEqual`) lower to `=` and `distinct` over the enum sort. Ordering comparison of enums is
  Unsupported.
- There is no membership construct in `ExpressionKind`. "Role is one of the permitted roles" is
  authored, and lowered, as the total disjunction of enum equalities of the term with each permitted
  literal. The disjunction is built from the exact Boolean `or` and enum equality constructs, and no
  separate membership encoding exists. Membership over a collection or set value
  (`quire.op.collection.contains` over `quire.op.collection.set`) and membership through a quantifier
  stay Unsupported here, and the quantifier forms are owned by the quantification requirements.
- `RecordLiteral` lowers to the constructor applied to the field values in declared field order, and
  `FieldAccess` lowers to the selector. Record equality and inequality lower to `=` and `distinct`.
- `OptionNone` lowers to the `none` constructor of the payload sort, `OptionSome` to `some`, and
  `IsPresent` to the `some` tester. Option equality and inequality lower to `=` and `distinct`.
- `Unwrap` lowers to the `some` selector and is defined only where its presence proof holds. The
  upstream check (`check_expression`) admits `Unwrap` only under a presence proof (an `IsPresent`
  fact established by the antecedent of an implication, or an `OptionSome` operand). An `Unwrap`
  without that proof is refused by `check_expression` at statement build as `InvalidStatement`
  (path `statement.expression`), and the lowering adds no separate check. The selector's value on
  `none` is unconstrained and unobservable under the proof, and is never given meaning.
- A `Compare` whose operands are not both Boolean, both of the same enum, both of the same record, or
  both of the same option sort is refused as `UnsupportedConstruct`. A mixed-sort comparison is never
  coerced.

### Capability contract

- `CAPABILITY_CONTRACT` shall list `enum_literal`, `enum_equality`, `record`, `record_equality`,
  `field_access`, `option`, `option_equality`, `option_presence` and `option_access` as exact
  constructs, and shall remove `enum`, `record`, `option`, `field_access` and `option_access` from its
  unsupported list. `collection`, `collection_access` and `text` remain unsupported.
- The exhaustive `ExpressionKind` match keeps an upstream variant addition a compile-time review
  point. Every construct not listed exact returns `unsupported` with no approximate query.

### Refusals

- A record or option that nests a collection or text at any depth is `UnsupportedType`, naming the
  declaration and the nested field path, and the whole request returns `unsupported`. It is never
  weakened by dropping the field or by abstracting it to an uninterpreted sort.
- A datatype bound of [NFR-020](../nonfunctional/NFR-020-datatype-bounds.md) that is exceeded is a
  `ResourceLimit` and never produces a conclusive result.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-020-AC-1 | A satisfiable and an unsatisfiable role-membership fixture, each written as a disjunction of enum equalities (`or` of `role = Admin` and `role = Auditor`, with and without a further `role = Guest`), lower to queries that both Z3 and cvc5 decide `sat` and `unsat` respectively. | Test (real solvers where installed) |
| FR-020-AC-2 | Every enum variant, record field and type name in a query traces to a declaration in the environment, and each unresolved name is refused with the code of the case table before solver execution. | Test |
| FR-020-AC-3 | A record-field constraint fixture and an option-access fixture (`IsPresent` in the antecedent of an implication, `Unwrap` in the consequent) each lower and are decided correctly by both solvers. | Test (real solvers where installed) |
| FR-020-AC-4 | An `Unwrap` with no presence proof is refused by `check_expression` as `InvalidStatement` at statement build and nothing is lowered; an `Unwrap` under the proof lowers to the selector. | Test |
| FR-020-AC-5 | A record or option nesting a collection or text returns `UnsupportedType`, an enum ordering comparison and a mixed-sort comparison return `UnsupportedConstruct`, and a self-containing record fails at `DeclarationEnvironment::new` with `RecursiveType`. None yields a query. | Test |
| FR-020-AC-6 | `CAPABILITY_CONTRACT` lists the constructs above as exact and `collection`, `collection_access` and `text` as unsupported, and no construct appears in both lists. | Test |
| FR-020-AC-7 | Permuting environment type order, variant order, field order and statement order yields byte-identical queries and maps, and two enums with identical variant names produce distinct sorts. | Test |
| FR-020-AC-8 | A membership disjunction with a repeated literal is lowered as written, and `quire.op.collection.contains` over `quire.op.collection.set` returns `UnsupportedConstruct`. | Test |
| FR-020-AC-9 | The query logic is `QF_UF` for a Boolean-only query, `QF_DT` for a datatype query without integers, and `QF_UFDTLIA` for a datatype query with an integer field, each emitted in `(set-logic ...)` and equal to the profile's `CapabilityContract.logic`. | Test |

## Dependencies

FR-002 and FR-010 (the single-logic-per-profile rule and the integer capability contract). Integer
record fields are exact only where the selected profile lists integers as exact. Quantification and
collection membership are owned by other requirements and stay Unsupported here.
