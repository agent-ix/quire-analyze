---
id: SR-17080
title: gap-analysis of VER-95 code PR 104
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-analyze; src/, tests/, crates/method-contract/, spec/; VER-95
  PR #104'
review_set: subset
relationships:
- target: ix://agent-ix/quire-analyze/FR-042
  type: references
---

## Summary

Reviewed frozen PR #104 head 673863e0a8d32d1964f8601f32e8a2df979e1bbf against origin/main cad3fdc03849d4147288adffb9c9b73d9bc76766. Ticket: VER-95. This is the code PR; SR-17070 belongs to the already merged spec PR #103, so these artifacts use distinct pr104 slugs. No branch edits or merge were performed.

## Verdict

FAIL — blocking findings remain.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | Restore every required passing fit case. The replacement fit array removes the second m-to-t reset +100 from the old multi-transition fixture, leaving only one edge per target, and contains no duplicate/reordered-transition fixture. Thus the reported passing sweep does not exercise the same-target join or order/duplication obligations in TC-041 step 2. Keep explicit case names and compare the full required set against the executed set before claiming no omissions. | src/hybrid_tool.rs:1353-1410 |
| FND-002 | medium | Existing baseline trace gap: FR-009-AC-10 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. src/report.rs:1081 names the criterion only in a prose comment above runner_maps_published_durability_unknown_to_exit_four; add an accepted trace marker to that real behavioral test. | spec/functional/FR-009-package-runner.md: |
| FND-003 | medium | Existing baseline trace gap: FR-033-AC-5 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-033-quantified-assertion-replay.md: |
| FND-004 | medium | Existing baseline trace gap: FR-033-AC-8 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-033-quantified-assertion-replay.md: |
| FND-005 | medium | Existing baseline trace gap: FR-034-AC-1 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-006 | medium | Existing baseline trace gap: FR-034-AC-2 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-007 | medium | Existing baseline trace gap: FR-034-AC-3 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-008 | medium | Existing baseline trace gap: FR-034-AC-4 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-009 | medium | Existing baseline trace gap: FR-034-AC-5 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-010 | medium | Existing baseline trace gap: FR-034-AC-6 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-011 | medium | Existing baseline trace gap: FR-034-AC-7 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |
| FND-012 | medium | Existing baseline trace gap: FR-034-AC-8 has no test binding in quire trace and no accepted trace marker in source/tests. This predates PR #104 and is not attributed to this diff. | spec/functional/FR-034-quantifier-package-translation.md: |

## Assurance Context

AP-001 (spec/assurance/AP-001-analyze-release.md) applies to this source candidate and its material false-conclusion/resource-containment scenarios. AD-001, CAC-001 and MP-001 were inspected. The hybrid result remains separate from SMT proof classifications; direct PHAVer spawning was not introduced. AD-001 still names the removed reach provider, so its boundary inventory is stale. The independent finite-set comparison and exact helper reproduction assess the selected oracle dimension, not global producer qualification. Full three-repetition MP-001 measurement records, a PHAVer-specific producer-reliance decision, cross-platform execution, structural coverage dispositions and release independence records were unavailable. No standing exception was found; release authority remains human-owned.

## Checks

The review independently ran make fmt-check lint test under the assigned locked-build wrapper, RUSTUP_TOOLCHAIN=1.98.1 and CARGO_PROFILE_DEV_DEBUG=0; exit 0. This broad command had finished when the lead clarified that review should run only focused checks, so it was not interrupted. No make ci, coverage, deny, documentation or aggregate final-candidate gate was run. The lead owns the final gate. Logs: /tmp/codex-theories/ver95-review-logs/gates.log.

Focused installed-PHAVer tests passed 4/4 in 28.86 seconds, including both unchanged F10 variants returning exactly Incomplete/tool_timeout with no enclosure, the smoke test and the current fit sweep. Evidence is recorded in focused-real-phaver.log. Public API reproductions are in reproduction.log, with probe.rs and stub executables; the exact unchanged oracle helper reproduction is in oracle-rust-probe.log. The separate finite-set comparison of interval Minkowski sums, guard intersections and additive resets passed 33,075 bounded combinations (oracle-probe.log); this checks the arithmetic independently and is not a substitute for the missing delegated fit fixtures.

The request uses an absolute PHAVer path, the model input-artifact role, producer-execute only, timeout 5,000 ms, input 16 MiB and stderr 1 MiB. The protocol caps stdout at 8 MiB, below NFR-001's 16 MiB ceiling; this is disclosed in the PR and the test expects the producer cap. NFR-001 is unchanged. The author source archive phaver/src/phaver.cpp:86-90 documents the -vXXXXXX output-detail and timer/progress flags; -v000000 is encoded as a literal argument. The author page and source archive are https://www-verimag.imag.fr/~frehse/phaver_web/index.html and https://www-verimag.imag.fr/~frehse/phaver_web/phaver.zip. PR #104 records version 0.38, GPLv2 licensing and external-execution evidence. No PHAVer source/binary is vendored in the repository.

The removed reach/replay_hybrid/bind_hybrid_result/hybrid_method_outcome symbols are absent. Canonical synthesis assertions are preserved and pass. No CI, Cargo or Makefile gate was changed. No fallback selects the old algorithm.

## Coverage

Plan completion: not assessed

Reconciliation: quoin matrix 0.30.1 and quire 0.45.0 / engine 0.60.0, followed by quire trace --scope . --id FR- --prefix --format json and the corresponding NFR- command, plus source inspection. Matrix raw output reports 203 untagged records and 20 method-without-symbol records (including NFR measurement rows); these are not treated as 203 real missing tests because the #[trace] matrix regression is recorded as PLAT-1672. Trace resolves 359 FR and 50 NFR binding claims. FR-010-AC-11 has a qualified #[ix_trace_rs::trace] marker that trace itself did not return; it is retained as a tooling limitation, not a missing-test finding. FR-042-AC-1 is conditional on the unselected keep-and-harden path and is not an active delegate-path gap. Eleven independently source-confirmed baseline AC trace gaps are listed separately from the PR regression. No run evidence was bound by quoin matrix.

The repository surface inventory covers src modules, their public types/functions, the publication and package-runner CLI paths, crates/method-contract, and tests. Existing behavior maps to FR-001 through FR-005, FR-008 through FR-012, FR-020 through FR-022, FR-030 through FR-034, FR-041/FR-042 and their NFRs. No new unowned production behavior or hollow production stub was found in this diff; the greeting scaffold is not promoted as requirement evidence. The review found a missing required fit-case census and an oracle false rejection, rather than treating passing coverage as semantic proof.

Optional whole-repository semantic review: skipped; no separate opt-in was supplied. The requested detailed hybrid oracle and intent checks ran under code-review. All AC scope entries below are examined for trace reconciliation only unless discussed in findings; no semantic PASS is claimed for untouched areas.

## Examined Scope

