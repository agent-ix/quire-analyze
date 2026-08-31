---
id: REV-018
title: "Differential reports and CLI specification review"
type: Review
---
# Differential reports and CLI specification review

## Scope

Pre-implementation producer review for native issue #5, excluding the unavailable shared PGM-01
envelope decision. It covers runtime report canonicalization, digest re-derivation, differential
dispositions, solver pins, corpus states, library/CLI parity, atomic publication, and the shared-tool
plan delta. This is not independent approval or a release decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| EVI-S01 | critical | Calling two equal status strings “agreement” could hide missing engines, unverified sat models, or two equal failures. | Closed in specification: agreement requires two exact available engines, identical conclusive status, and every required model replay-verified; all other equal states remain unavailable or inconclusive. |
| EVI-S02 | critical | A report-authored digest or disposition would let mutated raw evidence validate itself. | Closed: validation re-derives raw-stream digests, canonical report digest, query/config identities, and differential disposition. Unknown fields and missing required fields reject. |
| EVI-S03 | high | Runtime maps and serializer order could make library/CLI bytes differ. | Closed: recursively key-sorted compact canonical JSON defines the bytes, and both surfaces call one library renderer. |
| EVI-S04 | high | An unconditional model request after unsat/unknown can produce a protocol error that masks the primary result. | Implementation constraint: analysis queries explicitly bind the model request; the parser accepts only the single expected post-status model-unavailable response for non-sat, while sat still requires a valid model. Raw stdout remains retained. |
| EVI-S05 | high | Replacing an existing output could edit developer-owned data or leave partial bytes. | Closed: v1 publication is create-new only, stages in the destination directory, syncs, renames atomically, and cleans residue on every injected failure. |
| EVI-S06 | high | Bundling “latest” solvers or PATH resolution would make the differential claim irreproducible. | Closed: official release asset names and archive SHA-256 values are fixed; execution additionally pins the extracted executable digest and complete version output. |
| EVI-S07 | critical | Task-007 would otherwise create the ninth drifting assurance collector/envelope/verifier implementation. | Closed for work that can proceed: Quoin 0.22.5 owns retained run transcription/audit and no local script is added. The missing PGM-01 envelope/integrity selection stays unavailable under `quire-contract-ir#20` and cannot be claimed complete. |

## Plan Delta

Implement the application report/differential/corpus/CLI surfaces and adopt Quoin for retained run
transcription. Keep FR-005-AC-2 and Task-007 open until the upstream shared component is selected and
adopted. This is a dependency/signoff guard, not a runtime report design choice.

## Pre-implementation Verdict

PASS to implement the unblocked issue #5 slices. FAIL CLOSED on PGM-01 envelope completion: absence
is represented as unavailable and prevents Task-007/epic completion.
