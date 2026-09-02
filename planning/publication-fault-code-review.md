---
id: REV-022
title: "Atomic report publication fault-injection code review"
type: Review
---
# Atomic report publication fault-injection code review

## Scope

Producer review of native QA issue #23: publication state classification, the production-path I/O
seam, file and directory durability, cleanup, competing publishers, stale staging ownership, and
process-termination boundaries. This is not independent approval or a release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| PUB-F01 | critical | Returning a plain I/O error after rename could make a caller retry even though complete destination bytes already exist. | Closed: `PublicationError` binds the exact stage and distinguishes `destination-unmodified` from `published-durability-unknown`. |
| PUB-F02 | high | Syncing only the staged file does not make the directory entry durable. | Closed: success now requires parent-directory `sync_all` after no-replace rename. A failure at that boundary reports the post-rename state. |
| PUB-F03 | high | Write, sync, and rename failures could leave owned staging or overwrite an existing destination. | Closed for recoverable failures: the common state machine closes, removes, and directory-syncs cleanup; `renameat2(RENAME_NOREPLACE)` remains the production publication primitive. |
| PUB-F04 | high | Tests could exercise a fake algorithm instead of the production stage order. | Closed: production and fault tests call the same generic state machine; only the filesystem operations are replaced. Partial write, file sync, rename, directory sync, and cleanup errors are injected. |
| PUB-F05 | high | Concurrent publishers or a stale name collision could turn cleanup into deletion of another attempt's data. | Closed: staging creation is exclusive, only a successfully created staging path is cleaned, stale creation failures preserve the unknown owner, and an eight-publisher race produces exactly one complete winner. |
| PUB-F06 | medium | An uncatchable process termination cannot run cleanup code. | Explicit limitation: subprocess termination after create/write/file-sync exposes no destination but leaves one private staging entry; termination after rename exposes only complete bytes. No staging entry is treated as authoritative. Review signoff is required before issue #23 can close because its literal zero-residue-on-every-failure wording is physically unattainable for an uncatchable exit with a named-temp/rename design. |

## Verdict

PASS for the implemented recoverable-fault, concurrency, and durability boundaries. HOLD issue #23
closure on PUB-F06 wording/signoff; the implementation and test record do not falsify post-crash
cleanup.
