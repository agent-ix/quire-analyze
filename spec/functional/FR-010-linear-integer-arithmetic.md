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
to deterministic SMT-LIB2 and shall have the external Z3 and cvc5 solvers decide them through the
existing bounded adapters. The analyzer remains a runner that is fed by QSL contract packages: it
implements no solver and no decision procedure of its own. Terms outside linear integer arithmetic
return `unsupported`; they are never approximated.

## Behavior

- **Profile.** The capability contract of `src/smt.rs` ([FR-002](./FR-002-smt-lowering.md)) gains an
  integer profile, `quire.smtlib2/v2`, with SMT-LIB version 2.6 and logic `QF_LIA`. It replaces the
  Boolean-only `quire.smtlib2/v1`; every Boolean construct of FR-002 keeps its exact encoding under
  it. No query is emitted under two profiles, and no fallback to the Boolean-only profile exists.
- **Exact constructs added.** The capability contract lists these as exact: integer literal, integer
  value reference, integer negation, integer addition, integer subtraction, multiplication of a term
  by an integer constant, and the six comparisons (equal, not equal, less, less or equal, greater,
  greater or equal) between integer terms. Each lowers to the SMT-LIB `Int` sort, `+`, `-`, `*` with a
  literal factor, `=`, `distinct`, `<`, `<=`, `>`, `>=`.
- **Constant factor.** A multiplication is linear when at least one operand is a constant term: an
  integer literal, or an arithmetic combination of constant terms. A negative literal is written in
  SMT-LIB2 as the negation of its magnitude, so no solver sees a signed numeral.
- **Declared domain.** Each integer value reference carries its declared integer type (minimum and
  maximum). The query asserts that range for every integer symbol as a named assertion that appears in
  the assertion map with the symbol's declaration origin, so the solver cannot choose a value outside
  the declared domain, and a reviewer can see the bound.
- **Exact semantics only.** Arithmetic lowers over the mathematical integers. An operation whose result
  type has the saturating overflow policy, integer division, remainder or modulo, a product of two
  non-constant terms, exponentiation, and any integer term the lowering cannot place in the supported
  list above are unsupported. A literal or declared bound outside the signed 64-bit range of the IR is
  refused before lowering by the IR itself and is never wrapped or clamped. Rational and other
  non-integer arithmetic stay unsupported as FR-002 states.
- **Non-linear result.** An unsupported integer construct yields the stable `UnsupportedConstruct`
  diagnostic of FR-002, naming the construct class (for a product of non-constant terms, the class
  `non_linear_arithmetic`) and the path of the offending expression. The analysis status is
  `unsupported`. No query is produced, no solver is started, and no `satisfied` or `refuted` result
  is derivable from the statement.
- **Determinism.** The same model yields byte-identical integer queries and assertion/variable maps
  under the rules of [NFR-001](../nonfunctional/NFR-001-determinism-bounds.md): declaration order,
  assertion order, numeral spelling, and operand order are fixed and independent of host maps.
- **Both solvers.** The adapters of FR-003 submit the same query bytes to Z3 and cvc5. Both answers
  are normalized to `sat`, `unsat` or a non-conclusive state; the differential comparison of
  [FR-005](./FR-005-evidence-cli.md) classifies agreement. A disagreement between conclusive answers is
  a `disagreement`, never resolved by choosing an engine.
- **Verification where solvers exist.** Acceptance criteria that need a real solver run where Z3 and
  cvc5 are installed and are reported as skipped, not passed, where they are absent; criteria that
  use controlled solver responses run everywhere (see TC-016).

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-010-AC-1 | `CAPABILITY_CONTRACT` declares profile `quire.smtlib2/v2`, logic `QF_LIA`, and lists each construct of the exact-constructs bullet as exact; no construct appears in both the exact and unsupported lists. | Test (TC-016) |
| FR-010-AC-2 | A satisfiable fixture (for example `x >= 1` and `x + y = 10` and `y <= 4` over declared integers) lowers to a golden query that declares each symbol with sort `Int`, asserts each declared range as a named assertion, and uses only the exact constructs; the golden is byte-identical to production output. | Test (TC-016) |
| FR-010-AC-3 | Each of the six comparisons, negation, addition, subtraction, and multiplication by a literal factor on either side lowers to the stated SMT-LIB form, and a negative literal is emitted as a negation of its magnitude. | Test (TC-016) |
| FR-010-AC-4 | A product of two terms that each contain a value reference, integer division, remainder, a saturating-policy result, and exponentiation each return `UnsupportedConstruct` naming the class and path; the status is `unsupported`; no query bytes are produced. | Test (TC-016) |
| FR-010-AC-5 | A product of a term and a constant term built from several literals (for example `2 * 3 * x`) lowers as linear. | Test (TC-016) |
| FR-010-AC-6 | Lowering the same model twice, and lowering it with statement and declaration order permuted, yields byte-identical queries and maps. | Test (TC-016) |
| FR-010-AC-7 | With real solvers, a satisfiable fixture is decided `sat` and an unsatisfiable fixture (for example `x > 5` and `x < 3`) is decided `unsat` by Z3 and by cvc5, and the differential comparison reports agreement with the expected agreed status. | Test (TC-016, real solvers) |
| FR-010-AC-8 | With controlled solver responses, `sat` against `unsat` for an integer fixture is classified as disagreement, not as a conclusion. | Test (TC-016) |
| FR-010-AC-9 | With the profile selected, no Boolean-only fixture of TC-010 changes its meaning: each still lowers to a query that decides the same status. | Test (TC-010, TC-016) |

## Dependencies

FR-002, FR-003, FR-005 and [FR-004](./FR-004-conclusions.md).
