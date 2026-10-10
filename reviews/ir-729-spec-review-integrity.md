---
id: SR-5562
title: "Integrity review of IR-729 measurement decision rule"
type: SpecReview
analysis: integrity
scope: "agent-ix/quire-analyze@decb97b710c3f49646c5556e0c93edc1467ae8b7; spec/assurance/AP-001-analyze-release.md, spec/assurance/MP-001-analyze-measurements.md"
review_set: subset
---

## Summary

The new prose keeps absent lanes and inconclusive results visible and reserves release sufficiency for the human owner. The typed decision rule has no mechanical guard against a zero count from a missing or undersized population.

## Verdict

**CONDITIONAL** — An empty or partial collected population can satisfy the typed count threshold even though the prose calls such evidence incomplete.

## Examined Units

- AP-001 — examined: decision boundary, failure states, evidence policy, and human owner.
- MP-001 — examined: population and sampling definition, repetition count, typed estimator and decision rule, all three negative controls, protected apparatus, and mechanical decision prose.
- AA-001 — context only: top claim and human sufficiency decision remain open.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Count-zero gate lacks an enforced population floor or independent census, so missing observations can appear clean | spec/assurance/MP-001-analyze-measurements.md:18 |

The schema evaluates only the typed `decision_rule` over the collected metric. `minimum_population` is optional and absent here. Thus a collection can report zero adverse items after omitting fixtures or supported lanes and satisfy `eq: 0`; the new prose at lines 61–64 labels that evidence incomplete, but a checker does not evaluate that prose. The `suppressed-observation` control reconciles only declared fixtures, with no independently enforced population floor or authoritative complete fixture census. Keep the human boundary, and make an incomplete collection mechanically distinguishable from a measured zero before treating this as a gate.

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 436afb3b0ad990387b86af205de35a43b7da1e08 — `minimum_population: 12` matches the 12 declared TC files and the plan requires three repetitions. The prose explicitly requires the human gate to compare retained TC IDs against that declared set; the numeric floor alone cannot prove identities. |
