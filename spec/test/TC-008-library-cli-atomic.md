---
id: TC-008
title: "Verify library and CLI parity and atomic output"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-005
    type: verifies
---
# TC-008: Verify library and CLI parity and atomic output

## Description

Verify both public surfaces agree and failure cannot publish partial output.

## Test Procedure

Render an authoritative request result through the library, then validate and publish those bytes
through the CLI. Compare bytes and exit classes.
Inject failures at each staged output boundary and compare destination and developer-tree digests.

V1 solver execution remains library-first because the CLI does not reconstruct trusted contract
packages or exact solver configurations from JSON. The CLI refuses an existing destination and
writes only a same-directory uniquely named staging file before sync and rename. Tests exercise
successful publication and existing-destination refusal and census staging residue; deterministic
fault injection for the write and sync boundaries remains separately tracked.

## Expected Results

The two surfaces agree. Failed publication leaves the prior destination intact, creates no partial
published report, and modifies no developer-owned file.