```yaml
scope:
- id: FR-033-AC-1
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: For a failing universal model, replay evaluates the domain elements in
    order up to the first failing element, reports that element and the source identity,
    and evaluates no later element; for a passing universal model it evaluates exactly
    as many instances as the domain has elements.
- id: FR-033-AC-2
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: Replay of an existential quantifier stops at and reports its first holding
    instance with its element and source identity, does not evaluate later elements,
    and completes the assertion successfully; when none holds, it reports no holding
    instance and the count enumerated.
- id: FR-033-AC-3
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: A bound whose next instance would exceed the limit in NFR-031 yields `incomplete`
    with reason `replay_limit_exceeded`; that instance is not evaluated and the model
    is never `verified`.
- id: FR-033-AC-4
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: For each of an enum domain, a finite collection and an integer range, and
    for each of universal and existential, one passing model and one failing model
    are replayed to the expected outcome.
- id: FR-033-AC-5
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: A solver model that satisfies the lowered query but fails one instance
    of the original quantifier is not published as verified.
- id: FR-033-AC-6
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: A declared range bound that is missing from the model, repeated, malformed
    or outside its declared type yields `incomplete` with reason `bound_undecodable`;
    a model with lower bound above upper bound replays as an empty range.
- id: FR-033-AC-7
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: A body whose evaluation overflows yields `incomplete` with reason `evaluation_overflow`,
    never a wrapped value.
- id: FR-033-AC-8
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: Where Z3 and cvc5 are installed, each satisfiable quantified fixture in
    TC-030 is decided `sat` by both solvers with agreement, and each returned model
    is published as `verified` only after replay passes.
- id: FR-033-AC-9
  path: spec/functional/FR-033-quantified-assertion-replay.md
  role: examined
  excerpt: When replay evaluates every assertion for a returned model and each holds,
    the model is published as `verified`; if an assertion fails or replay is incomplete,
    it is not.
- id: FR-003-AC-1
  path: spec/functional/FR-003-bounded-adapters.md
  role: examined
  excerpt: In each of three timeout and cancellation repetitions on Linux, zero members
    of the spawned POSIX process group survive 1,000 ms after cleanup begins; the
    measured maximum is retained.
- id: FR-003-AC-2
  path: spec/functional/FR-003-bounded-adapters.md
  role: examined
  excerpt: Malformed, excessive, contradictory, signaled, and nonzero-exit responses
    cannot become conclusive.
- id: FR-003-AC-3
  path: spec/functional/FR-003-bounded-adapters.md
  role: examined
  excerpt: Z3 and cvc5 adapters expose the same normalized result contract.
- id: FR-003-AC-4
  path: spec/functional/FR-003-bounded-adapters.md
  role: examined
  excerpt: The record identifies the configured engine and executable path, observed
    solver version, argv, canonical query identity, limits, exit state, retained stdout/stderr,
    and elapsed/cleanup milliseconds. It contains no expected solver version or executable
    digest.
- id: FR-003-AC-5
  path: spec/functional/FR-003-bounded-adapters.md
  role: examined
  excerpt: Independently configured absolute solver paths work without network or
    shell resolution.
- id: FR-003-AC-6
  path: spec/functional/FR-003-bounded-adapters.md
  role: examined
  excerpt: Dependency and binary inspection finds no linked Z3 or cvc5 library.
- id: FR-008-AC-1
  path: spec/functional/FR-008-bounded-canonical-synthesis.md
  role: examined
  excerpt: Equivalent set-like inputs yield the same canonical candidate order and
    identity.
- id: FR-008-AC-2
  path: spec/functional/FR-008-bounded-canonical-synthesis.md
  role: examined
  excerpt: A candidate without independent validation is not a proof, and validation
    failure cannot become satisfaction.
- id: FR-008-AC-3
  path: spec/functional/FR-008-bounded-canonical-synthesis.md
  role: examined
  excerpt: Exhaustive no-candidate and incomplete bounded search are distinct terminal
    outcomes.
- id: FR-001-AC-1
  path: spec/functional/FR-001-analysis-algebra.md
  role: examined
  excerpt: All five analysis truth conditions, group cardinalities, roles, polarities,
    and assumption sets are explicit and independently executable.
- id: FR-001-AC-2
  path: spec/functional/FR-001-analysis-algebra.md
  role: examined
  excerpt: Default singleton identity and explicit binding groups neither alias same-named
    unequal scopes nor split accepted equal members; incompatible groups reject.
- id: FR-001-AC-3
  path: spec/functional/FR-001-analysis-algebra.md
  role: examined
  excerpt: Request and statement hashes change for every material semantic input,
    binding, analysis-model profile, or encoding-profile identity and ignore irrelevant
    input ordering.
- id: FR-001-AC-4
  path: spec/functional/FR-001-analysis-algebra.md
  role: examined
  excerpt: Invalid or ambiguous requests fail before query generation with stable
    diagnostics.
- id: FR-001-AC-5
  path: spec/functional/FR-001-analysis-algebra.md
  role: examined
  excerpt: Changing an assumption group, requirement revision, clause digest, execution
    point, binding set, semantic limit, analysis-model profile, encoding profile,
    or query bytes changes the applicable identity and cannot reuse stale evidence.
- id: FR-011-AC-1
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: A model returned by Z3 and a model returned by cvc5 for a satisfiable integer
    fixture are each parsed, every named assertion is re-evaluated true, and the published
    witness lists every symbol with its integer value and origins.
- id: FR-011-AC-2
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: For recorded model texts in each solver's output format (including negative
    values, a model that mixes Boolean and integer symbols, and a model that echoes
    named-assertion aliases) the parser yields the expected symbol-to-value assignment
    and skips the aliases.
- id: FR-011-AC-3
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: A model for which one assertion evaluates false is reported incomplete
    with reason `assertion_false` naming that assertion, never as a verified witness.
- id: FR-011-AC-4
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: A model that omits one declared integer symbol is reported incomplete with
    reason `symbol_missing`; a model with an undeclared symbol, a repeated symbol,
    or an integer value for a Boolean symbol (or the reverse) is reported with `symbol_unknown`,
    `symbol_repeated` or `sort_mismatch`.
- id: FR-011-AC-5
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: 'A value one above the declared maximum, one below the declared minimum,
    and a value exactly at each bound: the first two fail with `value_out_of_domain`,
    the bound values verify.'
- id: FR-011-AC-6
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: A decimal, a real-sorted and an exponent-form numeral, and a numeral of
    1,025 characters, each fail with `value_malformed` without panicking; a numeral
    of 1,024 characters, and the value 2^127, each fail with `value_out_of_domain`.
- id: FR-011-AC-7
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: A unit test of the replay evaluator over an unchecked expression (constructed
    directly, not an admitted fixture) `x + 1 > 0` with `x` at the 128-bit signed
    maximum fails with `evaluation_overflow`, not a wrapped result.
- id: FR-011-AC-8
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: In every failing case above the logical status of the solver response (`sat`)
    and the adapter outcome remain available and the witness evidence is marked incomplete.
- id: FR-011-AC-9
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: Replay reads only the typed analysis expressions and the parsed assignment;
    mutating the SMT-LIB query text of a lowered query does not change a replay verdict.
- id: FR-011-AC-10
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: A model with a missing symbol and a malformed numeral reports `value_malformed`;
    a model with an out-of-domain value and a false assertion reports `value_out_of_domain`;
    in the same unchecked-expression unit test, an overflowing assertion before a
    false one in assertion-map order reports `evaluation_overflow`, and the reverse
    order reports `assertion_false`.
- id: FR-011-AC-11
  path: spec/functional/FR-011-integer-witness-replay.md
  role: examined
  excerpt: With real solvers, the satisfiable integer fixture of FR-010-AC-2 is decided
    `sat` by Z3 and by cvc5, each returned model replay-verifies, and the differential
    comparison reports agreement with agreed status `satisfied`.
- id: FR-004-AC-1
  path: spec/functional/FR-004-conclusions.md
  role: examined
  excerpt: All ten sat/unsat-by-analysis-kind classifications match the independent
    truth table.
- id: FR-004-AC-2
  path: spec/functional/FR-004-conclusions.md
  role: examined
  excerpt: Every published Boolean model/counterexample defines the complete symbol
    set and re-evaluates every asserted expression true; incomplete or invalid models
    are never represented as verified.
- id: FR-004-AC-3
  path: spec/functional/FR-004-conclusions.md
  role: examined
  excerpt: Source mapping preserves package, clause, revision, declaration, observation,
    and span identity.
- id: FR-004-AC-4
  path: spec/functional/FR-004-conclusions.md
  role: examined
  excerpt: Missing optional explanation data cannot be represented as present or verified,
    while the exact logical status and adapter outcome remain available.
- id: FR-004-AC-5
  path: spec/functional/FR-004-conclusions.md
  role: examined
  excerpt: Seeded contradictions and dead antecedents are detected without false success
    on timeout.
- id: FR-020-AC-1
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: 'Two role-membership fixtures, each a single assertion `role = Admin` or
    `role = Auditor` (a disjunction of enum equalities) plus one further assertion,
    are decided by both Z3 and cvc5: with the further assertion `role != Admin` the
    query is `sat`, and with the further assertion `role = Guest` the query is `unsat`.'
- id: FR-020-AC-2
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: Every enum variant, record field and type name in a query traces to a declaration
    in the environment, and each unresolved name is refused with the code of the case
    table before solver execution.
- id: FR-020-AC-3
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: A record-field constraint fixture and an option-access fixture (`IsPresent`
    in the antecedent of an implication, `Unwrap` in the consequent) each lower and
    are decided correctly by both solvers.
- id: FR-020-AC-4
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: An `Unwrap` with no presence proof is refused by `check_expression` as
    `InvalidStatement` at statement build and nothing is lowered; an `Unwrap` under
    the proof lowers to the selector.
- id: FR-020-AC-5
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: A record or option nesting a collection or text returns `UnsupportedType`,
    an enum ordering comparison and a mixed-sort comparison are refused at statement
    build as `InvalidStatement` (path `statement.expression`) and, over an unchecked
    expression, by the lowering as `UnsupportedConstruct`, and a self-containing record
    fails at `DeclarationEnvironment::new` with `RecursiveType`. None yields a query.
- id: FR-020-AC-6
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: The capability contract of `quire.smtlib2/v2` lists the constructs above
    as exact and `collection`, `collection_access` and `text` as unsupported, and
    no construct appears in both lists.
- id: FR-020-AC-7
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: Permuting environment type order, variant order, field order and statement
    order yields byte-identical queries and maps, and two enums with identical variant
    names produce distinct sorts.
- id: FR-020-AC-8
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: A membership disjunction with a repeated literal is lowered as written,
    and a `CollectionLiteral` or any other collection expression returns `UnsupportedConstruct`,
    except a collection literal consumed as a quantifier domain per [FR-030](./FR-030-bounded-quantifier-domains.md)
    and [FR-034](./FR-034-quantifier-package-translation.md).
- id: FR-020-AC-9
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: The `(set-logic ...)` of a query declaring no datatype is `QF_LIA` and
    of a query declaring at least one datatype is `QF_UFDTLIA`, in both cases under
    `quire.smtlib2/v2`, that of a query holding a bounded solver quantifier is `LIA`
    without and `UFDTLIA` with a datatype, and a report naming `quire.smtlib2/v1`,
    `QF_UF`, or a logic other than the one the feature set requires is refused by
    validation.
- id: FR-020-AC-10
  path: spec/functional/FR-020-datatype-lowering.md
  role: examined
  excerpt: 'A satisfiable model for a query declaring any datatype (enum, record or
    option), including one whose terms are only datatype literals, is replayed by
    FR-021: it is verified when every assertion holds under the typed evaluation and
    is incomplete, with one of the FR-011 reasons, when it does not; no datatype kind
    is refused as unreplayable.'
- id: FR-031-AC-1
  path: spec/functional/FR-031-enum-and-collection-quantifier-lowering.md
  role: examined
  excerpt: For a universal and an existential fixture over an enum domain, the lowered
    query equals its golden query, and the status derived by the enumeration oracle
    of [FR-033](./FR-033-quantified-assertion-replay.md) equals the fixture's recorded
    expected status. Where Z3 and cvc5 are installed, both also answer the matching
    `sat`/`unsat`.
- id: FR-031-AC-2
  path: spec/functional/FR-031-enum-and-collection-quantifier-lowering.md
  role: examined
  excerpt: The same holds for a universal and an existential fixture over a finite
    collection literal; a `set` with a repeated literal yields the same query as the
    `set` without the repeat, a `sequence` with a repeated literal yields one instance
    per occurrence, and each of `set`, `ordered_set`, `sequence` and `bag` enumerates
    in the order stated above.
- id: FR-031-AC-3
  path: spec/functional/FR-031-enum-and-collection-quantifier-lowering.md
  role: examined
  excerpt: The query for a quantified fixture contains no `forall` or `exists` token
    and one body instance per domain element, in domain order.
- id: FR-031-AC-4
  path: spec/functional/FR-031-enum-and-collection-quantifier-lowering.md
  role: examined
  excerpt: Lowering the same quantified package twice, in separate processes, yields
    byte-identical SMT-LIB2 output.
- id: FR-031-AC-5
  path: spec/functional/FR-031-enum-and-collection-quantifier-lowering.md
  role: examined
  excerpt: The quantifier-map entry for an enum or collection quantifier records its
    source identity and typed elements in instance order. It does not identify a failed
    instance; replay does that under FR-033.
- id: FR-031-AC-6
  path: spec/functional/FR-031-enum-and-collection-quantifier-lowering.md
  role: examined
  excerpt: A body that mentions a free variable whose name equals the bound variable
    of a nested inner quantifier lowers so that the free variable is not captured;
    the golden query shows distinct symbols.
- id: FR-034-AC-1
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: 'Packages holding a universal and an existential quantifier over a collection
    literal translate to the matching bounded quantifiers: one fixture per constructor
    (`set`, `ordered_set`, `sequence`, `bag`), one enum-literal collection and one
    integer-literal collection, each with source spans preserved, and the quantifier
    map of its lowered query holds one entry for each.'
- id: FR-034-AC-2
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: A package holding a quantifier over an unbounded or text domain exits `2`
    naming `semantic_graph.nodes[N]` of the quantifier node, with the reason class
    and span on stderr and no report.
- id: FR-034-AC-3
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: Two quantifiers reusing one variable name in one claim produce distinct,
    non-colliding symbols.
- id: FR-034-AC-4
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: A quantified package run through `run-package` yields the same report bytes,
    except `elapsedMs`, on repeated runs.
- id: FR-034-AC-5
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: For the fixtures of FR-009-AC-21 and FR-012-AC-4, which hold no quantifier
    node, the lowered queries equal their golden queries.
- id: FR-034-AC-6
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: A quantifier whose body is a 10,000-node tree is not refused for size and
    one whose body is 10,001 nodes exits `2`, as FR-009-AC-23 states for other trees;
    an integer-literal collection of 1,024 elements translates, and its 1,024 expanded
    instances do not count toward that node bound.
- id: FR-034-AC-7
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: A collection with a non-literal element, a collection of text, rational,
    record or nested-collection elements, a `Reference` to a collection parameter,
    `quire.op.collection.contains`, `size` or `index` applied to a literal collection,
    and a collection literal outside a quantifier domain each exit `2` naming `semantic_graph.nodes[N]`,
    with no report.
- id: FR-034-AC-8
  path: spec/functional/FR-034-quantifier-package-translation.md
  role: examined
  excerpt: An integer-literal collection of 1,025 elements exits `2` with reason `expansion-ceiling`
    and emits no solver quantifier, and a quantifier with a range-domain operand for
    which no upstream form exists exits `2` naming `semantic_graph.nodes[N]` with
    no report.
- id: FR-041-AC-1
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: The review records a named adversarial set covering flow and reset overflow,
    outward widening, closed and overlapping guards, non-forced source-mode persistence,
    multiple transitions into one mode, invariant handling, horizon coverage and exhaustion,
    malformed inputs, and typed outcomes. Each case records its minimal request and
    the reachable states from independent finite-execution enumeration.
- id: FR-041-AC-2
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: Every case that omits an enumerated reachable state or reports an unreached
    horizon as reached is recorded as an unsound finding with its case name and minimal
    request. A case that adds states without omitting reachable states is recorded
    as a precision finding.
- id: FR-041-AC-3
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: The review lists the callers of reachability and replay operations and
    each requirement or planned consumer that depends on their output.
- id: FR-041-AC-4
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: The recorded disposition is exactly one of keep-and-harden or delegate-and-remove.
    The reason is stated against the number and severity of unsound findings, whether
    each unsound finding has a bounded fix, whether an established external tool supports
    the model class, and the cost of maintaining an algorithm that no external engine
    cross-checks. An unsound finding with a bounded fix and a supported model class
    selects delegate-and-remove with that tool. An unsound finding without a bounded
    fix selects delegate-and-remove with a supporting tool, or with the tool none
    otherwise. When no tool supp
- id: FR-041-AC-5
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: The disposition names no fallback between the two paths. A review that
    records both as alternatives or records a conditional switch from one to the other
    is incomplete.
- id: FR-041-AC-6
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: The disposition states whether the engine-ownership ruling for hybrid models,
    IR-1469, is recorded when the disposition is made. When it is recorded, the disposition
    agrees with it or states why it does not. When it is not recorded, the disposition
    proceeds without waiting for it and the application ticket re-checks agreement
    under [FR-042](./FR-042-hybrid-reachability-disposition.md)-AC-9.
- id: FR-041-AC-7
  path: spec/functional/FR-041-hybrid-reachability-review.md
  role: examined
  excerpt: The findings and disposition are posted on the tracking ticket before the
    disposition is applied.
- id: FR-012-AC-1
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: A package fixture whose `analysis_claims` bound a dose by a maximum and
    the maximum by a weight (`dose <= max` and `max <= 1 * weight` over three integer
    parameters of one `integer_range`, the form the upstream limitation admits) runs
    end to end with controlled solver results and exits `0` with a report that `validate_report_document`
    accepts.
- id: FR-012-AC-2
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: The same fixture with contradictory claims exits `1` with agreed status
    `refuted`.
- id: FR-012-AC-3
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: A claim whose body multiplies two parameters, one that applies `quire.op.integer.div`,
    `.rem` or `.mod`, and one with a reference cycle each exit `2` naming `semantic_graph.nodes[N]`
    of the node where translation stopped, with no report; the library refusal and
    the spawned `run-package` binary both show it.
- id: FR-012-AC-4
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: Each row of the Terms added table translates to the stated expression kind
    and operator, a comparison nests under a Boolean operation, and an integer operand
    of a Boolean operation exits `2` naming the node.
- id: FR-012-AC-5
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: Two claims that reference the same integer parameter node yield one declaration
    with the integer bounds of its type; two distinct parameter nodes yield two.
- id: FR-012-AC-6
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: Two translations of the same package bytes yield equal clause order, identifiers,
    declarations, bounds and spans.
- id: FR-012-AC-7
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: The 10,000/10,001 node bound, the depth 128/129 bound and the shared-subterm
    chain of FR-009-AC-23, rebuilt over integer sums, behave as in FR-009-AC-23.
- id: FR-012-AC-8
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: With real solvers, the fixtures of FR-012-AC-1 and AC-2 yield the same
    exit statuses and agreed statuses as the controlled run.
- id: FR-012-AC-9
  path: spec/functional/FR-012-integer-claim-translation.md
  role: examined
  excerpt: A literal of the bare integer type translates to an `IntegerLiteral` with
    the signed 128-bit bounds and `Reject`; an addition over an `integer_range` of
    more than one point exits `2` naming `claim-N`, as the upstream limitation states.
- id: FR-042-AC-1
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Where the recorded path is keep-and-harden, each case in the review's adversarial
    set yields either an enclosure containing every enumerated reachable state or
    a typed Incomplete or Refused outcome. This branch does not apply when the recorded
    path is delegate-and-remove.
- id: FR-042-AC-2
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove, reach, replay_hybrid, bind_hybrid_result and
    hybrid_method_outcome are absent from the crate, and no code path computes a hybrid
    enclosure inside the crate.
- id: FR-042-AC-3
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove, FR-007, FR-040, NFR-040, TC-013 and TC-040,
    including their index entries and test-matrix rows, are removed.
- id: FR-042-AC-4
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: For each fit case in TC-041 that returns an enclosure, the enclosure contains
    every state reached by the independent oracle for every mode and step.
- id: FR-042-AC-5
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove with a named tool, the tool is started only through
    producer-execute with the time and output limits of NFR-001, and a stub executor
    records exactly one request per valid hybrid call. Malformed request models are
    rejected before producer-execute and cause zero external launches.
- id: FR-042-AC-6
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: Under delegate-and-remove with the configured tool none, every valid hybrid
    request ends Refused with reason `tool_not_configured`, producer-execute request
    count and external invocation count are both zero, and no enclosure is returned.
- id: FR-042-AC-7
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: With a named tool, the crate validates the exact declared result schema,
    requires every requested mode at every requested step to be present exactly once
    or explicitly empty, and rejects unknown or missing members and inverted intervals.
    Timeout maps to Incomplete with reason `tool_timeout`; output-limit breach maps
    to Incomplete with reason `tool_output_limit_exceeded`; non-zero exit maps to
    Incomplete with reason `tool_exit_nonzero`; missing tool maps to Refused with
    reason `tool_unavailable`; schema-invalid output maps to Refused with reason `result_schema_invalid`;
    and an inverted interval
- id: FR-042-AC-8
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: The applied path is the recorded one only. Neither path contains a switch,
    flag or retry that selects the other.
- id: FR-042-AC-9
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: When IR-1469 is recorded, the applied path agrees with it, or the application
    ticket records why it does not. When IR-1469 is not recorded, the application
    ticket records that fact and the path proceeds.
- id: FR-042-AC-10
  path: spec/functional/FR-042-hybrid-reachability-disposition.md
  role: examined
  excerpt: When a named external tool is selected, the application pull request records
    the tool version, the license terms published by its authors, and evidence that
    invoking it as an external executable is permitted.
- id: FR-009-AC-1
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: With a stub `producer-execute` that records every request it receives,
    a `run-package` run issues exactly two requests per engine (version, query), each
    a valid `producer-execution-request/v1` naming the configured absolute solver
    path, an empty environment and the built-in response binding, and no solver process
    is started by any other path.
- id: FR-009-AC-2
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: With a stub `producer-execute` returning `completed` results in which both
    solvers answer `sat` with a model that replays against the assertions of consistent
    clauses, `run-package` exits `0` and writes a report that `validate_report_document`
    accepts with agreed status `satisfied`.
- id: FR-009-AC-3
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: With results in which both solvers answer `unsat` for contradictory clauses,
    `run-package` exits `1` and the report records agreed status `refuted`.
- id: FR-009-AC-4
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: For each of the states `unavailable`, `timed_out`, `failed`, `refused`,
    `malformed_response`, `containment_failure` and `cancelled` returned for one engine,
    and for `sat` against `unsat` across engines, `run-package` exits `3` and the
    report retains both engine records.
- id: FR-009-AC-5
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: A `producer-execute` that cannot be started, that exits with status `2`,
    with status `1`, or with any other non-zero status, that is ended by a signal,
    that prints no result line, two lines, or a line that is not a `producer-execution-result/v1`
    document makes `run-package` exit `4` with the cause on stderr and no report file.
- id: FR-009-AC-6
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: A missing package file, a file `ContractPackageWireV1::read` refuses, a
    package that names a dependency, a package with no `analysis_claim`, a package
    with a non-Boolean `analysis_claim`, and a document one byte over the 1,048,576-byte
    `CheckedPackageReadLimits::bounded()` limit (an `Incomplete` result) each exit
    `2`, write no report and name the cause on stderr. A document exactly at the limit
    is not refused for size.
- id: FR-009-AC-7
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: An unknown, repeated or missing argument and an option without a value
    each exit `2` with the usage on stderr and no report.
- id: FR-009-AC-8
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Each of a relative, an empty, a non-UTF-8 and an over-long path, given
    in turn as `--z3`, `--cvc5` and `--producer-execute`, exits `2` and writes no
    report.
- id: FR-009-AC-9
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: An existing `--output` file, a missing parent directory and an unwritable
    parent directory each exit `4`; an existing destination keeps its bytes.
- id: FR-009-AC-10
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: A unit test of the runner function, injecting a directory-sync failure
    after rename through the publication seam of [TC-008](../test/TC-008-library-cli-atomic.md),
    shows the runner maps the `published-durability-unknown` publication error to
    exit `4` with that state named on stderr.
- id: FR-009-AC-11
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Two runs over the same package bytes, executables and stub results, whose
    stub timings differ, write report files that are byte-identical once every `elapsedMs`
    field is set to `0`, and that differ in no other field.
- id: FR-009-AC-12
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: After a run the package file and its directory hold no added, removed or
    modified entry; the only entries added to the `--output` parent directory are
    the published report, and during the run a capability-root directory of mode `0700`
    that is gone after every exit path (conclusive, non-conclusive, exit `2`, exit
    `4`).
- id: FR-009-AC-13
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: '`run-package --help` lists the five arguments and the exit classes `0`
    to `4`.'
- id: FR-009-AC-17
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: For each row of the result-state table other than `completed`, a stub result
    of that state for one engine yields the stated engine outcome in the published
    report, the stated disposition, and exit `3`. For the `completed` row, the engine
    outcome is the classification the library gives the same bytes run directly and
    the exit status is the one the comparison gives (`0`, `1` or `3`).
- id: FR-009-AC-18
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: A symlinked component in `--z3`, `--cvc5`, `--producer-execute` or the
    `--output` parent each exits `2` and writes no report.
- id: FR-009-AC-19
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: A package whose Boolean `analysis_claim` term the translation cannot lower
    exits `2` naming the node as `semantic_graph.nodes[N]`, with no report.
- id: FR-009-AC-20
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: For a package with analysis claims at nodes `i` and `j`, the statements
    the translation builds have `ClauseRef` (`source.name`, `analysis`, `1`) with
    clause ids `claim-i` and `claim-j` in node order, one `RequirementRef` for both,
    and the execution point `Handler` named `analysis`; `from_clause` accepts both.
    A `source.name` that `PackageId::new` refuses exits `2` naming the package name.
- id: FR-009-AC-21
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Each of `quire.op.boolean.not`, `and`, `or`, `implies`, `eq` and `ne`,
    applied to Boolean parameters, translates to the `ExpressionKind` and operator
    of the Terms table, and a Boolean `Literal` to `BooleanLiteral`; a claim that
    is a `Reference` to an expression node translates to the body of that node.
- id: FR-009-AC-22
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Two claims that reference the same Boolean `value/parameter` node `N` yield
    one `ValueDeclaration` named `nN` of kind `Input` and type Boolean, referenced
    with observation `Current`, and the lowered query holds one variable symbol for
    it; two distinct parameter nodes yield two declarations.
- id: FR-009-AC-23
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Each of a quantifier over a text domain or an unbounded integer domain,
    a `quire.op.integer.div`, `quire.op.integer.rem` or `quire.op.integer.mod` application,
    an integer-typed operand of a Boolean operation (`not`, `and`, `or`, `implies`,
    Boolean `eq` or `ne`), a reference to a node that is neither an expression, a
    literal nor a parameter, a reference cycle, a tree deeper than 128 (128 nested
    `not` applications over a Boolean parameter, depth 129, are refused; 127, depth
    128, are not) and a shared-subterm chain `x1 = and(p, p)`, `xk = and(x(k-1), x(k-1))`
    of 61 nodes (about 2^61 expanded `Exp
