---
id: FR-010
title: "Lower and decide linear integer arithmetic on both solvers"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: depends_on
  - target: ix://agent-ix/quire-analyze/FR-003
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
  - target: ix://agent-ix/quire-analyze/interface_001
    type: implements
---
# FR-010: Lower and decide linear integer arithmetic on both solvers

## Description

The analyzer shall lower integer-typed analysis statements that stay within linear integer arithmetic
to deterministic SMT-LIB2.

The analyzer shall have the external Z3 and cvc5 solvers decide those queries through the existing
bounded adapters.

The analyzer shall return `unsupported` for a term outside linear integer arithmetic, with no
approximation. The analyzer remains a runner that is fed by QSL contract packages and implements no
solver and no decision procedure of its own.

## Behavior

- **Profile.** The capability contract of `src/smt.rs` ([FR-002](./FR-002-smt-lowering.md)) gains an
  integer profile, `quire.smtlib2/v2`, with SMT-LIB version 2.6. Its logic is determined by the feature set of the query: `QF_LIA` for a query
  without datatype declarations, including a Boolean-only query; a query that declares datatypes declares
  the logic FR-020 specifies; a query that holds a bounded solver quantifier ([FR-032](./FR-032-bounded-integer-range-quantifier-lowering.md)) declares `LIA` without datatypes and `UFDTLIA` with them. It replaces the
  Boolean-only `quire.smtlib2/v1`; every Boolean construct of FR-002 keeps its exact encoding under
  it, and a Boolean-only query declares `QF_LIA`. No query is emitted under two profiles
  and no fallback to the Boolean-only profile exists. Reports name `quire.smtlib2/v2` and the logic of the query
  ([FR-005](./FR-005-evidence-cli.md)).
- **Exact constructs added.** The capability contract lists these as exact: integer literal, integer
  value reference, integer negation, integer addition, integer subtraction, multiplication of a term
  by a constant term, and the six comparisons (equal, not equal, less, less or equal, greater,
  greater or equal) between integer terms. The analysis model's numeric operators are add, subtract,
  multiply, divide and remainder; the first three and negation are exact. Each lowers to the SMT-LIB
  `Int` sort, `+`, `-`, `*` with a literal factor, `=`, `distinct`, `<`, `<=`, `>`, `>=`.
- **Constant terms and folding.** A constant term is an integer literal, or an arithmetic combination
  (negation, addition, subtraction, multiplication) of constant terms; it contains no value
  reference. The lowering folds every maximal constant subtree to one literal with exact arithmetic
  before emission and does not reassociate other terms. A multiplication is linear when at least one
  operand is a constant term; it is emitted with the folded constant first, so `2 * 3 * x` and
  `x * 6` both emit `(* 6 x)`. A negative literal is emitted as the negation of its magnitude, for
  example `(- 6)`, so no solver sees a signed numeral. A fold whose result lies outside the 128-bit
  signed range is unsupported with class `constant_overflow`. This is a defensive reason: IR validation
  proves admitted constant subtrees inside the range, so no admitted fixture reaches it.
- **Declared domain.** Each integer value reference carries its declared integer type (minimum and
  maximum). The query asserts that range for every integer symbol as an unnamed assertion, in a
  fixed block after the declarations in ascending symbol order. Range assertions are not named
  assertions: they do not appear in the assertion map or in the report `assertions[]` (FR-005), which
  keep their clause, clause-digest, role and polarity fields for clause statements only. The bounds are
  part of the query bytes and so of the query identity, and the replay of FR-011 checks each model
  value against the declared range directly.
- **Exact semantics only.** Arithmetic lowers over the mathematical integers. The lowering does not
  prove that a result stays inside its declared result type; that is a definedness obligation
  discharged by upstream IR validation (the reject overflow policy), on which the exactness of the
  encoding depends. An operation whose result type has the saturating overflow policy, integer
  division, remainder, a product of two non-constant terms, and any integer term outside the
  list above are unsupported. Literals and declared bounds are 128-bit signed integers in the IR; the
  lowering emits every value of that range exactly and never wraps or clamps. Rational and
  non-integer arithmetic stay unsupported as FR-002 states.
- **Non-linear result.** An unsupported integer construct yields the stable `UnsupportedConstruct`
  diagnostic of FR-002, naming the construct class (`non_linear_arithmetic` for a product of
  non-constant terms, `integer_division`, `integer_remainder`, `saturating_arithmetic`,
  `constant_overflow`) and the path of the offending expression. The analysis status is `unsupported`.
  No query is produced, no solver is started, and no `satisfied` or `refuted` result is derivable.
