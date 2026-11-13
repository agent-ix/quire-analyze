---
id: FR-031
title: "Review the hybrid reachability module and record one disposition"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-030
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-031: Review the hybrid reachability module and record one disposition

## Description

When further work is about to build on the in-house hybrid reachability module, the maintainers SHALL
review it against [FR-030](./FR-030-hybrid-enclosure-soundness.md) and SHALL record exactly one disposition:
keep and harden it, or remove it and delegate hybrid reachability to an established external
reachability tool run through `producer-execute`. quire-analyze runs external solvers and does not
supply its own prover, so the review decides whether an in-house reachability algorithm belongs here
at all.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-031-AC-1 | The review holds an adversarial test set with at least one test for each of: rounding at a guard boundary, overlapping guards, a reset that leaves an invariant, horizon exhaustion, and a malformed mode. Each test states the FR-030 criterion it attacks. | Inspection |
| FR-031-AC-2 | Every test in the set that observes an unsound enclosure, meaning one that omits a reachable state or reports an unreached horizon as reached, is recorded as a finding with that test's name, the FR-030 criterion it violates and a minimal request that reproduces it. | Inspection |
| FR-031-AC-3 | The review lists the callers of `reach` and `replay_hybrid` and each requirement or planned consumer that depends on their output. | Inspection |
| FR-031-AC-4 | The recorded disposition is exactly one of keep-and-harden or delegate-and-remove, with the reason stated against these criteria: the number and severity of findings, whether each finding has a bounded fix, whether the model class in use is one an established external tool supports, and the cost of maintaining an algorithm that no external engine cross-checks. | Inspection |
| FR-031-AC-5 | The disposition names no fallback between the two paths. A review that records both as alternatives, or records a conditional switch from one to the other, is incomplete. | Inspection |
| FR-031-AC-6 | The disposition states whether it agrees with the engine-ownership ruling for hybrid models recorded in IR-1469, or states why it does not. | Inspection |
| FR-031-AC-7 | The findings and the disposition are posted on the tracking ticket for the review before the disposition is applied. | Inspection |

## Dependencies

- [FR-030](./FR-030-hybrid-enclosure-soundness.md) supplies the obligations the review judges against.
- [FR-007](./FR-007-sound-hybrid-reachability.md) is the requirement removed if the disposition is
  delegate-and-remove.
