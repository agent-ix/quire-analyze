---
id: REV-023
title: "Atomic report publication fault-injection gap analysis"
type: Review
---
# Atomic report publication fault-injection gap analysis

## Acceptance Coverage

| Issue #23 area | Evidence | Status |
|---|---|---|
| Test-only filesystem seam on the production algorithm | `PublicationIo` drives the same state machine used by `RealPublicationIo` | closed |
| Create and partial-write failure | Exact injected stage, unchanged destination, closed handle, removed staging, cleanup sync | closed |
| File-sync and rename failure | Exact injected stage, unchanged destination, removed staging, cleanup sync | closed |
| Cleanup failure | Primary and cleanup errors are both retained; no false clean claim | closed, with residue explicitly reported when removal itself is impossible |
| Directory durability contract | Parent sync is required for success; post-rename failure reports `published-durability-unknown` with complete bytes | closed |
| Competing publishers | Eight simultaneous production publishers, exactly one complete winner, no recoverable staging residue | closed |
| Stale same-name staging | Exclusive create refuses it and does not delete unknown-owner bytes | closed |
| Process termination between boundaries | Child exits after create/write/file-sync/rename/directory-sync; destination and staging state are censused | covered, limitation open |

## Open Gap

With a named temporary file and atomic rename, an uncatchable process exit before rename necessarily
prevents the terminated process from unlinking its staging name. Deleting all matching names from a
new process would be unsafe without an ownership/locking protocol because another publisher may be
live. The test therefore retains the real one-file private residue instead of pretending cleanup
occurred. Issue #23 needs reviewer acceptance of this recovery contract or a separately reviewed
publication-protocol redesign.

## Verdict

No recoverable I/O, concurrency, overwrite, partial-destination, or durability gap remains. The
literal crash-residue clause remains open and visible; #23 is not represented as complete.
