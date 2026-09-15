---
id: FR-008
title: "Provide bounded canonical synthesis candidates"
type: FR
relationships:
  - target: "ix://agent-ix/quire-analyze/StR-001"
    type: implements
---
# FR-008: Provide bounded canonical synthesis candidates

## Description

When a caller submits a finite canonical synthesis search, the provider SHALL
enumerate candidates in canonical order within the declared bound and SHALL keep
candidate discovery distinct from independent validation.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-008-AC-1 | Equivalent set-like inputs yield the same canonical candidate order and identity. | Test (TC-014) |
| FR-008-AC-2 | A candidate without independent validation is not a proof, and validation failure cannot become satisfaction. | Test (TC-014) |
| FR-008-AC-3 | Exhaustive no-candidate and incomplete bounded search are distinct terminal outcomes. | Test (TC-014) |
