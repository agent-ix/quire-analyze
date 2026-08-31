---
id: FR-001
title: "Define analysis algebra, identity, and shared-variable semantics"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/StR-001
    type: satisfies
  - target: ix://agent-ix/quire-analyze/interface-001
    type: implements
---
# FR-001: Define analysis algebra, identity, and shared-variable semantics

## Description

The analyzer shall define closed consistency, contradiction, implication, redundancy, and
dead-antecedent requests over validated Boolean contract clauses without reinterpreting contract-IR
types, observations, definedness, or identity.

## Inputs

A pinned validated package, ordered assumption/left/right/candidate clause groups as required by the
analysis kind, execution point, bounds, and encoding profile.

## Outputs

A validated analysis model or an ordered diagnostic set, plus canonical request and statement
digests.

## Behavior

- The closed analysis kinds and decision predicates are:

| Kind | Decision predicate | `satisfied` | `refuted` |
|---|---|---|---|
| consistency | conjunction of selected statements | `sat` with shared model | `unsat` |
| contradiction | conjunction of left and right groups under assumptions | `unsat` | `sat` with common model |
| implication | assumptions and antecedents and negated consequent | `unsat` | `sat` with counterexample |
| redundancy | assumptions and peer statements and negated candidate | `unsat` | `sat` with distinguishing model |
| dead antecedent | assumptions and selected case antecedent | `unsat` | `sat` with activation model |

- A request declares every assumption explicitly; no ambient requirement is inferred.
- Variables are shared only when package, declaration, observation, execution-point, and type
  identities agree; display names alone never alias variables.
- The model preserves contract-IR short-circuit, total Boolean, definedness, bounded arithmetic,
  option, record, collection, and observation semantics.
- Canonical request and statement identities bind the source package digest, clause revisions,
  normalized selection order, analysis kind, bounds, and encoding version.
- Ill-typed, stale, ambiguous, empty, or non-Boolean selections are rejected before SMT lowering.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-001-AC-1 | All five analysis truth conditions and assumption sets are explicit and independently executable. | Test (TC-001) |
| FR-001-AC-2 | Shared-variable identity neither aliases unequal identities nor splits equal identities. | Test (TC-001) |
| FR-001-AC-3 | Request and statement hashes change for every material semantic input and ignore irrelevant input ordering. | Test (TC-002) |
| FR-001-AC-4 | Invalid or ambiguous requests fail before query generation with stable diagnostics. | Test (TC-003) |
| FR-001-AC-5 | Changing an assumption group changes request identity and cannot reuse stale evidence. | Test (TC-002) |

## Dependencies

Accepted contract-IR FR-013 through FR-017 and ADR-0010 research in native issue #6.
