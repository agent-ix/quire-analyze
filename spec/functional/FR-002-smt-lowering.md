---
id: FR-002
title: "Lower deterministic SMT-LIB2 with capability contracts"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-001
    type: depends_on
  - target: ix://agent-ix/quire-analyze/interface-001
    type: implements
---
# FR-002: Lower deterministic SMT-LIB2 with capability contracts

## Description

The analyzer shall lower a validated analysis model into canonical SMT-LIB2 only when every selected
construct has a declared exact encoding under the selected profile.

## Behavior

- Declaration, assertion, and metadata order is stable and independent of host maps and paths.
- Symbols use injective encoding of complete semantic identities.
- Each named assertion maps to its complete package, requirement, revision, clause digest, and source
  span identity; each variable map retains all complete declaration origins and any explicit binding.
- `quire.smtlib2/v1` is deliberately Boolean-only: literals, typed Boolean input/state references,
  negation, short-circuit and total conjunction/disjunction, implication, and Boolean equality or
  inequality lower exactly to SMT-LIB 2.6 `QF_UF`.
- Integer, rational, text, enum, record, option, collection, field/index/option access, calls,
  arithmetic, ordering, locals, and quantifiers are explicitly unsupported by this profile. The
  exhaustive Rust match makes an upstream expression variant addition a compile-time review point.
- State observations and execution points remain part of variable identity. Short-circuit and total
  Boolean operators share an encoding only after upstream validation establishes total Boolean
  operands and discharges definedness obligations.
- The profile declares supported and unsupported constructs, logic and SMT-LIB version, exact
  dependency revision, statement/expression/query byte limits, and named-assertion/model features.
- Domain-separated statement, binding-set, analysis-request, and query digests bind sorted canonical
  identities, public limits, the encoding profile, and exact query bytes.
- Unsupported or approximate semantics return `unsupported`; v0.1 never emits an approximate query
  that can produce `satisfied` or `refuted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-002-AC-1 | Identical models produce byte-identical queries and assertion maps. | Test (TC-002, TC-010) |
| FR-002-AC-2 | Every public IR construct has an exact supported encoding or an explicit unsupported classification, with representative fixtures for arithmetic, quantification, and data types. | Test (TC-003, TC-010) |
| FR-002-AC-3 | Query symbols and named assertions map injectively to complete source identities. | Test (TC-004, TC-010) |
| FR-002-AC-4 | No approximation can yield a query or conclusive result. | Test (TC-003, TC-010) |
| FR-002-AC-5 | Golden queries expose declared logic, assertions, identities, and source maps for independent review. | Test (TC-004, TC-010) |

## Dependencies

FR-001 and native issue #7.
