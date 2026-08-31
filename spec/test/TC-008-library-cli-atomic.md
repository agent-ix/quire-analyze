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

Invoke identical requests through the library and CLI. Compare normalized bytes and exit classes.
Inject failures at each staged output boundary and compare destination and developer-tree digests.

V1 parity uses the built-in seeded conformance request plus identical exact Z3/cvc5 configurations.
The CLI refuses an existing destination and writes only a same-directory uniquely named staging file
before sync and rename. Tests inject create, write, sync, and rename failure and census residue.

## Expected Results

The two surfaces agree. Failed publication leaves the prior destination intact, creates no partial
published report, and modifies no developer-owned file.
