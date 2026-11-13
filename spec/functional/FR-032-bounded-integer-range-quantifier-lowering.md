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
- Analysis-model input. The contract model has no range-domain quantifier form, so a range reaches
  lowering as an analysis-model formula attached to a statement:
  - `AnalysisFormula` is a Boolean tree whose leaves are quantifier-free contract expressions
    (`Term`) and whose nodes are `Not`, `And`, `Or`, `Implies`, `Range` and `Domain`. `Range` holds
    a `RangeQuantifier`; `Domain` holds the `DomainQuantifier` defined by
    [FR-031](./FR-031-enum-and-collection-quantifier-lowering.md). A leaf names a
    quantifier's bound variable with `LocalReference`; the nearest enclosing quantifier that binds
    that name wins, so an inner quantifier that reuses a name shadows the outer one, and a
    `ValueReference` of the same name is a different, free variable that is never captured.
  - `RangeQuantifier` holds the quantifier's source `identity` (a non-empty string chosen by the
    caller), its kind (`ForAll` or `Exists`), the bound variable's name and its `local_type` (an
    integer type), the `lower` and `upper` `RangeBound`s, and the body formula.
  - `RangeBound` is `Literal(i128)`, `Declared(name)` or `Unbounded`. A **declared bound** is the
    value of a bounded integer input of the statement's declarations, read at the current
    observation: the bound is that input's value, not its minimum or maximum. It is finite because
    the input's declared integer type is finite, and the solver, not the lowering, chooses its
    value. `Unbounded`, a name that is not declared, and a name that is not an integer input are
    not provably finite.
  - `StatementInput::with_formula` attaches a formula to a statement built from a clause. The
    clause supplies the statement's identity, so the assertion map and the report attribute the
    assertion to that clause and its digest exactly as for any statement; the formula replaces the
    clause body as the lowered body, and the assertion symbol also binds the formula, so a
    different formula over one clause is a different assertion. `with_formula` refuses, as
    `InvalidStatement`, an empty quantifier identity, a bound-variable reference with no enclosing
    quantifier, a non-empty literal range that lies outside `local_type`, and a declared bound
    whose input type lies outside `local_type`. For a `DomainQuantifier`, it also checks the
    domain/local-type match and each finite-collection member as defined in
    [FR-031](./FR-031-enum-and-collection-quantifier-lowering.md) and
    [FR-030-AC-6](./FR-030-bounded-quantifier-domains.md). Whether a range bound is finite is
    decided at lowering.
  - `QueryBundle::quantifiers` is the quantifier map of [FR-030](./FR-030-bounded-quantifier-domains.md).
    Its entry carries either range bounds or typed domain members; these alternatives cannot both
    be present.
- Budget order. The budget of [NFR-030](../nonfunctional/NFR-030-quantifier-expansion-bound.md) is
  spent in canonical order: statements in ascending assertion-symbol order, and within a statement
  the quantifiers in depth-first pre-order (a quantifier before the quantifiers in its body,
  siblings left to right). The quantifier map lists entries in that order, so the encodings do not
  depend on the order in which the caller lists the statements.
- Instance identity. Range expansion records its lower and upper bounds in the quantifier map;
  enum and collection expansion records typed members in instance order. Replay identifies a
  failing instance under [FR-033](./FR-033-quantified-assertion-replay.md).
- Replay. Expanded ranges are quantifier-free. A bounded solver quantifier is replayed over its
  original range by [FR-033](./FR-033-quantified-assertion-replay.md). Replay failures and limits
  leave the solver's logical response unchanged and never publish the model as verified.
- Package path: until the upstream contract model and operation catalog define a range-domain
  quantifier operand, no QSL package reaches the bounded solver quantifier; a package range arrives
  as a literal integer collection and is expanded or refused as `expansion-ceiling`
  ([FR-034](./FR-034-quantifier-package-translation.md)). The upstream range-domain form is a blocking
  dependency of the package path for the bounded-solver-quantifier encoding only; the encoding itself
  is specified and tested at the lowering layer.
- Logic: a query holding a bounded solver quantifier declares the logic of its feature set
  ([FR-020](./FR-020-datatype-lowering.md)) with the quantifier-free prefix removed: `LIA` when it
  declares no datatype and `UFDTLIA` when it declares at least one (an enum, record or option in the
  body). Every other query keeps `QF_LIA` or `QF_UFDTLIA`. The logic is part of the one profile
  `quire.smtlib2/v2`, not a second profile.
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
| FR-032-AC-6 | A bounded quantifier whose body mentions an enum declares `UFDTLIA`, and one whose body is integer-only declares `LIA`. | Test (TC-030) |
| FR-032-AC-7 | `with_formula` refuses an empty quantifier identity, a bound-variable reference with no enclosing quantifier, a non-empty literal range outside `local_type` and a declared bound outside `local_type`, each as `InvalidStatement`. | Test (TC-030) |
| FR-032-AC-8 | A satisfiable model of a query containing a bounded solver quantifier is replayed over the original range as specified by FR-033; it is published as verified only when replay succeeds, and the solver's logical response is unchanged. | Test (TC-030) |
| FR-032-AC-9 | A contested budget gives the same encodings and the same quantifier map for any order of the caller's statements, and spends the budget in depth-first pre-order within a statement. | Test (TC-030) |

## Dependencies

[FR-030](./FR-030-bounded-quantifier-domains.md), [FR-002](./FR-002-smt-lowering.md) and
[FR-010](./FR-010-linear-integer-arithmetic.md).