- **Determinism.** The same model yields byte-identical integer queries and assertion/variable maps
  under the rules of [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md): declaration order,
  assertion order, numeral spelling, and operand order are fixed and independent of host maps.
- **Both solvers.** The adapters of FR-003 submit the same query bytes to Z3 and cvc5. Both answers
  are normalized to `sat`, `unsat` or a non-conclusive state; the differential comparison of
  FR-005 classifies agreement. A disagreement between conclusive answers is a `disagreement`, never
  resolved by choosing an engine.
- **Verification where solvers exist.** Acceptance criteria that need a real solver run where Z3 and
  cvc5 are installed and are reported as skipped, not passed, where they are absent; criteria that
  use controlled solver responses run everywhere (see TC-016).

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-010-AC-1 | `CAPABILITY_CONTRACT` declares profile `quire.smtlib2/v2`, logic `QF_LIA` for queries without datatypes, and lists each construct of the exact-constructs bullet as exact; no construct appears in both the exact and unsupported lists. | Test (TC-016) |
| FR-010-AC-2 | A satisfiable fixture (for example `x >= 1` and `x + y = 10` and `y <= 4` over declared integers) lowers to a golden query that declares each symbol with sort `Int`, asserts each declared range as an unnamed assertion in the fixed block, and uses only the exact constructs; the assertion map holds the clause assertions only; the golden is byte-identical to production output. | Test (TC-016) |
| FR-010-AC-3 | Each of the six comparisons, negation, addition, subtraction, and multiplication by a constant factor on either side lowers to the stated SMT-LIB form, and a negative literal is emitted as a negation of its magnitude. | Test (TC-016) |
| FR-010-AC-4 | A product of two terms that each contain a value reference, a divide, a remainder, and a saturating-policy result each return `UnsupportedConstruct` naming the class and path; the lowering returns that error, which is the `unsupported` outcome, and no query bytes are produced. | Test (TC-016) |
| FR-010-AC-11 | A unit test over an unchecked expression (constructed directly, not an admitted fixture) whose constant fold exceeds the 128-bit signed range returns `UnsupportedConstruct` of class `constant_overflow` and produces no query. | Test (TC-016) |
| FR-010-AC-5 | `2 * 3 * x`, `x * 6` and `(1 + 2) * x` with `x` a value reference each emit exactly `(* 6 x)`, `(* 6 x)` and `(* 3 x)`; `1 + (x + 2)` keeps its shape and folds no constant across the variable. | Test (TC-016) |
| FR-010-AC-6 | Lowering the same model twice, and lowering it with statement and declaration order permuted, yields byte-identical queries and maps. | Test (TC-016) |
| FR-010-AC-7 | With real solvers, an unsatisfiable fixture (for example `x > 5` and `x < 3`) is decided `unsat` by Z3 and by cvc5 and the differential comparison reports agreement with the agreed status `refuted`; the satisfiable fixture is decided `sat` by both and each returned integer model replay-verifies ([FR-011](./FR-011-integer-witness-replay.md)), so the comparison is an agreement with the agreed status `satisfied`, as [FR-011-AC-11](./FR-011-integer-witness-replay.md) states. | Test (TC-016, real solvers) |
| FR-010-AC-8 | With controlled solver responses, `sat` against `unsat` for an integer fixture is classified as disagreement, not as a conclusion. | Test (TC-016) |
| FR-010-AC-9 | Each Boolean fixture of TC-010 lowers to a query under `quire.smtlib2/v2` that decides the same status as before, and no Boolean-only query is emitted under `quire.smtlib2/v1`. | Test (TC-010, TC-016) |
| FR-010-AC-10 | A declared integer range and literals at the minimum and maximum of the 128-bit signed range lower to the exact numerals with no wrap or clamp; the report for such a run names `quire.smtlib2/v2` and `QF_LIA` (no datatypes), and a report naming `quire.smtlib2/v1` or `QF_UF` is refused by validation. | Test (TC-016, TC-007) |

## Dependencies

FR-002, FR-003, FR-005, [FR-004](./FR-004-conclusions.md) and, for agreement on a satisfiable integer fixture, [FR-011](./FR-011-integer-witness-replay.md).
