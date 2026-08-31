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
- Each named assertion maps to one or more source clause/span identities.
- Integer and rational bounds, overflow, division, remainder, definedness, options, records,
  collections, finite quantifiers, and state observations follow the accepted IR semantics exactly.
- A profile declares supported constructs, solver logic, encoding version, and resource limits.
- Unsupported or approximate semantics return `unsupported`; v0.1 never emits an approximate query
  that can produce `satisfied` or `refuted`.

## Acceptance Criteria

| ID | Criteria | Verification |
|---|---|---|
| FR-002-AC-1 | Identical models produce byte-identical queries and assertion maps. | Test (TC-002) |
| FR-002-AC-2 | Every public IR construct has an exact supported encoding test or an explicit unsupported fixture. | Test (TC-003) |
| FR-002-AC-3 | Query symbols and named assertions map injectively to complete source identities. | Test (TC-004) |
| FR-002-AC-4 | No approximation can yield a conclusive result. | Test (TC-003) |

## Dependencies

FR-001 and native issue #7.
