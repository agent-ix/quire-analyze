---
id: SR-5561
title: "Base spec review of IR-729 assurance schema repair"
type: SpecReview
analysis: base
scope: "agent-ix/quire-analyze@decb97b710c3f49646c5556e0c93edc1467ae8b7; spec/assurance/AP-001-analyze-release.md, spec/assurance/MP-001-analyze-measurements.md"
review_set: base
---

## Summary

The changed documents validate under installed Quire 0.36.2. The new measurement plan apparatus list conflicts with this repository's explicit prohibition on introducing file-tracking records.

## Verdict

**FAIL** — The new `protected_apparatus` list is a file-tracking record. The installed MeasurementPlan schema requires it for a gate-stage plan, so this is a real policy/schema conflict requiring resolution, not a reason to silently omit validation.

## Examined Units

- AP-001 — examined: schema version, scope, review policy, impact assessment, and human-release authority.
- MP-001 — examined: mechanical ground truth, statistical design, decision-use prose, measures, protected apparatus, and negative controls.
- Repository `AGENTS.md` and `CLAUDE.md` — context only: both prohibit new records that track files and direct removal when a touched spec contains one.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | New protected apparatus list tracks repository files despite the repository's explicit prohibition on new file-tracking records | spec/assurance/MP-001-analyze-measurements.md:21 |

The new list at lines 21–24 names `tests/**`, `spec/test/**`, and `Makefile`. The installed MeasurementPlan schema describes `protected_apparatus` as resolving those entries into a recorded set of paths and digests. That is a new file-tracking record in a repository whose own `AGENTS.md` and `CLAUDE.md` prohibit new hashes, digests, pins, or records that track files. The same schema requires the field when `stage: gate`, so the author cannot simply delete the field and retain this schema-valid gate plan. Resolve the conflict at the source of authority before accepting the PR.

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-002 | high | MP-001 remains structurally invalid as a gate-stage MeasurementPlan because protected_apparatus is required | spec/assurance/MP-001-analyze-measurements.md:9 |

At 436afb3b0ad990387b86af205de35a43b7da1e08, `quire validate --scope . spec/assurance/AP-001-analyze-release.md spec/assurance/MP-001-analyze-measurements.md` exits 3 and reports `"protected_apparatus" is a required property` for MP-001. The installed schema requires that field whenever status is not retired and `stage: gate`. Removing the prohibited file-tracking list resolves FND-001 but leaves the changed document outside the installed MeasurementPlan contract. Upstream EA #180 and Quoin #688 are reported as proposed resolution paths; neither is an implemented fix in this PR.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 436afb3b0ad990387b86af205de35a43b7da1e08 — removed the new `protected_apparatus` field and `apparatus-edit` control |
| FND-002 | still-open | At 436afb3b0ad990387b86af205de35a43b7da1e08, installed Quire 0.36.2 rejects MP-001 while `stage: gate` is retained and `protected_apparatus` is absent. |
