---
id: FR-022
title: "Translate enum and record analysis_claim terms from QSL contract packages"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-009
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-020
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-022: Translate enum and record analysis_claim terms from QSL contract packages

## Description

The `run-package` translation of [FR-009](./FR-009-package-runner.md) shall translate
`analysis_claim` terms that use enums, records and options into analysis expressions that
[FR-020](./FR-020-datatype-lowering.md) lowers, so data-classification and role rules written in QSL
reach the solvers. This extends the term table and value declarations of FR-009 and keeps every
other rule of that requirement (identity, anchor, spans, node and depth bounds, exit statuses).

## Behavior

- **Types and declarations.** The runner shall build the `DeclarationEnvironment` of FR-009 with
  the type declarations of the enums and records that the reached nodes name, taken from the
  package's `semantic_type` nodes, and shall not invent a variant, field or type. A type node that
  cannot be resolved to an enum, record or option over a supported payload is refused, naming the node.
- **Value declarations.** A `value/parameter` node whose `semantic_type` is an enum, record or option
  declares one `ValueDeclaration` of that type under the FR-009 naming and ordering rule, kind `Input`,
  observation `Current`.
- **Terms.** In addition to the Boolean rows of FR-009, the runner shall translate: a `Literal` of an
  enum type to `ExpressionKind::EnumLiteral`; a `Reference` to an enum, record or option typed
  `value/parameter` to `ExpressionKind::ValueReference`; and the `Application` of each catalog
  operation that denotes enum equality or inequality, enum membership in a literal set, record
  construction, field access, option construction, option presence and option access to the matching
  expression of FR-020. The operation is identified by `CheckedApplicationOperation::identity()`; an
  application with no identity, or whose identity is not one of these operations, cannot be lowered.
- **Option access.** An option-access application is translated only where the claim body holds a
  presence guard for the same option term that dominates it under the FR-020 presence-proof rule. An
  unguarded access exits `2` naming the node.
- **Refusals.** A term that uses a collection, text, a rational, ordering of enums, or a record or
  option that nests a collection or text exits `2` naming the node `semantic_graph.nodes[N]` at which
  the translation stopped. No datatype term is approximated. The node and depth bounds of FR-009 count
  datatype expressions like any other `Expression`.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-022-AC-1 | A built contract-package fixture with an enum-membership `analysis_claim` and one with a record-field `analysis_claim` each run end to end through `run-package`, with real solvers where installed, and exit `0` or `1` as the independent expectation requires. | Test (TC-020, TC-015) |
| FR-022-AC-2 | Every enum variant, record field and type in the lowered query comes from the package's `semantic_type` nodes, and a fixture naming an unresolved type exits `2` naming the node. | Test (TC-020) |
| FR-022-AC-3 | An unsupported datatype term (collection, text, rational, enum ordering, a record nesting a collection or text, an unguarded option access) exits `2` naming `semantic_graph.nodes[N]`, writes no report and invokes no solver. | Test (TC-020) |
| FR-022-AC-4 | Datatype terms are translated by a pure function of the admitted package: the same bytes yield the same clause order, declarations and query. | Test (TC-020) |
| FR-022-AC-5 | Datatype expressions count toward the FR-009 node and depth bounds, and a shared enum or record chain that exceeds them exits `2` without expanding the refused tree. | Test (TC-020) |

## Dependencies

FR-009, FR-020, and the integer term translation that lands first. Contract-package fixtures are
built at test time (FR-009-AC-27) and none is committed.
