---
id: REV-019
title: "Differential evidence and CLI code review"
type: Review
---
# Differential evidence and CLI code review

## Scope

Producer review of the unblocked native issue #5 slice: two-engine comparison, versioned canonical
reports, schema and mutation validation, solver identity/configuration retention, guarded PGM-01
state, no-replace Linux publication, CLI exit classes, and pinned real-engine execution. This is not
independent approval, PGM-01 evidence authority, or a release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| EVI-F01 | critical | Digest-only query/configuration fields could not reproduce the executed solver boundary. | Closed: reports retain query bytes plus byte digest and domain-separated identity; each engine retains configured path, expected executable/version pins, limits, argv, process exit, actual identity, raw streams, model, and their digests. |
| EVI-F02 | critical | A self-resealed report could evade a test-only schema check or author a contradictory status/disposition. | Closed: the production validator applies the embedded strict schema, exact key census, canonical/report/raw/query digests, engine order/query identity, outcome-to-status mapping, and re-derived disposition/agreed status. Contextual validation also reconstructs authoritative bytes from sealed library types. |
| EVI-F03 | critical | Z3 5.1.0 emits declared named-assertion aliases in SAT models, causing valid replay evidence to become incomplete. | Closed: the decoder ignores only assertion names sealed in the query, still requires the exact declared variable set, and independently replays every original assertion. Unknown symbols remain rejected. |
| EVI-F04 | high | Z3 exits 1 after a valid non-sat result when the following model request reports model unavailable. | Closed narrowly: only Z3 exit 1, empty stderr, an unsat/unknown primary status, and one whitelisted model-unavailable response preserve the primary result. Other nonzero exits remain failures. |
| EVI-F05 | high | The CLI initially mapped both satisfied and refuted agreement to exit 0. | Closed: authoritative agreement maps satisfied to 0 and refuted to 1; nonagreement maps to 3, invalid input to 2, and publication/internal failure to 4. |
| EVI-F06 | high | Default jsonschema features introduced vulnerable h2 0.3.27 through unused HTTP resolution. | Closed without suppression: HTTP/file/CLI resolver defaults are disabled; the vulnerable network stack is absent and cargo-deny advisories pass. |
| EVI-F07 | high | Rendering a legacy non-analysis query could panic. | Closed: rendering is fallible and rejects a query without an analysis kind. |
| EVI-F08 | high | Atomic publication boundary failures are not individually injectable. | Open as defense-in-depth QA issue #23; existing-destination refusal, unchanged bytes, and staging residue are tested now. |
| EVI-F09 | high | The direct analysis CLI and complete retained real-engine corpus are not implemented. | Open as QA issue #24. V1 is explicitly library-first and the CLI validates/publishes authoritative library bytes. |
| EVI-F10 | critical | The shared PGM-01 envelope/integrity component remains undecided upstream. | Open and fail-closed on `quire-contract-ir#20`; reports say `unavailable`, and FR-005-AC-2/Task-007 cannot complete. |

## Verdict

PASS for the implemented library-first runtime report, differential, real-engine, validator, and
publisher slice. No known finding permits false agreement or overwritten developer data. Task-007
remains in progress because the PGM-01 lane and the explicitly ticketed broader QA surfaces remain
open.