- id: FR-009-AC-24
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: 'The `SourceSpan` of a claim is built from `source_map.nodes[N].occurrences[0].regions[0]`
    of its node: line `1`, column `1`, byte offsets the region start and end, document
    `contract-package`, revision `1`. A claim node whose `source_map.nodes[N].occurrences`
    is empty exits `2` naming the node.'
- id: FR-009-AC-25
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: 'By inspection, an error from `check_expression`, `from_clause` or `lower_boolean_statements`
    for a clause maps to exit `2` naming `claim-N`, the path and the message, with
    no report; a Boolean claim does not reach these errors, because the translation
    bounds are tighter, but an integer claim can: the IR refuses, for example, an
    addition over a range of more than one point (FR-012), which exits `2` naming
    `claim-N`.'
- id: FR-009-AC-26
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Two translations of the same package bytes yield equal clause order, ids,
    declarations and spans.
- id: FR-009-AC-28
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: When the capability root cannot be removed (an entry inside it is unreadable),
    the runner names the root path on stderr and exits with the status the run would
    otherwise have had, and the report is still published.
- id: FR-009-AC-27
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Every contract-package document that a test of `run-package` reads is built
    at test time through the public APIs of the `quire-contract-*` dependencies and
    the QSL crates (a `quire.checked-package/v2` document built in test code, read
    with `CheckedPackageV2::read`, lowered with `CheckedPackageV2::lower`, and the
    canonical bytes of the resulting `CompleteContractPackageV2`), or by byte-level
    mutation of such a built document (malformed, oversized, at-limit); no contract-package
    document or copy of another repository's file is committed to this repository.
