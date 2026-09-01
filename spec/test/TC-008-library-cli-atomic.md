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
writes only a same-directory uniquely named staging file before file sync, no-replace rename, and
parent-directory sync. A production-path seam injects create, partial-write, file-sync, rename,
directory-sync, and cleanup failures. Tests also race eight publishers, preserve unknown stale
staging files, and terminate a subprocess after create, write, file sync, rename, and directory
sync.

## Expected Results

The two surfaces agree. Every recoverable pre-rename failure leaves the prior destination intact,
removes and directory-syncs its staging entry, creates no partial published report, and modifies no
developer-owned file. A post-rename directory-sync failure reports complete published bytes with
unknown crash durability. Abrupt pre-rename process termination exposes no destination and can
leave one private staging entry because no code can execute after termination; abrupt post-rename
termination exposes only the complete report. No crash residue is accepted as authoritative output.
