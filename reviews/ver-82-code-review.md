---
id: SR-17052
title: "Code review of VER-82 integer witness replay (quire-analyze#95)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-analyze; src/analysis.rs, src/smt.rs, tests/integer_lowering.rs (git diff origin/main...HEAD)"
review_set: subset
relationships:
  - { target: "ix://agent-ix/quire-analyze/FR-011", type: reviews }
---
# Code Review of VER-82 Integer Witness Replay

## Summary

Ticket: VER-82. PR quire-analyze#95, two commits over origin/main, three files.
Includes the rust-review lane (rust-style loaded). The PR replaces the
`Unreplayed` placeholder with typed `ReplayTerm`/`Compare` replay over checked
i128, adds a two-sort model parser for the Z3 and cvc5 shapes, the closed
eight-reason `ReplayFailureReason`, direct declared-range checks, and
`VerifiedBooleanModel::integer_values()` / `AnalysisConclusion::replay_failure()`.

Measured on the PR head with `cargo +1.98.1` (target/ in the worktree, debug=0,
-j4): lib 24 passed; analysis_semantics 9 passed, 1 ignored (real solvers);
integer_lowering 19 passed, 1 ignored (real solvers); smt_lowering 17,
solver_adapters 12, package_translation 7 passed; clippy --all-targets
--all-features -D warnings clean. The real-solver test is a genuine `#[ignore]`;
Z3/cvc5 are not installed and it was not run.

## Verdict

**CHANGES REQUESTED.** Parsing of both solver spellings, `(- n)` negatives,
alias skipping, the 1,024-character bound before conversion, i128-boundary
out-of-domain, Real/decimal/exponent/leading-zero malformed, direct range
checks, symbol_missing in ascending order, assertion-map ordering of
overflow/false, Incomplete-never-verified with status kept `sat`, and
verified-only sat agreement are all correct. The eager Boolean evaluation
(FND-001) makes a defensive reason reachable on admitted clauses and must
become short-circuit before FR-021 builds on it. Precedence is wrong when a
malformed value sits on an unknown, repeated or mis-sorted entry (FND-002).
Structural unreadable models as `value_malformed` with no subject is within
FR-011 (see SR-17053 FND-001 for the missing spec line).

Merge-conflict hotspots with quire-analyze#94 (VER-85): #94 still constructs
`ReplayExpression::Unreplayed` (removed here), so whichever lands second must
port those sites; both add fields to `QueryBundle` and its construction in
`lower_statement_specs`; both touch `LowerState` comparison/emit. After both
land, `verify_model` derives sort as Int-if-ranged-else-Bool, so enum symbols
from #94 would be read as Bool and fail with sort_mismatch until FR-021 replay
extends `Sort`.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `ReplayExpression::evaluate` evaluates both operands of And/Or/Implies eagerly. quire-contract-model admits overflow-prone arithmetic in the right operand of ShortCircuitAnd/Or/Implication using guard facts from the left (expression.rs guard_facts), so an admitted clause such as `x < MAX -> x + 1 > x` over an i128-extreme type replays a correct solver model x = MAX as evaluation_overflow / Incomplete and loses differential agreement. This contradicts FR-011 ("no admitted fixture reaches it") and FR-021's lazy-evaluation rule (FR-021-AC-7), which reuses this evaluator. Use short-circuit evaluation. | `src/smt.rs:584-586` |
| FND-002 | medium | `record_value` returns on unknown, repeated or sort-mismatched entries before reading the value, so `(define-fun z () Int 1.5)`, a repeat of x with `1.5`, or a Bool symbol given `1.5` under `Int` report symbol_unknown / symbol_repeated / sort_mismatch instead of value_malformed, violating FR-011 precedence. No test combines a malformed value with those defects. | `src/analysis.rs:855-875` |
| FND-003 | low | `QueryBundle::query` is made `pub(crate)` only so a unit test can overwrite it; crate code can now desynchronise the query text from `query_digest`. A `#[cfg(test)]` constructor or setter keeps the field private. | `src/smt.rs:618` |
| FND-004 | low | Unit-test helper `defect()` returns `ValueOutOfDomain` whenever the parse has no defect, even with `out_of_domain == None`, so the at-limit, 2^127 and `(- 2^127+1)` assertions in `numeral_boundaries` would pass if the parser returned a value. Assert on `parsed.out_of_domain` directly. | `src/analysis.rs:1162-1171` |
| FND-005 | low | New public items (`MappedIntegerValue` and its accessors, `ReplayFailureReason::as_str`, `VerifiedBooleanModel::integer_values`) carry no doc comment; rust-style asks for docs on every `pub` item (the neighbouring Boolean items share the gap). | `src/analysis.rs:388-412` |