- id: FR-009-AC-15
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: Every request the runner logs validates against the request schema published
    by `producer-execute --schema request`, and the budget it carries has `maxStdoutBytes`
    and `maxStderrBytes` at most 8,388,608, `maxOutputArtifacts` and `maxOutputBytes`
    `0`, `maxDescendants` `16` and `maxConcurrency` `1`.
- id: FR-009-AC-16
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: The version request of an engine carries `producer.version` `unprobed`
    and stdin `null`; its query request carries the trimmed version stdout of the
    version result; a version result that is not `completed` or is empty yields no
    query request, the engine is recorded with the outcome the result-state table
    gives for that version result (`identity-error` when the text is empty), and the
    exit status is `3`.
- id: FR-009-AC-14
  path: spec/functional/FR-009-package-runner.md
  role: examined
  excerpt: A run with the real `engineering-assurance` executable and real Z3 and
    cvc5 on a contract-package fixture built at test time (FR-009-AC-27) exits `0`
    or `1` and its report validates; the test is skipped with a stated reason when
    either solver or the executable is absent.
- id: FR-005-AC-1
  path: spec/functional/FR-005-evidence-cli.md
  role: examined
  excerpt: CLI publication preserves the authoritative library report byte-for-byte
    and returns its stable exit class.
- id: FR-005-AC-2
  path: spec/functional/FR-005-evidence-cli.md
  role: examined
  excerpt: Every report validates against its versioned schema and PGM-01 evidence
    envelope.
