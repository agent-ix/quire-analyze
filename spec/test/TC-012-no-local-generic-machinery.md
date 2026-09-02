---
id: TC-012
title: "Verify that no repository-local generic assurance machinery remains"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-006
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-002
    type: verifies
---
# TC-012: Verify that no repository-local generic assurance machinery remains

## Description

Verify that no repository-local generic runner, evidence envelope, manifest,
identity framework, retention store, audit store, anchor file or aggregate
verdict remains in this repository's execution path.

## Test Procedure

Assert `scripts/` against a closed allow-list of the scripts this repository
declares, rather than a blocklist of names a suffix defeats. Confirm `make ci`
defines no repository-local evidence verifier target and retains no evidence
directory of its own.

## Expected Results

`scripts/` contains exactly the declared set and nothing else: an undeclared
script is how a generic collector, envelope builder or verifier returns, and the
allow-list fails on any addition rather than on a name somebody thought to
forbid. No `verify-evidence` target is defined, and the repository retains no
local evidence tree, manifest, or digest oracle of its own.

The retained legacy evidence this test once guarded was deleted under the
preservation constraint the owner released for the pre-stable phase
(`agent-ix/engineering-assurance#7`). Nothing here restates a claim that those
records verify anything.
