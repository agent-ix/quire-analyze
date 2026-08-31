---
id: TC-004
title: "Verify injective symbols and source mapping"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-002
    type: verifies
  - target: ix://agent-ix/quire-analyze/FR-004
    type: verifies
---
# TC-004: Verify injective symbols and source mapping

## Description

Verify generated identities cannot collide and source/model maps round-trip exactly.

## Test Procedure

Construct collision families for punctuation, Unicode, prefixes, package IDs, revisions,
declarations, observations, and spans. Round-trip every assertion and model symbol through the map.

## Expected Results

No unequal complete identity collides. Every decoded value and assertion resolves to its exact
package, clause, revision, declaration, observation, execution point, and source span.