- id: FR-005-AC-3
  path: spec/functional/FR-005-evidence-cli.md
  role: examined
  excerpt: Differential disagreement retains both results and cannot be reported as
    agreement.
- id: FR-005-AC-4
  path: spec/functional/FR-005-evidence-cli.md
  role: examined
  excerpt: Failed publication leaves no partial result or modified developer-owned
    file.
- id: FR-005-AC-5
  path: spec/functional/FR-005-evidence-cli.md
  role: examined
  excerpt: Every filed semantic defect maps to an executable retained regression fixture.
- id: FR-005-AC-6
  path: spec/functional/FR-005-evidence-cli.md
  role: examined
  excerpt: The closed `quire.differential-report/v2` schema requires `producer.name`
    and `producer.version`; every `assertions[].clause`, `assertions[].source`, and
    `assertions[].clauseDigest`; `bindingSetDigest`, `requestDigest`, `queryDigest`,
    and `queryHex`; and every engine's `configuredExecutable`, observed `identity.path`
    and `identity.version` when available, `stdoutHex`, `stderrHex`, and `modelHex`.
    It refuses `identity.bytes`, `identity.sha256`, `limits.executableBytes`, `expectedVersion`,
    `expectedExecutableSha256`, `configurationDigest`, `querySha256`, `stdoutSha256`,
    `stderrSha256`, `modelSha
- id: FR-022-AC-1
  path: spec/functional/FR-022-datatype-package-translation.md
  role: examined
  excerpt: 'Six built contract-package fixtures run end to end through `run-package`,
    with a stub `producer-execute` and with real solvers where installed, each with
    a fixed expected exit: an enum-membership disjunction (`role = Admin` or `role
    = Auditor`) with `role != Admin`; the same disjunction with `role = Guest`, which
    exits `1`; a record-field claim `record.project(user)` with field `role` equal
    to `Admin`; and the same with a second claim that field equal to `Guest`, which
    exits `1`. A fifth fixture is a single claim, `quire.op.boolean.implies` with
    antecedent `quire.op.option.present` of an optio'
- id: FR-022-AC-2
  path: spec/functional/FR-022-datatype-package-translation.md
  role: examined
  excerpt: Every enum variant, record field and type in the lowered query comes from
    the package's `semantic_type` nodes, and a fixture naming an unresolved type exits
    `2` naming the node.
- id: FR-022-AC-3
  path: spec/functional/FR-022-datatype-package-translation.md
  role: examined
  excerpt: 'An unsupported datatype term (collection, except a collection literal
    consumed as a quantifier domain per [FR-030](./FR-030-bounded-quantifier-domains.md)
    and [FR-034](./FR-034-quantifier-package-translation.md); text, rational, enum
    ordering, a record nesting a collection or text) exits `2` naming `semantic_graph.nodes[N]`,
    writes no report and invokes no solver. An unguarded `quire.op.option.value` is
    the one exception: it exits `2` naming `claim-N` and the validator''s path, writes
    no report and invokes no solver.'
- id: FR-022-AC-4
  path: spec/functional/FR-022-datatype-package-translation.md
  role: examined
  excerpt: 'Datatype terms are translated by a pure function of the admitted package:
    the same bytes yield the same clause order, declarations and query.'
- id: FR-022-AC-5
  path: spec/functional/FR-022-datatype-package-translation.md
  role: examined
  excerpt: Datatype expressions count toward the FR-009 node and depth bounds, and
    a shared enum or record chain that exceeds them exits `2` without expanding the
    refused tree.
- id: FR-021-AC-1
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: Enum, record and option values in a model, in the spelling of each solver,
    parse into the same typed values, and every assertion replays to true.
- id: FR-021-AC-2
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: A model value that does not match its declared datatype (unknown or foreign
    constructor, wrong arity, wrong sort, abstract element, malformed text) fails
    replay and the counterexample evidence is incomplete, and the result is not marked
    verified.
- id: FR-021-AC-3
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: 'Replay tests cover each datatype kind: enum, record, option present, option
    absent, and a record that nests an option and an enum.'
- id: FR-021-AC-4
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: A model that satisfies the solver but violates one asserted expression
    under the independent evaluator is rejected as incomplete.
- id: FR-021-AC-5
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: A model that omits a declared datatype symbol, or defines an unknown symbol
    or a symbol twice, is not verified.
- id: FR-021-AC-7
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: An implication, short-circuit `and` or short-circuit `or` whose guard excludes
    the unwrapped operand replays without evaluating its `Unwrap`, and an `Unwrap`
    evaluated on an absent option fails replay and the model is invalid.
- id: FR-021-AC-6
  path: spec/functional/FR-021-datatype-witness-replay.md
  role: examined
  excerpt: A model value nested beyond its declared shape (which lowering bounds at
    the NFR-020 depth) is `value_malformed` without unbounded recursion, and a model
    larger than the model ceiling is the adapter `ModelLimit`; neither is ever conclusive.
- id: FR-002-AC-1
  path: spec/functional/FR-002-smt-lowering.md
  role: examined
  excerpt: Identical models produce byte-identical queries and assertion maps.
- id: FR-002-AC-2
  path: spec/functional/FR-002-smt-lowering.md
  role: examined
  excerpt: Every public IR construct has an exact supported encoding or an explicit
    unsupported classification, with representative fixtures for arithmetic, bounded
    quantification, unbounded quantification (unsupported), and data types.
- id: FR-002-AC-3
  path: spec/functional/FR-002-smt-lowering.md
  role: examined
  excerpt: Query symbols and named assertions map injectively to complete source identities.
- id: FR-002-AC-4
  path: spec/functional/FR-002-smt-lowering.md
  role: examined
  excerpt: No approximation can yield a query or conclusive result.
- id: FR-002-AC-5
  path: spec/functional/FR-002-smt-lowering.md
  role: examined
  excerpt: Golden queries expose declared logic, assertions, identities, and source
    maps for independent review.
- id: FR-032-AC-1
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: For a universal and an existential fixture over a literal range within
    the budget, the lowered query equals its golden query and the status derived by
    the enumeration oracle equals the recorded expected status; where Z3 and cvc5
    are installed, both answer the matching `sat`/`unsat`.
- id: FR-032-AC-2
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: A fixture with declared bounds lowers to the bounded form with the expected
    golden query, and a range with a bound that is not provably finite returns `unsupported`
    with reason `non-finite-bound`.
- id: FR-032-AC-3
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: A literal range whose instance count fits the budget is expanded, and one
    exceeding it is emitted as a bounded quantifier and not refused; the quantifier
    map records each choice.
- id: FR-032-AC-4
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: In the bounded form every quantifier carries a finite interval guard; a
    quantifier without one is never emitted.
- id: FR-032-AC-5
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: Lowering a range-quantified package twice yields byte-identical SMT-LIB2
    output.
- id: FR-032-AC-6
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: A bounded quantifier whose body mentions an enum declares `UFDTLIA`, and
    one whose body is integer-only declares `LIA`.
- id: FR-032-AC-7
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: '`with_formula` refuses an empty quantifier identity, a bound-variable
    reference with no enclosing quantifier, a non-empty literal range outside `local_type`
    and a declared bound outside `local_type`, each as `InvalidStatement`.'
- id: FR-032-AC-8
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: A satisfiable model of a query containing a bounded solver quantifier is
    replayed over the original range as specified by FR-033; it is published as verified
    only when replay succeeds, and the solver's logical response is unchanged.
- id: FR-032-AC-9
  path: spec/functional/FR-032-bounded-integer-range-quantifier-lowering.md
  role: examined
  excerpt: A contested budget gives the same encodings and the same quantifier map
    for any order of the caller's statements, and spends the budget in depth-first
    pre-order within a statement.
- id: FR-030-AC-1
  path: spec/functional/FR-030-bounded-quantifier-domains.md
  role: examined
  excerpt: For a universal and an existential quantifier over each of an enum domain,
    a finite collection and a bounded integer range, the query's quantifier map holds
    one entry with the expected source identity, domain class and encoding, and no
    `unsupported` status is returned.
- id: FR-030-AC-2
  path: spec/functional/FR-030-bounded-quantifier-domains.md
  role: examined
  excerpt: Each of an unbounded integer, a text domain, a rational domain, an unfixed-size
    collection, a collection with an unencodable element type and a range with a non-finite
    bound returns `unsupported` with its matching reason class and the quantifier's
    source identity, and no solver is invoked.
- id: FR-030-AC-3
  path: spec/functional/FR-030-bounded-quantifier-domains.md
  role: examined
  excerpt: An unbounded quantifier nested inside admitted quantifiers refuses the
    whole request; no partial query is emitted.
- id: FR-030-AC-4
  path: spec/functional/FR-030-bounded-quantifier-domains.md
  role: examined
  excerpt: A universal quantifier over an empty domain evaluates to true and an existential
    quantifier over an empty domain to false in the lowered query, in replay and in
    the controlled-solver result.
- id: FR-030-AC-5
  path: spec/functional/FR-030-bounded-quantifier-domains.md
  role: examined
  excerpt: An enum or collection quantifier whose expansion exceeds the budget returns
    `unsupported` with reason `expansion-ceiling`.
- id: FR-030-AC-6
  path: spec/functional/FR-030-bounded-quantifier-domains.md
  role: examined
  excerpt: A domain quantifier has a non-empty source identity and a local type matching
    its enum domain or an encodable collection element sort; each collection member
    fits that sort. A collection with a text, rational, record, option or nested-collection
    element sort is rejected even when empty.
- id: FR-010-AC-1
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: '`CAPABILITY_CONTRACT` declares profile `quire.smtlib2/v2`, logic `QF_LIA`
    for queries without datatypes, and lists each construct of the exact-constructs
    bullet as exact; no construct appears in both the exact and unsupported lists.'
- id: FR-010-AC-2
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: A satisfiable fixture (for example `x >= 1` and `x + y = 10` and `y <=
    4` over declared integers) lowers to a golden query that declares each symbol
    with sort `Int`, asserts each declared range as an unnamed assertion in the fixed
    block, and uses only the exact constructs; the assertion map holds the clause
    assertions only; the golden is byte-identical to production output.
- id: FR-010-AC-3
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: Each of the six comparisons, negation, addition, subtraction, and multiplication
    by a constant factor on either side lowers to the stated SMT-LIB form, and a negative
    literal is emitted as a negation of its magnitude.
- id: FR-010-AC-4
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: A product of two terms that each contain a value reference, a divide, a
    remainder, and a saturating-policy result each return `UnsupportedConstruct` naming
    the class and path; the lowering returns that error, which is the `unsupported`
    outcome, and no query bytes are produced.
- id: FR-010-AC-11
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: A unit test over an unchecked expression (constructed directly, not an
    admitted fixture) whose constant fold exceeds the 128-bit signed range returns
    `UnsupportedConstruct` of class `constant_overflow` and produces no query.
- id: FR-010-AC-5
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: '`2 * 3 * x`, `x * 6` and `(1 + 2) * x` with `x` a value reference each
    emit exactly `(* 6 x)`, `(* 6 x)` and `(* 3 x)`; `1 + (x + 2)` keeps its shape
    and folds no constant across the variable.'
- id: FR-010-AC-6
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: Lowering the same model twice, and lowering it with statement and declaration
    order permuted, yields byte-identical queries and maps.
- id: FR-010-AC-7
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: With real solvers, an unsatisfiable fixture (for example `x > 5` and `x
    < 3`) is decided `unsat` by Z3 and by cvc5 and the differential comparison reports
    agreement with the agreed status `refuted`; the satisfiable fixture is decided
    `sat` by both and each returned integer model replay-verifies ([FR-011](./FR-011-integer-witness-replay.md)),
    so the comparison is an agreement with the agreed status `satisfied`, as [FR-011-AC-11](./FR-011-integer-witness-replay.md)
    states.
- id: FR-010-AC-8
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: With controlled solver responses, `sat` against `unsat` for an integer
    fixture is classified as disagreement, not as a conclusion.
- id: FR-010-AC-9
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: Each Boolean fixture of TC-010 lowers to a query under `quire.smtlib2/v2`
    that decides the same status as before, and no Boolean-only query is emitted under
    `quire.smtlib2/v1`.
- id: FR-010-AC-10
  path: spec/functional/FR-010-linear-integer-arithmetic.md
  role: examined
  excerpt: A declared integer range and literals at the minimum and maximum of the
    128-bit signed range lower to the exact numerals with no wrap or clamp; the report
    for such a run names `quire.smtlib2/v2` and `QF_LIA` (no datatypes), and a report
    naming `quire.smtlib2/v1` or `QF_UF` is refused by validation.
- id: NFR-003-AC-1
  path: spec/nonfunctional/NFR-003-runner-invocation.md
  role: examined
  excerpt: A solver that stalls past its wall-time limit or floods past its output
    limit, run through the real `producer-execute`, yields a published report with
    a typed non-conclusive outcome and exit `3`, never a hang and never exit `0` or
    `1`.
- id: NFR-003-AC-2
  path: spec/nonfunctional/NFR-003-runner-invocation.md
  role: examined
  excerpt: A run with null stdin and an empty environment yields the same exit status
    and the same report bytes, except the `elapsedMs` fields, as a run whose stdin
    is an open pipe holding unread data and whose environment is full; the run with
    open stdin does not wait for it. Network access is not restricted or asserted.
- id: NFR-003-AC-3
  path: spec/nonfunctional/NFR-003-runner-invocation.md
  role: examined
  excerpt: The runner reads no standard input and consults no environment variable
    to select a solver, a limit or an output; inspection of `run-package` finds no
    such read.
- id: NFR-004-AC-1
  path: spec/nonfunctional/NFR-004-integer-exactness.md
  role: examined
  excerpt: Every unsupported integer construct of FR-010 returns `unsupported` and
    none yields a query, a `satisfied` result or a `refuted` result.
- id: NFR-004-AC-2
  path: spec/nonfunctional/NFR-004-integer-exactness.md
  role: examined
  excerpt: Integer literals and bounds at the minimum and maximum of the 128-bit signed
    range lower and replay without panic; a unit test over an unchecked replay expression
    (not an admitted fixture) whose exact result exceeds the 128-bit signed range
    fails replay with `evaluation_overflow`, as FR-011-AC-7 states.
- id: NFR-004-AC-3
  path: spec/nonfunctional/NFR-004-integer-exactness.md
  role: examined
  excerpt: An integer model above the parsed-model ceiling is rejected as non-conclusive
    by the adapter bounds of NFR-001.
- id: NFR-004-AC-4
  path: spec/nonfunctional/NFR-004-integer-exactness.md
  role: examined
  excerpt: Repeated and order-permuted integer analyses preserve query and normalized
    evidence bytes.
- id: NFR-020-AC-1
  path: spec/nonfunctional/NFR-020-datatype-bounds.md
  role: examined
  excerpt: 'Each bound has an exact boundary test: the limit is accepted and the limit
    plus one is refused as an explicit limit breach before solver execution.'
- id: NFR-020-AC-2
  path: spec/nonfunctional/NFR-020-datatype-bounds.md
  role: examined
  excerpt: Repetition and permutation of environment, variant, field and statement
    order preserve query bytes.
- id: NFR-020-AC-3
  path: spec/nonfunctional/NFR-020-datatype-bounds.md
  role: examined
  excerpt: A model nested beyond the depth bound is rejected without unbounded recursion
    or allocation.
- id: NFR-032-AC-1
  path: spec/nonfunctional/NFR-032-quantifier-exhaustion.md
  role: examined
  excerpt: A fault-injected solver that exceeds its wall-time limit on a bounded-quantifier
    query yields `timeout`, never `satisfied` or `refuted`.
- id: NFR-032-AC-2
  path: spec/nonfunctional/NFR-032-quantifier-exhaustion.md
  role: examined
  excerpt: A fault-injected solver that answers `unknown` on a bounded-quantifier
    query yields `unknown`, never `satisfied` or `refuted`.
- id: NFR-032-AC-3
  path: spec/nonfunctional/NFR-032-quantifier-exhaustion.md
  role: examined
  excerpt: Where Z3 and cvc5 are installed, a bounded-quantifier fixture that a solver
    cannot decide within its limits is reported `timeout` or `unknown` for that solver;
    where they are absent the case is reported as not run.
- id: NFR-030-AC-1
  path: spec/nonfunctional/NFR-030-quantifier-expansion-bound.md
  role: examined
  excerpt: A request whose expanded instance counts sum to 1,024 lowers, and one summing
    to 1,025 refuses its enum or collection expansion as `unsupported` with reason
    `expansion-ceiling` and emits a literal range as a bounded quantifier.
- id: NFR-030-AC-2
  path: spec/nonfunctional/NFR-030-quantifier-expansion-bound.md
  role: examined
  excerpt: The nested fixture of the example (outer literal range of width 100, inner
    literal range of width 100) expands the outer range, emits the inner range as
    a bounded quantifier, and records 100 instances in the quantifier map.
- id: NFR-030-AC-3
  path: spec/nonfunctional/NFR-030-quantifier-expansion-bound.md
  role: examined
  excerpt: A literal range of width 2^64 and nested ranges whose instance-count product
    exceeds 2^128 are treated as exceeding the budget; the computation does not wrap
    or panic.
- id: NFR-030-AC-4
  path: spec/nonfunctional/NFR-030-quantifier-expansion-bound.md
  role: examined
  excerpt: Quantified lowering allocates in proportion to the checked instance count
    and never from an unchecked bound.
- id: NFR-002-AC-1
  path: spec/nonfunctional/NFR-002-evidence-authority.md
  role: examined
  excerpt: Every report retains its schema and encoding, canonical clause, binding,
    request and query identities, configured engine and executable path, observed
    solver version when available, raw output and normalized result. Bound validation
    rejects any difference from the authoritative query or run.
- id: NFR-002-AC-2
  path: spec/nonfunctional/NFR-002-evidence-authority.md
  role: examined
  excerpt: Unknown, unsupported, timeout, cancellation, absence, malformed output,
    and tool failure remain distinct.
- id: NFR-002-AC-3
  path: spec/nonfunctional/NFR-002-evidence-authority.md
  role: examined
  excerpt: Standalone validation rejects a changed query with unchanged canonical
    query identity, omitted required artifacts, and contradictions between retained
    output and declared outcome. Validation bound to an authoritative run rejects
    raw-output substitution even when the substitute and its declared outcome are
    internally consistent.
- id: NFR-002-AC-4
  path: spec/nonfunctional/NFR-002-evidence-authority.md
  role: examined
  excerpt: Human release and consuming-project qualification claims remain open.
- id: NFR-031-AC-1
  path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md
  role: examined
  excerpt: A request needing exactly 1,048,576 instances replays fully; one needing
    1,048,577 yields `incomplete` with reason `replay_limit_exceeded`, has evaluated
    no more than 1,048,576 instances, and is never `verified`.
- id: NFR-031-AC-2
  path: spec/nonfunctional/NFR-031-quantifier-replay-bound.md
  role: examined
  excerpt: A 2^64-wide range and nested ranges whose instance-count product overflows
    yield `incomplete` with reason `replay_limit_exceeded`, without wrapping or panicking.
- id: NFR-001-AC-1
  path: spec/nonfunctional/NFR-001-determinism-bounds.md
  role: examined
  excerpt: Repetition and irrelevant-order permutation preserve query and normalized
    evidence bytes.
- id: NFR-001-AC-2
  path: spec/nonfunctional/NFR-001-determinism-bounds.md
  role: examined
  excerpt: Every adapter process, time, input, output, model, and path boundary has
    a numeric limit and exact boundary test; lowering recursion remains owned by TC-010.
- id: NFR-001-AC-3
  path: spec/nonfunctional/NFR-001-determinism-bounds.md
  role: examined
  excerpt: A breached limit is explicit and never conclusive.
- id: TC-041-step-1
  path: spec/test/TC-041-delegated-hybrid-reachability.md
  role: examined
  excerpt: At each later step, it adds every integer produced by every allowed flow
    delta, keeps the flowed state in its source mode under non-forced transition semantics,
    and adds the reset image of each flowed state that satisfies a closed guard.
- id: TC-041-step-2
  path: spec/test/TC-041-delegated-hybrid-reachability.md
  role: examined
  excerpt: Valid passing cases include a guard touched at one point, a partial-guard
    reset, multiple transitions joined into one target, duplicate and reordered transitions,
    exact bounded-state enumeration, valid error-bound widening, an unreachable target,
    and bounded small models.
bindings: []
```


