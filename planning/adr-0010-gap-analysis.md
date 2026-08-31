---
id: REV-006
title: "ADR-0010 completion gap analysis"
type: Review
---
# ADR-0010 completion gap analysis

## Acceptance Coverage

| Issue #6 acceptance criterion | Evidence | Status |
|---|---|---|
| Binding v0.1 decision with measured examples | Accepted ADR-0010; REV-004 dual encoding; TC-009 55-pair measurement | closed |
| Different scopes cannot merge by same name | Explicit-only groups; `(2, 12, 0)` name result; incompatible-member test | closed |
| Encoding changes invalidate stale evidence | Separate model/encoding profiles and domain-separated identity chain; TC-009 required-term guard | closed as architecture; implementation is issue #7 |
| Replacement linked and supersedes quire-rs#164 | Historical issue is closed with a human comment linking issue #6; ADR/report retain the link | closed |
| Code review, requirement test, gap analysis, retained evidence | REV-005, TC-009, this review, candidate evidence task | ready for evidence capture |

## Open Downstream Gaps

| Gap | Disposition |
|---|---|
| Production analysis-model validation, type-shape projection, canonical hash bytes, and SMT lowering do not exist. | Existing issue #7; blocks adapter work. |
| The accepted upstream corpus has one expression owner and cannot estimate real cross-requirement binding prevalence. | Issue #7 must add multi-requirement golden and collision fixtures; the current measurement is adversarial, not prevalence evidence. |
| Cross-execution-point temporal mapping is unsupported. | Explicit v0.1 capability limitation; any future frame profile requires a reviewed requirement and encoding-profile change. |
| Binding-label semantic correctness depends on upstream human review. | Reports must carry binding coverage and identity; automatic proposals have no authority. |
| Formalization sampling and equivalence-cluster size remain empirical workflow questions. | Outside this deterministic crate and issue; no default is invented. |

## Ticket Census

No new ticket is filed. The only in-scope production gaps map to existing issue #7, and later adapter,
analysis, and evidence work maps to #3, #4, and #5. Cross-point temporal mapping is explicitly outside
v0.1 rather than silently promised. A duplicate ticket would weaken the accepted DAG.

## Verdict

No unresolved blocker remains for issue #6 completion. Issue #7 must remain Backlog until the issue
#6 ADR candidate is merged and Done. Semantic TC-001 through TC-008 remain planned.
