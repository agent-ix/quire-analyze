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
  datatypes: a nullary constructor (`Red` or `(as Red Role)`), a constructor application
  (`(mk-User Admin true)`), a nested application (`(some (mk-User Admin true))`) and the qualified
  `none` (`(as none (Option Role))`). The two solvers' spellings parse to the same typed value.
- A typed value is one of: Boolean; enum (enumeration, variant); record (record, every field in
  declared order, each a typed value); option (payload type, absent or present with a typed value).
- A model value is valid only when every constructor, arity, field count and nested value matches the
  declared datatype of the symbol it is bound to, for the encoded symbol map of the query. A value
  that names an unknown constructor or a constructor of a different datatype, has the wrong arity or
  field, is of the wrong sort, is solver-abstract (for example an uninterpreted element), or is
  malformed makes the model invalid.
- A verified model defines every declared query symbol exactly once, defines no unknown symbol, and
  makes every asserted replay expression true under an independent evaluator of the same expression
  semantics as [FR-020](./FR-020-datatype-lowering.md): enum and record equality, membership, field
  access, option presence and option access, and the Boolean structure of FR-004.
- An invalid or incomplete model keeps the logical response and marks the requested counterexample
  evidence incomplete; it never fabricates a source value, never repairs a mismatched value, and
  never makes the result conclusive on the strength of a model that failed replay. The exact adapter
  outcome stays available.
- Published values preserve the complete origin identity and explicit binding group of every symbol
  from the query map, and show enum values by variant name, record values by field name, and options
  as `absent` or `present` with the payload.
- Model parsing is bounded by the parsed-model ceiling of [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md)
  and by the datatype nesting bound of NFR-010.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-021-AC-1 | Enum, record and option values in a model, in the spelling of each solver, parse into the same typed values, and every assertion replays to true. | Test (TC-020; real solvers where installed) |
| FR-021-AC-2 | A model value that does not match its declared datatype (unknown or foreign constructor, wrong arity, wrong sort, abstract element, malformed text) fails replay and the counterexample evidence is incomplete, and the result is not marked verified. | Test (TC-020) |
| FR-021-AC-3 | Replay tests cover each datatype kind: enum, record, option present, option absent, and a record that nests an option and an enum. | Test (TC-020) |
| FR-021-AC-4 | A model that satisfies the solver but violates one asserted expression under the independent evaluator is rejected as incomplete. | Test (TC-020) |
| FR-021-AC-5 | A model that omits a declared datatype symbol, or defines an unknown symbol or a symbol twice, is not verified. | Test (TC-020) |
| FR-021-AC-6 | A model nested deeper than the NFR-010 bound or larger than the model ceiling is rejected as a limit breach and is never conclusive. | Test (TC-020) |

## Dependencies

FR-004 and FR-020, and the integer witness replay that lands first for integer-typed record fields.