## Disposition pass 1

Reviewed fix 5bab70078d4e589762158fd6b93bf4c193adef22. Before appending these dispositions, verified the committed reviews/ copy was byte-identical to the original scratchpad artifact. Original findings and scope are preserved unchanged. No newly numbered finding was added; remaining failures are unresolved parts of the existing findings.

VER-95-specific finding fixed. Eleven unrelated baseline gaps explicitly accepted without change for this PR; this is not a repository-wide traceability PASS.

Focused checks only: cargo test --lib hybrid_tool::tests -- --include-ignored --test-threads=1 passed 11/11 (37.44 s), including both unchanged F10 timeout cases, real fit sweep, malformed-output checks, ordinary supervisor cleanup, and empty-target regression. cargo test --test package_runner passed 32 tests with 2 existing ignored tests (25.67 s), covering the caller moved to the shared seam. Every cargo/rustc command used the assigned locked-build.sh wrapper, target directory, RUSTUP_TOOLCHAIN=1.98.1 and CARGO_PROFILE_DEV_DEBUG=0. No aggregate gate ran. Public API reproductions are in /tmp/codex-theories/ver95-disposition1/public-reproductions.log; test and probe-build logs are in the same directory. The first nested-group harness used captured stderr, which the surviving descendant held open; the repeated file-backed run measured the adapter return directly. Reproduction descendants were explicitly killed by the reviewer.

