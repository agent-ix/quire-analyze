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

- **Terms added.** In addition to the Boolean terms of FR-009, the translation accepts: a `Literal` of
  integer value kind with an integer type; a `Reference` to a `value/parameter` node whose
  `semantic_type` is an `integer_range`; an `Application` of the integer operations of the checked
  operation catalog for negation, addition, subtraction, multiplication, equal, not equal, less, less
  or equal, greater and greater or equal (for example `quire.op.integer.lt`). They translate to the
  integer expression kinds of FR-010 with the operator the operation names; a comparison yields a
  Boolean term that Boolean operations may combine.
- **Parameters.** A parameter node yields one `ValueDeclaration` of kind `Input` named by the node
  index, with the integer type and bounds of its `integer_range`. Claims that reference the same
  node share one declaration, as FR-009 states for Boolean parameters.
- **Non-linear and other integer terms.** A multiplication whose operands both contain a parameter, an
  integer division, remainder or modulo application, and any integer operation not listed above
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
| FR-012-AC-3 | A claim whose body multiplies two parameters, one that applies integer division, remainder or modulo, and one with a reference cycle each exit `2` naming `semantic_graph.nodes[N]` of the node where translation stopped, with no report. | Test (TC-018) |
| FR-012-AC-4 | Each integer operation listed under Terms added translates to the stated expression kind and operator, and a comparison nests under a Boolean operation. | Test (TC-018) |
| FR-012-AC-5 | Two claims that reference the same integer parameter node yield one declaration with the integer bounds of its type; two distinct parameter nodes yield two. | Test (TC-018) |
| FR-012-AC-6 | Two translations of the same package bytes yield equal clause order, identifiers, declarations, bounds and spans. | Test (TC-018) |
| FR-012-AC-7 | The 10,000/10,001 node bound, the depth 128/129 bound and the shared-subterm chain of FR-009-AC-23, rebuilt over integer sums, behave as in FR-009-AC-23. | Test (TC-018) |
| FR-012-AC-8 | With real solvers, the fixtures of FR-012-AC-1 and AC-2 yield the same exit statuses and agreed statuses as the controlled run. | Test (TC-018, real solvers) |

## Dependencies

FR-009, FR-010 and FR-011.
