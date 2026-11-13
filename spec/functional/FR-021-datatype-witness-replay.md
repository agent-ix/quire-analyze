---
id: FR-021
title: "Replay enum, record and option witness models"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-004
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-020
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface_001
    type: implements
---
# FR-021: Replay enum, record and option witness models

## Description

The analyzer shall parse enum, record and option values from a Z3 or cvc5 model into typed values,
check them against the declared datatypes, and replay every asserted expression against them before
a model or counterexample is published as verified. This extends the verified-model rule of
[FR-004](./FR-004-conclusions.md) from Boolean constants to datatype values. The solver stays an
untrusted external engine.

## Behavior

- A model value is parsed by a bounded parser of the SMT-LIB value syntax solvers print for
  datatypes: a nullary constructor, optionally qualified (`C` or `(as C S)`), a constructor
  application (`(C v1 ... vn)`) and a nested application. Constructor and sort symbols are the encoded
  symbols of [FR-020](./FR-020-datatype-lowering.md) taken from the query's symbol map, never guessed
  from a source name. The option constructors `none` and `some` are those of the option datatype of
  the symbol's payload sort. The two solvers' spellings of one value parse to the same typed value.
- A typed value is one of: Boolean; enum (enumeration, variant); record (record, every field in
  declared field order, each a typed value); option (payload type, absent or present with a typed value).
- A model value is valid only when every constructor, arity, field count and nested value matches the
  declared datatype of the symbol it is bound to, for the encoded symbol map of the query. A value
  that names an unknown constructor or a constructor of a different datatype, has the wrong arity or
  field, is of the wrong sort, is solver-abstract (for example an uninterpreted element, or a constructor not in the symbol map), or is
  malformed makes the model invalid.
- A verified model defines every declared query symbol exactly once, defines no unknown symbol, and
  makes every asserted replay expression true under an independent evaluator of the same expression
  semantics as [FR-020](./FR-020-datatype-lowering.md): enum and record equality, field access,
  option presence and option access, and the Boolean structure of FR-004.
- Evaluation is lazy in the guarded operand of every operator that can guard an `Unwrap` upstream: the
  consequent of an implication (a false antecedent makes it true), the right operand of a
  short-circuit `and` (a false left operand makes it false), and the right operand of a short-circuit
  `or` (a true left operand makes it true), without evaluating the guarded operand. An `Unwrap` that is
  evaluated on an absent option is a replay failure and the model is invalid. The evaluator never reads
  the solver's value of the `some` selector on `none`, and never substitutes a default.
- A datatype failure is one of the eight reasons of [FR-011](./FR-011-integer-witness-replay.md):
  `value_malformed` for text that is not a value, names a constructor no declared datatype has (an
  abstract element included), or nests a constructor application beyond its declared shape (a
  depth-9 text for a depth-8 declared type, for example); `sort_mismatch` for a constructor of
  another datatype, a nullary constructor where a Boolean or integer is expected, a wrong arity or
  a wrong qualifying or declared sort (a sort that is not the symbol of the query's symbol map, a
  parametric spelling such as `(Option S)` included); `value_out_of_domain` for an integer part
  outside its declared range or an integer beyond 128 bits; and `assertion_false` for a failed
  assertion, including an `Unwrap` evaluated on an absent option.
- An invalid or incomplete model keeps the logical response and marks the requested counterexample
  evidence incomplete; it never fabricates a source value, never repairs a mismatched value, and
  never makes the result conclusive on the strength of a model that failed replay. The exact adapter
  outcome stays available.
- Published values preserve the complete origin identity and explicit binding group of every symbol
  from the query map, and show enum values by variant name, record values by field name, and options
  as `absent` or `present` with the payload.
- Model parsing is bounded by the parsed-model ceiling of [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md)
  and parsing follows the declared shape, whose nesting is bounded by NFR-020.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-021-AC-1 | Enum, record and option values in a model, in the spelling of each solver, parse into the same typed values, and every assertion replays to true. | Test |
| FR-021-AC-2 | A model value that does not match its declared datatype (unknown or foreign constructor, wrong arity, wrong sort, abstract element, malformed text) fails replay and the counterexample evidence is incomplete, and the result is not marked verified. | Test |
| FR-021-AC-3 | Replay tests cover each datatype kind: enum, record, option present, option absent, and a record that nests an option and an enum. | Test |
| FR-021-AC-4 | A model that satisfies the solver but violates one asserted expression under the independent evaluator is rejected as incomplete. | Test |
| FR-021-AC-5 | A model that omits a declared datatype symbol, or defines an unknown symbol or a symbol twice, is not verified. | Test |
| FR-021-AC-7 | An implication, short-circuit `and` or short-circuit `or` whose guard excludes the unwrapped operand replays without evaluating its `Unwrap`, and an `Unwrap` evaluated on an absent option fails replay and the model is invalid. | Test |
| FR-021-AC-6 | A model value nested beyond its declared shape (which lowering bounds at the NFR-020 depth) is `value_malformed` without unbounded recursion, and a model larger than the model ceiling is the adapter `ModelLimit`; neither is ever conclusive. | Test |

## Dependencies

FR-004 and FR-020, and the integer witness replay that lands first for integer-typed record fields.
