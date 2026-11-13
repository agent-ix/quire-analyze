---
id: FR-032
title: "Lower quantifiers over bounded integer ranges"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-010
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-032: Lower quantifiers over bounded integer ranges

## Description

The analyzer shall lower a universal or existential quantifier over an inclusive integer range with
literal or declared bounds by the encoding that the rule below selects, expansion or a bounded
solver quantifier.

## Behavior

- Encoding choice, per quantifier, using the instance budget of
  [NFR-030](../nonfunctional/NFR-030-quantifier-expansion-bound.md):
  - when both bounds are literals and the quantifier's instance count fits the remaining budget, the
    quantifier is expanded exactly as in [FR-031](./FR-031-enum-and-collection-quantifier-lowering.md);
  - otherwise (a declared bound, or an instance count above the remaining budget) the quantifier is
    emitted as a bounded solver quantifier whose guard conjoins the lower and upper bound to the bound
    variable (`forall` with an implication guard, `exists` with a conjunction guard). A bounded
    quantifier consumes no budget.
  - A range wider than the budget is therefore never `unsupported`; it takes the bounded form.
- Logic: a query holding a bounded solver quantifier declares the quantified linear-integer logic
  that the capability contract names for it, with the datatype sorts of
  [FR-020](./FR-020-datatype-lowering.md) included when the body mentions an enum, record or option.
  The logic is part of the one profile `quire.smtlib2/v2`, not a second profile.
- Decidability argument: expansion is quantifier-free linear integer arithmetic. Quantified linear
  integer arithmetic is decidable in principle (Presburger arithmetic), independent of the interval
  guards; the guards keep every quantifier finite, which makes the instance count checkable and
  expansion possible, but they are not the source of decidability. A solver is not required to
  decide such a query within its limits and may answer `unknown`, which is inconclusive under
  [NFR-032](../nonfunctional/NFR-032-quantifier-exhaustion.md). No decidability claim is made for a
  bounded quantifier whose body mentions datatypes.
- A declared bound is admitted only when its declaration gives a finite lower and upper value; a
  bound that is not provably finite returns `unsupported` with reason `non-finite-bound`. Range
  width and instance counts are computed with checked arithmetic as NFR-030 states.
- The quantifier-map entry of [FR-030](./FR-030-bounded-quantifier-domains.md) records the encoding
  chosen, the lower and upper bound, and the source identity.
- The body's integer arithmetic follows the exact integer encoding of
  [FR-010](./FR-010-linear-integer-arithmetic.md) and its overflow rule (the saturating policy is
  `unsupported`); a body construct without an exact encoding returns `unsupported` and is not
  approximated.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-032-AC-1 | For a universal and an existential fixture over a literal range within the budget, the lowered query equals its golden query and the status derived by the enumeration oracle equals the recorded expected status; where Z3 and cvc5 are installed, both answer the matching `sat`/`unsat`. | Test (TC-030; real solvers where installed) |
| FR-032-AC-2 | A fixture with declared bounds lowers to the bounded form with the expected golden query, and a range with a bound that is not provably finite returns `unsupported` with reason `non-finite-bound`. | Test (TC-030) |
| FR-032-AC-3 | A literal range whose instance count fits the budget is expanded, and one exceeding it is emitted as a bounded quantifier and not refused; the quantifier map records each choice. | Test (TC-030) |
| FR-032-AC-4 | In the bounded form every quantifier carries a finite interval guard; a quantifier without one is never emitted. | Test (TC-030) |
| FR-032-AC-5 | Lowering a range-quantified package twice yields byte-identical SMT-LIB2 output. | Test (TC-030) |
| FR-032-AC-6 | A bounded quantifier whose body mentions an enum declares a logic that includes datatype sorts, and one whose body is integer-only does not. | Test (TC-030) |

## Dependencies

[FR-030](./FR-030-bounded-quantifier-domains.md), [FR-002](./FR-002-smt-lowering.md) and
[FR-010](./FR-010-linear-integer-arithmetic.md).
