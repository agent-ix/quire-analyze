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
