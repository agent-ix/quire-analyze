---
id: FR-007
title: "Provide sound hybrid reachability enclosures"
type: FR
relationships:
  - target: "ix://agent-ix/quire-analyze/StR-001"
    type: implements
---
# FR-007: Provide sound hybrid reachability enclosures

## Description

When a caller submits a bounded hybrid model, the analysis provider SHALL retain
the exact mode, guard, reset, initial set, horizon, and declared numerical error
bound in its result identity and SHALL emit only a sound enclosure or a typed
non-conclusion.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-007-AC-1 | A successful result contains an enclosure and its declared error bound, not a point estimate. | Test (TC-013) |
| FR-007-AC-2 | Failed convergence, an exceeded bound, or malformed mode/guard/reset data returns incomplete, failed, or refused and never proved. | Test (TC-013) |
| FR-007-AC-3 | A retained witness replays against the exact model and bound. | Test (TC-013) |
