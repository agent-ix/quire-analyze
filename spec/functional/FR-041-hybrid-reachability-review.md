---
id: FR-041
title: "Review the hybrid reachability module and record one disposition"
type: FR
relationships:
  - target: ix://agent-ix/quire-analyze/FR-040
    type: depends_on
  - target: ix://agent-ix/quire-analyze/StR-001
    type: implements
---
# FR-041: Review the hybrid reachability module and record one disposition

## Description

When a ticket proposes work that builds on the in-house hybrid reachability module, the maintainers SHALL review the module against [FR-040](./FR-040-hybrid-enclosure-soundness.md).

The review SHALL record exactly one disposition: keep and harden the module, or remove it and delegate hybrid reachability to an established external reachability tool run through `producer-execute`.

quire-analyze runs external solvers and supplies no prover of its own, so the review decides whether an in-house reachability algorithm belongs here at all.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-041-AC-1 | The review holds an adversarial test set with at least one test for each of FR-040-AC-1 through FR-040-AC-12, and each test names the FR-040 criterion it attacks. The set includes tests for rounding and representability at a guard boundary, overlapping guards, a reset that leaves an invariant, horizon exhaustion and a malformed mode. | Inspection |
| FR-041-AC-2 | Every test in the set that observes a violated FR-040 criterion is recorded as a finding with the test's name, the criterion and a minimal request that reproduces it. The finding is marked unsound when the result omits a reachable state or reports an unreached horizon as reached, and marked precision when the result contains only extra states. | Inspection |
| FR-041-AC-3 | The review lists the callers of `reach` and `replay_hybrid` and each requirement or planned consumer that depends on their output. | Inspection |
| FR-041-AC-4 | The recorded disposition is exactly one of keep-and-harden or delegate-and-remove. The reason is stated against four criteria: the number and severity of unsound findings, whether each unsound finding has a bounded fix, whether the model class in use is one an established external tool supports, and the cost of maintaining an algorithm that no external engine cross-checks. Any unsound finding without a bounded fix, or any unsound finding when an established external tool supports the model class, selects delegate-and-remove; otherwise the disposition is keep-and-harden. | Inspection |
| FR-041-AC-5 | The disposition names no fallback between the two paths. A review that records both as alternatives, or records a conditional switch from one to the other, is incomplete. | Inspection |
| FR-041-AC-6 | The disposition states whether the engine-ownership ruling for hybrid models, IR-1469, is recorded when the disposition is made. When it is recorded, the disposition agrees with it or states why it does not. When it is not recorded, the disposition proceeds without waiting for it and the application ticket re-checks agreement under FR-042-AC-6. | Inspection |
| FR-041-AC-7 | The findings and the disposition are posted on the tracking ticket for the review before the disposition is applied. | Inspection |

## Dependencies

- [FR-040](./FR-040-hybrid-enclosure-soundness.md) supplies the obligations the review judges against.
- [FR-007](./FR-007-sound-hybrid-reachability.md) is the requirement removed if the disposition is delegate-and-remove.
