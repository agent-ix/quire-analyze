---
id: REV-008
title: "Deterministic SMT lowering completion gap analysis"
type: Review
---
# Deterministic SMT lowering completion gap analysis

## Acceptance Coverage

| Issue #7 acceptance criterion | Evidence | Status |
|---|---|---|
| Supported expressions lower to canonical SMT-LIB2 | Exhaustive Boolean v1 lowering; exact-operator test; retained golden | closed |
| Required theories/features are declared before solver use | Capability contract and golden declare SMT-LIB 2.6, model production, and `QF_UF`; no solver is invoked in this slice | closed |
| Assertions map to requirement clauses | `AssertionMap` retains name, complete `ClauseRef`, recomputed canonical digest, and authoritative source span | closed |
| Unsupported or potentially unsound encodings reject | Arithmetic, ordering, quantification, text/non-Boolean references, malformed bindings, and invalid authoritative inputs return errors without a query | closed |
| Equivalent supported packages emit byte-identical SMT-LIB2 | Forward/reverse statement order and forward/reverse named-type declaration order tests | closed |
| Every assertion has stable requirement/clause identity | Domain-separated statement digest and injective readable assertion symbol; golden and source-map assertions | closed |
| Unsupported arithmetic, quantification, and data types return Unsupported | Requirement-tagged representative fixtures assert `UnsupportedConstruct` | closed |
| Golden encodings are independently reviewable | `tests/golden/boolean-v1.smt2` is byte-compared to production output | closed |
| Code review, tests, gap analysis, and retained review record | REV-007, TC-010, this review, and the issue #7 validation record | closed after source-bound validation capture |

## Open Downstream Gaps

| Gap | Disposition |
|---|---|
| No bounded external solver process, cleanup proof, or cancellation path exists yet. | Existing issue #3 / Task-005. Reviewer issue #16 remains open until that implementation measures process-tree cleanup. |
| The five analysis kinds, model replay, and conclusion classification do not exist yet. | Existing issue #4 / Task-006. |
| Differential execution, deterministic reports, CLI publication, and machine-produced derivation evidence do not exist yet. | Existing issue #5 / Task-007; shared evidence component remains guarded on `quire-contract-ir#20`. |
| Mutation, fuzz, digest-collision-family, every-unsupported-variant, and large boundary campaigns are not part of the issue slice. | New QA issue #19. These are defense-in-depth additions, not untested issue #7 acceptance paths. |
| `time` cannot both consume its patched release and retain Cargo/Rust 1.75 because the patched dependency uses edition 2024. | Upstream `quire-contract-ir#37`; exact unreachable-parser exception is documented and narrowly scoped. |
| Installed Quire 0.31.0 disagrees between matrix validation (`Coverage Status`) and coverage calculation (`Status`). | Existing issue #14; local parsed matrix census fails closed while Quire validation remains gated. |

## Ticket Census

The QA assessment filed issue #19. Dependency hygiene is filed upstream as
`agent-ix/quire-contract-ir#37`. All other gaps map to existing #3, #4, #5, #14, #16, or the shared
evidence issue; no duplicate tickets are needed.

## Verdict

No issue #7 acceptance gap remains after source-bound validation capture. The implementation is a
deliberately narrow Boolean lowering boundary and must not be represented as a solver, analyzer, or
machine-evidence pipeline.
