---
id: TC-012
title: "Verify the read-only legacy compatibility view"
type: TC
relationships:
  - target: ix://agent-ix/quire-analyze/FR-006
    type: verifies
  - target: ix://agent-ix/quire-analyze/NFR-002
    type: verifies
---
# TC-012: Verify the read-only legacy compatibility view

## Description

Verify that retained evidence is read through the pinned Engineering Assurance
mapping without being modified, that the mapping's answer is reported as it
stands, and that no repository-local generic evidence machinery remains in the
execution path.

## Test Procedure

Read every file under `evidence/` through
`engineering_assurance.verification_semantics.map_pgm01_bytes` with each record's
committed `evidence/manifest.sha256` digest bound as the expected digest. Compare
the retained census against the manifest in both directions: a record present but
undeclared, and a record declared but absent, are each reported.

Re-derive every negative fixture from the pinned release's own
`pgm01-v1.json` and `pgm01-v2.json` bytes at run time, and require each committed
fixture to equal its declared derivation. Check each pinned upstream artifact
against the digest `assurance/pins.json` records.

Run the mutation probes with no exception handling: the pinned release fixture
must be read rather than refused; a wrong expected digest must read as a tampered
source; a single altered byte must change the mapped source identity; a committed
fixture must equal its re-derivation; an unknown schema version must be
`incompatible`; and this repository's own retained bytes must be `unreadable`.

Assert `scripts/` against a closed allow-list of the scripts this repository
declares, rather than a blocklist of names a suffix defeats, and confirm `make ci`
runs no repository-local evidence verifier.

## Expected Results

Every retained record is read without its bytes or source identity changing, and
the mapping's answer for each is reported exactly. This repository's records are
Markdown validation summaries, so the answer is `unreadable` for all of them;
that refusal is the reported compatibility result and is not converted into a
pass, a failure, or `incompatible`.

`incompatible`, `unreadable`, `stale` and a readable control remain four
distinguishable answers — `stale` discriminated by the mapped evidence state, since
a retracted record stays readable and shares its control's outcome — each demonstrated by a case that ran and matched, and each negative
paired with a positive control that was observed to be accepted. A control naming
a case that did not run is refused rather than passing vacuously.

Every mutation probe is detected. No generic evidence script exists under
`scripts/`, and `evidence/manifest.sha256` remains present and frozen as the
digest source the compatibility view binds.