NFR-001 and the literal -v000000 PHAVer request argument are unchanged. The shared seam still invokes only producer-execute. Platform and header fixes were inspected directly. Baseline FND-002 through FND-012 in SR-17080 were checked against unchanged source and the explicit review scope.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | Named same-target join and duplicate/reordered transition cases are present in the required census and were executed. Same-target +0/+100 contributions use the exact interval-union oracle; the reordered duplicate cases additionally compare outcomes. The real PHAVer fit test passed. |
| FND-002 | accepted-no-change | FR-009-AC-10 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-003 | accepted-no-change | FR-033-AC-5 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-004 | accepted-no-change | FR-033-AC-8 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-005 | accepted-no-change | FR-034-AC-1 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-006 | accepted-no-change | FR-034-AC-2 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-007 | accepted-no-change | FR-034-AC-3 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-008 | accepted-no-change | FR-034-AC-4 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-009 | accepted-no-change | FR-034-AC-5 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-010 | accepted-no-change | FR-034-AC-6 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-011 | accepted-no-change | FR-034-AC-7 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |
| FND-012 | accepted-no-change | FR-034-AC-8 remains a pre-existing baseline trace gap. Source inspection on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier source and requirement files are outside this diff. Per the code lead's explicit disposition brief, retain this real gap without expanding PR #104 into unrelated work. Acceptance is limited to VER-95 merge scope; it does not claim that the baseline gap is fixed or that repository-wide trace coverage passes. |

