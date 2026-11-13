---
id: FR-011
title: "Replay integer witness models against every assertion"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-004
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-010
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-011: Replay integer witness models against every assertion

## Description

The analyzer shall verify an integer-valued model returned by a solver by re-evaluating every named
assertion of the query against that model before the model is published as a witness. Replay is
performed by the analyzer on the typed analysis expressions, not by the solver that produced the
model, so a witness is never accepted on a solver's own word. This extends the Boolean replay of
[FR-004](./FR-004-conclusions.md) to the integer values of FR-010.

## Behavior

- **Model parsing.** The analyzer parses the model of each solver (Z3 and cvc5) for `Int` symbols,
  accepting a non-negative numeral and the SMT-LIB negation of a numeral for a negative value, and
  assigning each value to the declared query symbol it names. Boolean and integer symbols can occur in
  one model. A numeral with a decimal point, a real or other sort, or an exponent form is malformed.
  A numeral of more than 1,024 characters is malformed and is refused before conversion.
- **Numeral boundary.** A well-formed integer numeral of at most 1,024 characters whose value does not
  fit the 128-bit signed range is `value_out_of_domain`, because every declared integer domain lies
  inside that range.
- **Complete and exact symbol set.** A verified model defines every declared integer and Boolean
  symbol of the query exactly once and defines no other symbol. The named-assertion aliases of the
  query that a solver may echo in a returned model are accepted and skipped, as the Boolean parser
  does; they are not values and not symbols for this rule. A model text that is not a sequence of
  `define-fun` entries in the Z3 or cvc5 shape is `value_malformed` and names no symbol.
- **Domain.** Each integer value must lie within the declared minimum and maximum of its symbol. The
  query's range constraints are unnamed assertions (FR-010), so this check, not an assertion
  replay, is the domain verdict.
- **Replay.** Every replay assertion is evaluated on the analysis expression with exact 128-bit signed
  arithmetic that detects overflow; an evaluation that overflows fails replay and never wraps. `evaluation_overflow` is a defensive reason: IR validation refuses an admitted
  clause whose intermediate results can leave the 128-bit range, so no admitted fixture reaches it, and
  it is verified by a unit test over an unchecked replay expression. Every assertion must evaluate true.
- **Typed failure and precedence.** A model that does not verify is reported with exactly one reason
  from the closed set `value_malformed`, `symbol_repeated`, `symbol_unknown`, `sort_mismatch`,
  `symbol_missing`, `bound_undecodable`, `value_out_of_domain`, `evaluation_overflow`,
  `replay_limit_exceeded` and `assertion_false`. When several apply, the first in that order is
  reported. For symbol-related reasons, the first symbol in ascending symbol order is named; for
  `bound_undecodable`, the first undecodable bound symbol is named; and for `evaluation_overflow` or
  `assertion_false`, the first assertion in assertion-map order is named. A replay that exceeds its
  instance limit reports `replay_limit_exceeded` as defined by
  [FR-033](./FR-033-quantified-assertion-replay.md) and
  [NFR-031](../nonfunctional/NFR-031-quantifier-replay-bound.md).
- **Incomplete, never verified.** A model that fails replay, or that omits an integer symbol, leaves
  the logical response of the solver unchanged but marks the requested witness evidence incomplete
  (as FR-004 states). It is never published as a verified witness and no value is invented to fill a
  gap.
- **Published values.** A verified integer value preserves every complete origin identity and explicit
  binding group of its symbol from the variable map, as Boolean values do.
- **Datatype-sorted symbols.** Enum, record and option symbols are replayed by
  [FR-021](./FR-021-datatype-witness-replay.md), whose failures use this requirement's ten reasons.
- **No in-house proving.** Replay checks one concrete assignment. It does not search for, repair or
  derive a model.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-011-AC-1 | A model returned by Z3 and a model returned by cvc5 for a satisfiable integer fixture are each parsed, every named assertion is re-evaluated true, and the published witness lists every symbol with its integer value and origins. | Test (TC-017, real solvers) |
| FR-011-AC-2 | For recorded model texts in each solver's output format (including negative values, a model that mixes Boolean and integer symbols, and a model that echoes named-assertion aliases) the parser yields the expected symbol-to-value assignment and skips the aliases. | Test (TC-017) |
| FR-011-AC-3 | A model for which one assertion evaluates false is reported incomplete with reason `assertion_false` naming that assertion, never as a verified witness. | Test (TC-017) |
| FR-011-AC-4 | A model that omits one declared integer symbol is reported incomplete with reason `symbol_missing`; a model with an undeclared symbol, a repeated symbol, or an integer value for a Boolean symbol (or the reverse) is reported with `symbol_unknown`, `symbol_repeated` or `sort_mismatch`. | Test (TC-017) |
| FR-011-AC-5 | A value one above the declared maximum, one below the declared minimum, and a value exactly at each bound: the first two fail with `value_out_of_domain`, the bound values verify. | Test (TC-017) |
| FR-011-AC-6 | A decimal, a real-sorted and an exponent-form numeral, and a numeral of 1,025 characters, each fail with `value_malformed` without panicking; a numeral of 1,024 characters, and the value 2^127, each fail with `value_out_of_domain`. | Test (TC-017) |
| FR-011-AC-7 | A unit test of the replay evaluator over an unchecked expression (constructed directly, not an admitted fixture) `x + 1 > 0` with `x` at the 128-bit signed maximum fails with `evaluation_overflow`, not a wrapped result. | Test (TC-017) |
| FR-011-AC-8 | In every failing case above the logical status of the solver response (`sat`) and the adapter outcome remain available and the witness evidence is marked incomplete. | Test (TC-017) |
| FR-011-AC-9 | Replay reads only the typed analysis expressions and the parsed assignment; mutating the SMT-LIB query text of a lowered query does not change a replay verdict. | Test (TC-017) |
| FR-011-AC-10 | A model with a missing symbol and a malformed numeral reports `value_malformed`; a model with an out-of-domain value and a false assertion reports `value_out_of_domain`; in the same unchecked-expression unit test, an overflowing assertion before a false one in assertion-map order reports `evaluation_overflow`, and the reverse order reports `assertion_false`. | Test (TC-017) |
| FR-011-AC-11 | With real solvers, the satisfiable integer fixture of FR-010-AC-2 is decided `sat` by Z3 and by cvc5, each returned model replay-verifies, and the differential comparison reports agreement with agreed status `satisfied`. | Test (TC-017, real solvers) |

## Dependencies

FR-004 and FR-010.
