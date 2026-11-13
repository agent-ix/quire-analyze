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
- **Terms.** In addition to the Boolean rows of FR-009, the runner shall translate exactly these,
  by `CheckedApplicationOperation::identity()` of the locked `quire-verification-contracts` catalog:

| Term | Result |
|---|---|
| `Literal` of an enum type | `ExpressionKind::EnumLiteral` |
| `Literal` of value kind `none` with an option type | `ExpressionKind::OptionNone` |
| `Reference` to an enum, record or option typed `value/parameter` | `ExpressionKind::ValueReference`, as for Boolean |
| `Application` of `quire.op.enum.eq` / `quire.op.enum.ne` | `Compare` with `Equal` / `NotEqual` |
| `Application` of `quire.op.structural.eq` / `quire.op.structural.ne` over operands of one enum, record or option sort | `Compare` with `Equal` / `NotEqual` |
| `Application` of `quire.op.record.project`, one operand (the record) and the catalog `field` member naming the field | `ExpressionKind::FieldAccess` |
| `Application` of `quire.op.option.value`, one operand (the option) | `ExpressionKind::Unwrap`, only under a presence proof (Option access below) |
| `Application` of `quire.op.option.present` | `ExpressionKind::IsPresent` |

- **Not translated.** The catalog has no identity for record construction or option construction
  (a present option), so a package cannot carry them and they exit `2`. No membership
  construct exists: a role-membership rule is a disjunction of `quire.op.enum.eq` applications under
  `quire.op.boolean.or`. `quire.op.collection.contains` and `quire.op.collection.set` are collection
  terms and exit `2`. An application with no identity, or an identity not in the table, cannot be
  lowered.
- **Option access.** `quire.op.option.value` is translated to `Unwrap` only where the same option
  term is proven present by the upstream rule of [FR-020](./FR-020-datatype-lowering.md): inside the
  consequent of `quire.op.boolean.implies` whose antecedent is the `quire.op.option.present` of that
  term, or inside the right operand of a short-circuit `and` (or `or`, with the negated fact) whose
  left operand establishes it. The runner translates `quire.op.boolean.and` and `or` to the total
  operators (FR-009), which establish no fact, so only the implication guard is reachable from a
  package today; a guard through a short-circuit operator needs a short-circuit identity that FR-009
  does not translate. An unguarded `quire.op.option.value` exits `2` naming the node, as the upstream
  check refuses it.
- **Refusals.** A term that uses a collection, text, a rational, ordering of enums, or a record or
  option that nests a collection or text exits `2` naming the node `semantic_graph.nodes[N]` at which
  the translation stopped. No datatype term is approximated. The node and depth bounds of FR-009 count
  datatype expressions like any other `Expression`.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-022-AC-1 | Five built contract-package fixtures run end to end through `run-package` with real solvers where installed, each with a fixed expected exit: an enum-membership disjunction (`role = Admin` or `role = Auditor`) with `role != Admin` exits `0`; the same disjunction with `role = Guest` exits `1`; a record-field claim `record.project(user)` with field `role` equal to `Admin` exits `0`; and the same with a second claim that field equal to `Guest` exits `1`. A fifth fixture, `quire.op.option.value` of a parameter inside the consequent of `implies` with `quire.op.option.present` of the same parameter as antecedent, translates to an `Unwrap` and runs; the same value without the antecedent exits `2`. | Test |
| FR-022-AC-2 | Every enum variant, record field and type in the lowered query comes from the package's `semantic_type` nodes, and a fixture naming an unresolved type exits `2` naming the node. | Test |
| FR-022-AC-3 | An unsupported datatype term (collection, text, rational, enum ordering, a record nesting a collection or text, an unguarded option access) exits `2` naming `semantic_graph.nodes[N]`, writes no report and invokes no solver. | Test |
| FR-022-AC-4 | Datatype terms are translated by a pure function of the admitted package: the same bytes yield the same clause order, declarations and query. | Test |
| FR-022-AC-5 | Datatype expressions count toward the FR-009 node and depth bounds, and a shared enum or record chain that exceeds them exits `2` without expanding the refused tree. | Test |

## Dependencies

FR-009, FR-020, and the integer term translation that lands first. Contract-package fixtures are
built at test time (FR-009-AC-27) and none is committed.
