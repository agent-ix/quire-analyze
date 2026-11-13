---
id: FR-012
title: "Translate integer analysis_claim terms from QSL contract packages"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-009
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-010
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-012: Translate integer analysis_claim terms from QSL contract packages

## Description

The `run-package` runner of [FR-009](./FR-009-package-runner.md) shall translate an `analysis_claim`
whose body uses integer values, linear arithmetic or ordering into an analysis statement that
FR-010 lowers, instead of exiting `2` for it. The runner stays QSL-fed: the terms come from the
contract package, and the runner adds no source language and no solver.

## Behavior

- **Terms added.** In addition to the Boolean terms of FR-009, the translation accepts the terms below.
  A comparison yields a Boolean term that Boolean operations may combine. The translation of a
  `Reference` follows FR-009. An integer-typed operand of a Boolean operation, or a Boolean operand of
  an integer operation, is refused.

| Term | Result |
|---|---|
| `Literal` of integer value kind with an integer type | `ExpressionKind::IntegerLiteral` with that integer type |
| `Reference` to a `value/parameter` node whose `semantic_type` is an `integer_range` | `ExpressionKind::ValueReference`, name `nN`, observation `Current` |
| `Application` of `quire.op.integer.negate` | `ExpressionKind::NumericNegate` |
| `Application` of `quire.op.integer.add` | `ExpressionKind::Numeric` with `NumericOperator::Add` |
| `Application` of `quire.op.integer.sub` | `ExpressionKind::Numeric` with `NumericOperator::Subtract` |
| `Application` of `quire.op.integer.mul` | `ExpressionKind::Numeric` with `NumericOperator::Multiply` |
| `Application` of `quire.op.integer.eq` | `ExpressionKind::Compare` with `ComparisonOperator::Equal` |
| `Application` of `quire.op.integer.ne` | `ExpressionKind::Compare` with `ComparisonOperator::NotEqual` |
| `Application` of `quire.op.integer.lt` | `ExpressionKind::Compare` with `ComparisonOperator::Less` |
| `Application` of `quire.op.integer.le` | `ExpressionKind::Compare` with `ComparisonOperator::LessEqual` |
| `Application` of `quire.op.integer.gt` | `ExpressionKind::Compare` with `ComparisonOperator::Greater` |
| `Application` of `quire.op.integer.ge` | `ExpressionKind::Compare` with `ComparisonOperator::GreaterEqual` |

- **Parameters.** A parameter node yields one `ValueDeclaration` of kind `Input` named by the node
  index, with the integer type (domain, minimum, maximum and overflow policy) of its `integer_range`; the 128-bit signed bounds of the IR apply. Claims that reference the same
  node share one declaration, as FR-009 states for Boolean parameters.
- **Non-linear and other integer terms.** A multiplication whose operands both contain a parameter, a
  `quire.op.integer.div`, `quire.op.integer.rem` or `quire.op.integer.mod` application, and any integer operation not listed above
  still make the runner exit `2` naming the node as `semantic_graph.nodes[N]`, with no report.
- **Same bounds.** The depth, node-count and shared-subterm bounds of FR-009 apply unchanged to the
  integer terms.
- **Determinism.** Translating the same package bytes yields the same clause order, identifiers,
  declarations, integer bounds and source spans.
- **End to end.** A package with an integer claim runs through lowering, both solvers executed by
  `producer-execute`, integer witness replay (FR-011) and report publication as FR-009 describes, and
  the published report is a `quire.differential-report/v2` document.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-012-AC-1 | A package fixture whose `analysis_claim` bounds a dose by a maximum for a weight (a linear inequality over two integer parameters, for example `dose <= 5 * weight`) runs end to end with controlled solver results and exits `0` with a report that `validate_report_document` accepts. | Test (TC-018) |
| FR-012-AC-2 | The same fixture with contradictory claims exits `1` with agreed status `refuted`. | Test (TC-018) |
| FR-012-AC-3 | A claim whose body multiplies two parameters, one that applies `quire.op.integer.div`, `.rem` or `.mod`, and one with a reference cycle each exit `2` naming `semantic_graph.nodes[N]` of the node where translation stopped, with no report. | Test (TC-018) |
| FR-012-AC-4 | Each row of the Terms added table translates to the stated expression kind and operator, a comparison nests under a Boolean operation, and an integer operand of a Boolean operation exits `2` naming the node. | Test (TC-018) |
| FR-012-AC-5 | Two claims that reference the same integer parameter node yield one declaration with the integer bounds of its type; two distinct parameter nodes yield two. | Test (TC-018) |
| FR-012-AC-6 | Two translations of the same package bytes yield equal clause order, identifiers, declarations, bounds and spans. | Test (TC-018) |
| FR-012-AC-7 | The 10,000/10,001 node bound, the depth 128/129 bound and the shared-subterm chain of FR-009-AC-23, rebuilt over integer sums, behave as in FR-009-AC-23. | Test (TC-018) |
| FR-012-AC-8 | With real solvers, the fixtures of FR-012-AC-1 and AC-2 yield the same exit statuses and agreed statuses as the controlled run. | Test (TC-018, real solvers) |

## Dependencies

FR-009, FR-010 and FR-011.
