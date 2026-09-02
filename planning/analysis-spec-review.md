---
id: REV-014
title: "Analysis semantics specification review"
type: Review
---
# Analysis semantics specification review

## Scope

Pre-implementation producer review for native issue #4. This review covers the five decision
predicates, group cardinalities, assertion polarity, status normalization, model completeness,
independent replay, source identity, and plan delta. It is not independent approval or a release
decision.

## Findings and Dispositions

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| ANA-S01 | critical | Generic left/right arrays could allow the same bytes to mean different analysis predicates. | Closed in specification: five dedicated constructors enforce kind-specific roles and cardinalities; every mapped assertion retains its role and polarity. |
| ANA-S02 | critical | Treating `sat` or `unsat` uniformly would invert four of the five conclusions. | Closed: FR-001 owns the exact five-row predicate table and TC-001 exercises all ten kind/outcome combinations independently. |
| ANA-S03 | critical | Solver-shaped model text could be published without proving it describes the query. | Closed: verification requires an exact complete Boolean symbol set and independent replay of every sealed assertion. Missing, duplicate, unknown, ill-typed, or predicate-refuting assignments produce incomplete explanation state, never a verified model. |
| ANA-S04 | high | A conclusive label could erase timeout, cancellation, unsupported platform, or tool failure. | Closed: the six public statuses and adapter-outcome mapping are exhaustive; only satisfied/refuted are conclusive and the sealed solver record remains attached. |
| ANA-S05 | high | Ambient assumptions or duplicate clauses across roles could make request identity ambiguous. | Closed: assumptions are always explicit, a clause may appear in only one role, canonical identity binds kind/role/polarity/bindings/profiles/limits, and irrelevant input order is normalized. |
| ANA-S06 | high | The requirement text promised data theories absent from the accepted lowering profile. | Closed for v0.1: issue #4 consumes exactly the Boolean profile from issue #7 and rejects every other construct before query generation. No arithmetic or data approximation is permitted. |
| ANA-S07 | medium | Real-engine self-agreement would not independently validate the truth table. | Closed in verification design: TC-001 exhaustively enumerates finite Boolean assignments independently of solver output. Real-engine differential execution remains Task-007. |

## Plan Delta

Task-006 advances to `in_progress` because Task-005 is locally complete. Implementation order is:
closed request/status types; role-aware deterministic lowering; bounded Boolean model decoder;
independent replay and source mapping; exhaustive truth-table and malformed-model tests; code review,
completion gap analysis, and source-bound validation capture. Task-007 remains guarded.

## Pre-implementation Verdict

PASS to implement the exact Boolean-v1 analysis slice. No unresolved design choice blocks production
work; broader data theories and real-engine differential evidence are explicitly outside issue #4.
