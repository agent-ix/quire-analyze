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
- **Complete and exact symbol set.** A verified model defines every declared integer and Boolean
  symbol of the query exactly once and defines no other symbol.
- **Domain.** Each integer value must lie within the declared minimum and maximum of its symbol.
- **Replay.** Every replay assertion, including the declared-range assertions, is evaluated on the
  analysis expression with exact arithmetic that detects overflow; an evaluation that overflows
  fails replay and never wraps. Every assertion must evaluate true.
- **Typed failure.** A model that does not verify is reported with one reason from the closed set
  `symbol_missing`, `symbol_unknown`, `symbol_repeated`, `sort_mismatch`, `value_malformed`,
  `value_out_of_domain`, `assertion_false` and `evaluation_overflow`. The reason names the symbol or
  assertion where one applies.
- **Incomplete, never verified.** A model that fails replay, or that omits an integer symbol, leaves
  the logical response of the solver unchanged but marks the requested witness evidence incomplete
  (as FR-004 states). It is never published as a verified witness and no value is invented to fill a
  gap.
- **Published values.** A verified integer value preserves every complete origin identity and explicit
  binding group of its symbol from the variable map, as Boolean values do.
- **No in-house proving.** Replay checks one concrete assignment. It does not search for, repair or
  derive a model.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-011-AC-1 | A model returned by Z3 and a model returned by cvc5 for a satisfiable integer fixture are each parsed, every named assertion is re-evaluated true, and the published witness lists every symbol with its integer value and origins. | Test (TC-017, real solvers) |
| FR-011-AC-2 | For recorded model texts in each solver's output format (including negative values and a model that mixes Boolean and integer symbols) the parser yields the expected symbol-to-value assignment. | Test (TC-017) |
| FR-011-AC-3 | A model for which one assertion evaluates false is reported incomplete with reason `assertion_false` naming that assertion, never as a verified witness. | Test (TC-017) |
| FR-011-AC-4 | A model that omits one declared integer symbol is reported incomplete with reason `symbol_missing`; a model with an undeclared symbol, a repeated symbol, or an integer value for a Boolean symbol (or the reverse) is reported with `symbol_unknown`, `symbol_repeated` or `sort_mismatch`. | Test (TC-017) |
| FR-011-AC-5 | A value one above the declared maximum, one below the declared minimum, and a value exactly at each bound: the first two fail with `value_out_of_domain`, the bound values verify. | Test (TC-017) |
| FR-011-AC-6 | A decimal, a real-sorted, an exponent-form and an over-long numeral each fail with `value_malformed` without panicking. | Test (TC-017) |
| FR-011-AC-7 | An assertion whose evaluation overflows the exact arithmetic range fails with `evaluation_overflow`, not a wrapped result. | Test (TC-017) |
| FR-011-AC-8 | In every failing case above the logical status of the solver response (`sat`) and the adapter outcome remain available and the witness evidence is marked incomplete. | Test (TC-017) |
| FR-011-AC-9 | Replay reads only the typed analysis expressions and the parsed assignment; mutating the SMT-LIB query text of a lowered query does not change a replay verdict. | Test (TC-017) |

## Dependencies

FR-004 and FR-010.