### Disposition evidence

```yaml
dispositions:
- fnd: FND-001
  outcome: fixed
  after_excerpt: "            fit_case(\n                \"AC5_same_target_join\"\
    ,\n                \"m\",\n                (0, 5),\n                (0, 0),\n\
    \                1,\n                0,\n                &[(\"m\", \"t\", 0, 20,\
    \ 0), (\"m\", \"t\", 0, 20, 100)],\n            ),\n"
- fnd: FND-002
  outcome: accepted-no-change
  reason: 'FR-009-AC-10 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-003
  outcome: accepted-no-change
  reason: 'FR-033-AC-5 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-004
  outcome: accepted-no-change
  reason: 'FR-033-AC-8 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-005
  outcome: accepted-no-change
  reason: 'FR-034-AC-1 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-006
  outcome: accepted-no-change
  reason: 'FR-034-AC-2 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-007
  outcome: accepted-no-change
  reason: 'FR-034-AC-3 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-008
  outcome: accepted-no-change
  reason: 'FR-034-AC-4 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-009
  outcome: accepted-no-change
  reason: 'FR-034-AC-5 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-010
  outcome: accepted-no-change
  reason: 'FR-034-AC-6 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-011
  outcome: accepted-no-change
  reason: 'FR-034-AC-7 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
- fnd: FND-012
  outcome: accepted-no-change
  reason: 'FR-034-AC-8 remains a pre-existing baseline trace gap. Source inspection
    on 5bab700 confirms it is unchanged by the VER-95 fix; the relevant report/quantifier
    source and requirement files are outside this diff. Per the code lead''s explicit
    disposition brief, retain this real gap without expanding PR #104 into unrelated
    work. Acceptance is limited to VER-95 merge scope; it does not claim that the
    baseline gap is fixed or that repository-wide trace coverage passes.'
```
