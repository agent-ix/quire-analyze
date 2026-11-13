---
id: SR-17040
title: "Failure-domain review of VER-84 datatype lowering, replay and bounds"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-analyze; quire-analyze#88 diff origin/main...HEAD: spec/functional/FR-020-datatype-lowering.md, spec/functional/FR-021-datatype-witness-replay.md, spec/functional/FR-022-datatype-package-translation.md, spec/nonfunctional/NFR-010-datatype-bounds.md; context src/smt.rs, src/package_translation.rs, quire-contract-model check_expression/check_unwrap/DeclarationEnvironment::new"
review_set: subset
---

## Summary

Ticket: VER-84. This lens checks the Unwrap soundness argument, symbol identity, model-value
parsing, and the reachability of the refusal paths. The argument is that unguarded `Unwrap` is
excluded upstream. That holds: `AnalysisStatement` construction runs
`DeclarationEnvironment::check_expression` (`src/smt.rs:205`), and `check_unwrap` refuses any
`Unwrap` with no `OptionSome` operand and no `Present` fact. Facts come only from
`ShortCircuitAnd`, `Implication` and `ShortCircuitOr`, never from `TotalAnd`/`TotalOr`.
Lowering `Unwrap` to the `some` selector is therefore sound for sat/unsat: the selector's value on
`none` cannot affect the guarded formula. The defects are elsewhere.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-020-AC-4 requires an unguarded `Unwrap` to be refused as `UnsupportedConstruct`, but no such statement can reach lowering. `AnalysisStatement` construction already fails it as `InvalidStatement` (`src/smt.rs:205-222`, from `check_unwrap`'s `OptionPresence` obligation). As written, the AC names a code the real path never yields, and it tests an unreachable branch. Fix: assert that the statement is refused as `InvalidStatement` at construction, with no query, and treat the lowering-time check as defensive. | spec/functional/FR-020-datatype-lowering.md:68-73, FR-020-AC-4 |
| FND-002 | medium | FR-021 gives the parser's accepted spellings as raw user names: `Red`, `(as Red Role)`, `(mk-User Admin true)`, `(as none (Option Role))`. FR-020 instead requires sort, constructor and selector symbols that injectively encode type identity, and one monomorphic option datatype per payload. The solver therefore prints the encoded (likely bar-quoted) symbols, and `(Option Role)` is a parametric sort the encoding never declares. An implementer cannot tell which grammar the parser accepts. Fix: state the spellings in terms of FR-020's encoded symbols, and drop the parametric `(Option T)` form or make FR-020 use it. | spec/functional/FR-021-datatype-witness-replay.md:24-28 |
| FND-003 | medium | FR-021's independent evaluator has no rule for `Unwrap` of an absent option. A model with `x = none` makes `(is-some x)` false, and the guarded subterm must not be evaluated. The solver's model may still assign the `some` selector on `none` a value, and an evaluator that reads it, or that evaluates both sides of the guard, gives a spurious mismatch or a fabricated value. Fix: state that the evaluator uses the short-circuit semantics of FR-004, never evaluates `Unwrap` on an absent value, and ignores any selector-on-`none` value in the model. Add an option-absent-under-guard case to FR-021-AC-3. | spec/functional/FR-021-datatype-witness-replay.md:36-39, FR-021-AC-3 |
| FND-004 | medium | FR-022 translates an option access only "where the claim body holds a presence guard … that dominates it under the FR-020 presence-proof rule". But FR-009's term table maps `quire.op.boolean.and`/`or` to `TotalAnd`/`TotalOr` (`src/package_translation.rs:651-652`), which carry no presence fact upstream. Only `quire.op.boolean.implies` can establish the guard. A QSL `present(x) and unwrap(x) == Admin` claim is therefore always refused, and FR-022 does not say so. Fix: state which connectives carry the guard. Either translate `and`/`or` to the short-circuit operators where a guard is needed, or document that only `implies` guards an access, and add that case to FR-022-AC-3. | spec/functional/FR-022-datatype-package-translation.md:39-41 |
| FND-005 | low | FR-020-AC-5 includes "a self-containing record" as a case the analyzer refuses as `unsupported`. `DeclarationEnvironment::new` already rejects record containment cycles (`RecursiveType`, `detect_record_cycle`), so no statement with such a record can be built, and the case cannot be exercised at the analyzer. Fix: drop it from AC-5, or restate it as an upstream rejection that yields no query. | FR-020-AC-5, spec/functional/FR-020-datatype-lowering.md:97-98 |
| FND-006 | medium | The NFR-010 bounds cannot be tested exactly as defined. "Datatype nesting depth (record, option, enum)" does not say whether a bare enum, Boolean or option level counts, or whether a top-level record is depth 1, so the 8/9 boundary differs between implementers. "Datatypes declared per query" does not say whether each per-payload option datatype counts. The numbers (256/64/8/256) are given no rationale, though they are far below the upstream limits (10,000 variants/fields, depth 256) that they tighten. Fix: define the counting (for example, depth of the `ValueType` tree with the root at 1, and option datatypes counted) and add one line of rationale. | NFR-010-AC-1, spec/nonfunctional/NFR-010-datatype-bounds.md:24-28 |

## Verdict

FAIL on mediums. The Unwrap-under-presence-proof design is sound. The upstream checker excludes
an unguarded `Unwrap` before lowering, and the unconstrained selector-on-`none` value is
unobservable under the guard. FND-001, FND-002, FND-003, FND-004 and FND-006 are real gaps an
implementer would trip on, and FND-005 is a redundant AC case. The symbol-injectivity rule,
dependency-ordered declarations, the refusal of text and collection nesting, and the
no-approximation rule are well defined.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-007 | medium | FR-020's Unwrap bullet now limits the presence proof to "an `IsPresent` fact established by the antecedent of an implication, or an `OptionSome` operand". Upstream `check_expression` also establishes the fact from the left operand of `ShortCircuitAnd` and the negated left operand of `ShortCircuitOr` (quire-contract-model `expression.rs:3313-3316`), and the analysis API admits such statements directly. The bullet misstates which statements reach lowering. Fix: cite the upstream rule as it is (short-circuit and, short-circuit or, implication). | spec/functional/FR-020-datatype-lowering.md:93-98 |
| FND-008 | low | The `quire.op.record.project` row does not say where the field name comes from. The catalog entry has one operand (`field_owner`) and a `field` member (`checked-operation-catalog-v1.json:175`), but FR-022-AC-1 writes it as two arguments (`record.project(user, role)`). An implementer following the AC would expect arity 2. Fix: state that the field is the operation's `field` member, with one argument. | spec/functional/FR-022-datatype-package-translation.md:42 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-020-AC-4 now expects `InvalidStatement` from `check_expression` at statement build. |
| FND-002 | fixed | FR-021 now states the spellings generically, as encoded symbols from the query symbol map, and the parametric `(Option T)` form is gone. |
| FND-003 | still-open | FR-021 makes only implication lazy. A statement guarded by `ShortCircuitAnd` or `ShortCircuitOr`, for example `IsPresent(x) and-then Unwrap(x) = A` with `x` absent, is a valid model that the evaluator would reject by evaluating `Unwrap` on `none`. The laziness rule must cover every guard form upstream admits (see new FND-007). |
| FND-004 | fixed | FR-022 now admits a guard only inside the consequent of `quire.op.boolean.implies` and explains why `and`/`or` cannot carry one. The missing `option.value` translation is recorded as SR-17038 FND-006. |
| FND-005 | fixed | FR-020-AC-5 now expects `RecursiveType` at `DeclarationEnvironment::new`, with no query. |
| FND-006 | fixed | NFR-020 now defines nesting depth (outermost datatype is depth 1, with a worked example) and the count of datatypes per query (each option payload counts once). It adds a rationale: 256×256×64 B ≈ 4 MiB, under the 16 MiB query limit, and depth 8 sits well inside expression depth 128. |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-009 | medium | FR-022 says "an unguarded `quire.op.option.value` exits `2` naming the node, as the upstream check refuses it", and FR-022-AC-3 requires the unguarded option access to exit `2` naming `semantic_graph.nodes[N]`. These conflict. A refusal by the upstream `check_expression` reaches the runner as a clause error (`TranslationError::at_clause`, which names `claim-N` at the validator's path, `src/package_translation.rs:181-187`), not a node error naming the `option.value` node. Fix: either have the translation check the implies-guard itself and refuse at the `option.value` node (`semantic_graph.nodes[N]`), or let AC-3 accept the `claim-N` clause error for this one case. | spec/functional/FR-022-datatype-package-translation.md:52-60, FR-022-AC-3 |

## Dispositions (round 2)

| FND | outcome | reason |
| --- | --- | --- |
| FND-003 | fixed | FR-021 is now lazy in the guarded operand of implication, short-circuit `and` and short-circuit `or`, matching upstream `guard_facts` (expression.rs:3313-3316): `or` skips its right operand when the left is true. FR-021-AC-7 covers all three. |
| FND-007 | fixed | FR-020's Unwrap bullet now lists every upstream guard (implication antecedent, short-circuit `and` left, short-circuit `or` left as the negated fact, `OptionSome`) and says total `and`/`or` establish none. |
| FND-008 | fixed | The `record.project` row now says one operand plus the catalog `field` member, which matches `checked-operation-catalog-v1.json:175`, and FR-022-AC-1 writes `record.project(user)` with field `role`. |
